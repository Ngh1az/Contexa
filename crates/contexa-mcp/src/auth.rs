//! `AuthMiddleware` — `docs/11_MCP_Runtime.md` §7.1.
//!
//! bcrypt verification is deliberately slow (~100ms) — fine for the rare
//! `validate()` call at server startup, but would blow docs/11 §11's
//! "<10ms tool call" target if repeated per call. `validated` caches
//! raw-token → `token_id` after the first successful verify so repeat calls
//! (the common case — one Cursor session reuses one configured token) skip
//! bcrypt entirely; cleared for a token on `revoke`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use contexa_core::{ContexaError, Result};
use contexa_db::McpRepository;

pub struct AuthMiddleware {
    repo: Arc<dyn McpRepository>,
    validated: Mutex<HashMap<String, String>>,
}

impl AuthMiddleware {
    #[must_use]
    pub fn new(repo: Arc<dyn McpRepository>) -> Self {
        Self {
            repo,
            validated: Mutex::new(HashMap::new()),
        }
    }

    /// Generates a new token, persists its bcrypt hash, and returns the raw
    /// token — shown once; only the hash is ever stored.
    ///
    /// # Errors
    /// Returns an error if hashing or persisting the token fails.
    pub async fn generate_token(&self, label: &str) -> Result<String> {
        let raw = format!("ctx_{}", hex_encode(&rand::random::<[u8; 32]>()));
        let hash = bcrypt::hash(&raw, bcrypt::DEFAULT_COST)
            .map_err(|e| ContexaError::Conversion(e.to_string()))?;
        self.repo.create_token(label, &hash).await?;
        Ok(raw)
    }

    /// # Errors
    /// Returns `ContexaError::Unauthorized` if `token` doesn't match any
    /// active (non-revoked) token.
    pub async fn validate(&self, token: &str) -> Result<String> {
        {
            let cache = self.validated.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(id) = cache.get(token) {
                return Ok(id.clone());
            }
        }

        let tokens = self.repo.find_active_tokens().await?;
        for info in tokens {
            if bcrypt::verify(token, &info.token_hash).unwrap_or(false) {
                self.repo.touch_token(&info.id).await?;
                let mut cache = self.validated.lock().unwrap_or_else(PoisonError::into_inner);
                cache.insert(token.to_string(), info.id.clone());
                return Ok(info.id);
            }
        }
        Err(ContexaError::Unauthorized)
    }

    /// # Errors
    /// Returns an error if the revoke fails to persist.
    pub async fn revoke(&self, token_id: &str) -> Result<()> {
        self.repo.revoke_token(token_id).await?;
        let mut cache = self.validated.lock().unwrap_or_else(PoisonError::into_inner);
        cache.retain(|_, id| id != token_id);
        Ok(())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}
