//! API key storage — `docs/16_Security_Privacy.md` §8.2 sketches this as
//! `CredentialVault` but names two different backing crates (`keyring` in
//! docs/02's dependency table vs. `windows-credentials` in docs/16's own
//! code comment) and was never implemented. `keyring = "3"` is already a
//! workspace dependency with zero consumers — resolving the inconsistency
//! in favor of the crate actually in the dependency tree.
//!
//! `keyring` has no default feature set — without an explicit platform
//! backend it silently no-ops (writes succeed, reads return nothing, no
//! error). `Cargo.toml` enables `windows-native` (Contexa is Windows-only
//! per ADR-0001); found this the hard way when the round-trip test below
//! failed with a passing `store()` and an empty `retrieve()`.

use contexa_core::{ContexaError, Result};

const SERVICE_NAME: &str = "contexa";

#[derive(Default)]
pub struct CredentialVault;

impl CredentialVault {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// # Errors
    /// Returns an error if the OS credential store can't be reached.
    pub fn store(&self, key: &str, value: &str) -> Result<()> {
        entry(key)?
            .set_password(value)
            .map_err(|e| ContexaError::Conversion(e.to_string()))
    }

    /// # Errors
    /// Returns an error if `key` has no stored credential, or the OS
    /// credential store can't be reached.
    pub fn retrieve(&self, key: &str) -> Result<String> {
        entry(key)?
            .get_password()
            .map_err(|e| ContexaError::Conversion(e.to_string()))
    }

    /// # Errors
    /// Returns an error if `key` has no stored credential, or the OS
    /// credential store can't be reached.
    pub fn delete(&self, key: &str) -> Result<()> {
        entry(key)?
            .delete_credential()
            .map_err(|e| ContexaError::Conversion(e.to_string()))
    }
}

fn entry(key: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE_NAME, key).map_err(|e| ContexaError::Conversion(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercises the real OS credential store (Windows Credential Manager) —
    // safe to unit-test, unlike UIA/clipboard: keyring access isn't
    // apartment-threaded COM and has no message-pump requirement.
    #[test]
    fn store_retrieve_delete_round_trip() {
        let vault = CredentialVault::new();
        let key = "test-contexa-llm-credential-vault-round-trip";

        assert!(vault.store(key, "secret-value").is_ok());
        assert_eq!(vault.retrieve(key).ok().as_deref(), Some("secret-value"));

        assert!(vault.delete(key).is_ok());
        assert!(vault.retrieve(key).is_err());
    }
}
