use async_trait::async_trait;

/// Cross-platform secret store used to back SSH profile credentials
/// (passwords + key passphrases).
///
/// Backed by the [`keyring`] crate, which dispatches to:
/// - macOS Keychain (`apple-native`)
/// - Windows Credential Manager (`windows-native`)
/// - Linux Secret Service over D-Bus (`sync-secret-service`)
#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn store(&self, key: &str, value: &str) -> Result<(), String>;
    async fn get(&self, key: &str) -> Result<Option<String>, String>;
    async fn delete(&self, key: &str) -> Result<(), String>;
}

const SERVICE_NAME: &str = "Clauge SSH";

pub struct KeyringStore;

impl KeyringStore {
    fn entry(key: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE_NAME, key).map_err(|e| format!("keyring entry: {}", e))
    }
}

#[async_trait]
impl CredentialStore for KeyringStore {
    async fn store(&self, key: &str, value: &str) -> Result<(), String> {
        let key = key.to_string();
        let value = value.to_string();
        tokio::task::spawn_blocking(move || {
            Self::entry(&key)?
                .set_password(&value)
                .map_err(|e| format!("keyring set: {}", e))
        })
        .await
        .map_err(|e| format!("join error: {}", e))?
    }

    async fn get(&self, key: &str) -> Result<Option<String>, String> {
        let key = key.to_string();
        tokio::task::spawn_blocking(move || match Self::entry(&key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("keyring get: {}", e)),
        })
        .await
        .map_err(|e| format!("join error: {}", e))?
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let key = key.to_string();
        tokio::task::spawn_blocking(move || match Self::entry(&key)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // idempotent
            Err(e) => Err(format!("keyring delete: {}", e)),
        })
        .await
        .map_err(|e| format!("join error: {}", e))?
    }
}

/// Returns the platform-appropriate credential store implementation.
pub fn credential_store() -> impl CredentialStore {
    KeyringStore
}

#[cfg(test)]
mod tests {
    use super::*;

    // Linux CI runners are usually headless and lack a D-Bus session, so the
    // Secret Service backend cannot bind. Skip there.
    #[cfg(not(target_os = "linux"))]
    #[tokio::test]
    async fn keyring_round_trip() {
        let store = KeyringStore;
        let key = "clauge-test-key-do-not-collide";
        let value = "secret-value-xyz";

        let _ = store.delete(key).await; // pre-clean

        store.store(key, value).await.expect("store failed");
        let got = store.get(key).await.expect("get failed");
        assert_eq!(got.as_deref(), Some(value));

        store.delete(key).await.expect("delete failed");
        let after = store.get(key).await.expect("get after delete failed");
        assert_eq!(after, None);
    }
}
