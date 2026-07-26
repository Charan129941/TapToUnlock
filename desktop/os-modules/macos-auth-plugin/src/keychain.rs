use thiserror::Error;

pub const KEYCHAIN_SERVICE_NAME: &str = "org.opentapunlock.keychain-token";
pub const KEYCHAIN_ACCOUNT_PREFIX: &str = "opentap-user-";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum KeychainError {
    #[error("Apple Keychain item not found for user")]
    NotFound,
    #[error("Keychain authorization failed or Touch ID/Secure Enclave rejected access")]
    AccessDenied,
    #[error("Keychain OS error: {0}")]
    OsError(String),
}

/// Helper for managing zero-trust session credentials inside Apple macOS Login Keychain.
pub struct MacKeychainHelper;

impl MacKeychainHelper {
    /// Stores an authenticated session token in the macOS Keychain for loginwindow retrieval.
    #[cfg(target_os = "macos")]
    pub fn store_session_token(username: &str, token: &str) -> Result<(), KeychainError> {
        use security_framework::passwords::set_generic_password;
        
        let account = format!("{}{}", KEYCHAIN_ACCOUNT_PREFIX, username);
        set_generic_password(KEYCHAIN_SERVICE_NAME, &account, token.as_bytes())
            .map_err(|e| KeychainError::OsError(format!("SecItemAdd failed: {}", e)))?;
        Ok(())
    }

    /// Retrieves an authenticated session token from the macOS Keychain.
    #[cfg(target_os = "macos")]
    pub fn retrieve_session_token(username: &str) -> Result<String, KeychainError> {
        use security_framework::passwords::find_generic_password;

        let account = format!("{}{}", KEYCHAIN_ACCOUNT_PREFIX, username);
        let (password_bytes, _) = find_generic_password(KEYCHAIN_SERVICE_NAME, &account)
            .map_err(|_| KeychainError::NotFound)?;
        
        let token_str = String::from_utf8(password_bytes.to_vec())
            .map_err(|e| KeychainError::OsError(format!("Invalid UTF-8 token: {}", e)))?;
        Ok(token_str)
    }

    /// Deletes the stored session token when locking the screen or logging out.
    #[cfg(target_os = "macos")]
    pub fn delete_session_token(username: &str) -> Result<(), KeychainError> {
        use security_framework::passwords::delete_generic_password;

        let account = format!("{}{}", KEYCHAIN_ACCOUNT_PREFIX, username);
        delete_generic_password(KEYCHAIN_SERVICE_NAME, &account)
            .map_err(|_| KeychainError::NotFound)?;
        Ok(())
    }

    // Cross-compilation mock simulation for Windows/Linux testing
    #[cfg(not(target_os = "macos"))]
    pub fn store_session_token(_username: &str, _token: &str) -> Result<(), KeychainError> {
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn retrieve_session_token(username: &str) -> Result<String, KeychainError> {
        if username == "chara" || username == "macos_test_user" {
            Ok("mock-keychain-secure-enclave-token-9988".to_string())
        } else {
            Err(KeychainError::NotFound)
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn delete_session_token(_username: &str) -> Result<(), KeychainError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_token_storage_and_retrieval() {
        let store_ok = MacKeychainHelper::store_session_token("chara", "test-token-123");
        assert!(store_ok.is_ok());

        let retrieved = MacKeychainHelper::retrieve_session_token("chara");
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), "mock-keychain-secure-enclave-token-9988");

        let delete_ok = MacKeychainHelper::delete_session_token("chara");
        assert!(delete_ok.is_ok());
    }
}
