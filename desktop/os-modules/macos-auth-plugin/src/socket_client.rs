use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::time::Duration;
use thiserror::Error;

pub const DEFAULT_MACOS_SOCKET_PATH: &str = "/var/run/opentapd.sock";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MacClientError {
    #[error("Failed to connect to OpenTap daemon UNIX socket at {0}")]
    ConnectionFailed(String),
    #[error("I/O error during macOS socket communication: {0}")]
    IoError(String),
    #[error("JSON serialization or parsing error: {0}")]
    SerializationError(String),
    #[error("Authentication query timed out waiting for mobile Triple Tap")]
    Timeout,
    #[error("Authentication explicitly denied by user or security rule")]
    Denied,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MacAuthRequest {
    pub cmd: String,
    pub user: String,
    pub timeout_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum MacAuthStatus {
    APPROVED,
    DENIED,
    TIMEOUT,
    ERROR,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MacAuthResponse {
    pub status: MacAuthStatus,
    pub token: Option<String>,
    pub device_name: Option<String>,
    pub reason: Option<String>,
}

pub struct DaemonUnixClient;

impl DaemonUnixClient {
    /// Connects to the local UNIX domain socket and requests biometric mobile unlock.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn request_unlock(
        socket_path: &str,
        username: &str,
        timeout_ms: u64,
    ) -> Result<MacAuthResponse, MacClientError> {
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(socket_path)
            .map_err(|e| MacClientError::ConnectionFailed(format!("{}: {}", socket_path, e)))?;

        stream
            .set_read_timeout(Some(Duration::from_millis(timeout_ms + 1000)))
            .map_err(|e| MacClientError::IoError(e.to_string()))?;
        stream
            .set_write_timeout(Some(Duration::from_millis(2000)))
            .map_err(|e| MacClientError::IoError(e.to_string()))?;

        let req = MacAuthRequest {
            cmd: "CHECK_AUTH".to_string(),
            user: username.to_string(),
            timeout_ms,
        };

        let mut json_str = serde_json::to_string(&req)
            .map_err(|e| MacClientError::SerializationError(e.to_string()))?;
        json_str.push('\n');

        stream
            .write_all(json_str.as_bytes())
            .map_err(|e| MacClientError::IoError(e.to_string()))?;
        stream.flush().map_err(|e| MacClientError::IoError(e.to_string()))?;

        let mut buffer = [0u8; 4096];
        let bytes_read = stream
            .read(&mut buffer)
            .map_err(|e| MacClientError::IoError(e.to_string()))?;

        if bytes_read == 0 {
            return Err(MacClientError::IoError("Daemon closed connection unexpectedly".to_string()));
        }

        let resp_slice = &buffer[..bytes_read];
        let resp_str = std::str::from_utf8(resp_slice)
            .map_err(|e| MacClientError::SerializationError(e.to_string()))?;

        let resp: MacAuthResponse = serde_json::from_str(resp_str.trim())
            .map_err(|e| MacClientError::SerializationError(e.to_string()))?;

        match resp.status {
            MacAuthStatus::APPROVED => Ok(resp),
            MacAuthStatus::DENIED => Err(MacClientError::Denied),
            MacAuthStatus::TIMEOUT => Err(MacClientError::Timeout),
            MacAuthStatus::ERROR => Err(MacClientError::IoError(
                resp.reason.unwrap_or_else(|| "Unknown daemon error".to_string()),
            )),
        }
    }

    /// Cross-platform mock simulation when cross-compiling on Windows.
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub fn request_unlock(
        _socket_path: &str,
        username: &str,
        _timeout_ms: u64,
    ) -> Result<MacAuthResponse, MacClientError> {
        if username == "chara" || username == "macos_test_user" {
            Ok(MacAuthResponse {
                status: MacAuthStatus::APPROVED,
                token: Some("macos-session-token-112233".to_string()),
                device_name: Some("iPhone 16 Pro (Simulated)".to_string()),
                reason: None,
            })
        } else if username == "blocked_user" {
            Err(MacClientError::Denied)
        } else {
            Err(MacClientError::Timeout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_auth_request_serialization() {
        let req = MacAuthRequest {
            cmd: "CHECK_AUTH".to_string(),
            user: "chara".to_string(),
            timeout_ms: 15000,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("CHECK_AUTH"));
        assert!(json.contains("chara"));
    }

    #[test]
    fn test_daemon_unix_client_simulation() {
        let ok = DaemonUnixClient::request_unlock(DEFAULT_MACOS_SOCKET_PATH, "chara", 5000);
        assert!(ok.is_ok());
        let resp = ok.unwrap();
        assert_eq!(resp.status, MacAuthStatus::APPROVED);
        assert_eq!(resp.device_name.unwrap(), "iPhone 16 Pro (Simulated)");

        let denied = DaemonUnixClient::request_unlock(DEFAULT_MACOS_SOCKET_PATH, "blocked_user", 5000);
        assert_eq!(denied, Err(MacClientError::Denied));
    }
}
