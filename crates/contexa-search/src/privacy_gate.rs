//! `PrivacyGate` — `docs/09_Search_Engine.md` §5.1.
//!
//! In-memory only (defaults `false`, opt-in) — persisting this through
//! `user_settings` is Phase 3 "Settings window" work with a UI to go with
//! it; no repository wraps that table yet.

use std::sync::{PoisonError, RwLock};

use contexa_core::{ContexaError, Result};

pub struct PrivacyGate {
    enabled: RwLock<bool>,
}

impl PrivacyGate {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: RwLock::new(enabled),
        }
    }

    #[must_use]
    pub fn is_allowed(&self) -> bool {
        *self.enabled.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().unwrap_or_else(PoisonError::into_inner) = enabled;
    }

    /// # Errors
    /// Returns `ContexaError::SearchDisabled` if search is not enabled.
    pub fn check(&self) -> Result<()> {
        if !self.is_allowed() {
            return Err(ContexaError::SearchDisabled);
        }
        Ok(())
    }
}

impl Default for PrivacyGate {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        let gate = PrivacyGate::default();
        assert!(!gate.is_allowed());
        assert!(gate.check().is_err());
    }

    #[test]
    fn enabling_allows_search() {
        let gate = PrivacyGate::new(false);
        gate.set_enabled(true);
        assert!(gate.is_allowed());
        assert!(gate.check().is_ok());
    }
}
