use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::time::Duration;
use thiserror::Error;

pub const DEFAULT_SOCKET_PATH: &str = "/run/opentapd/opentapd.sock";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ClientError {
    #[error("Failed to connect to OpenTap daemon socket at {0}")]
    ConnectionFailed(String),
    #[error("I/O error during IPC communication: {0}")]
    IoError(String),
    #[error("JSON serialization or parsing error: {0}")]
    SerializationError(String),
    #[error("Authentication query timed out waiting for mobile Triple Tap")]
    Timeout,
    #[error("Authentication explicitly denied by user or security rule")]
    Denied,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AuthRequest {
    pub cmd: String,
    pub user: String,
    pub service: String,
    pub timeout_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    APPROVED,
    DENIED,
    TIMEOUT,
    ERROR,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AuthResponse {
    pub status: AuthStatus,
    pub pc_uuid: Option<String>,
    pub device_name: Option<String>,
    pub reason: Option<String>,
}

pub struct DaemonSocketClient;

impl DaemonSocketClient {
    /// Connects to the local UNIX domain socket (or mock) and requests biometric verification.
    #[cfg(target_os = "linux")]
    pub fn verify_user(
        socket_path: &str,
        username: &str,
        service: &str,
        timeout_ms: u64,
    ) -> Result<AuthResponse, ClientError> {
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(socket_path)
            .map_err(|e| ClientError::ConnectionFailed(format!("{}: {}", socket_path, e)))?;

        stream
            .set_read_timeout(Some(Duration::from_millis(timeout_ms + 1000)))
            .map_err(|e| ClientError::IoError(e.to_string()))?;
        stream
            .set_write_timeout(Some(Duration::from_millis(2000)))
            .map_err(|e| ClientError::IoError(e.to_string()))?;

        let req = AuthRequest {
            cmd: "CHECK_AUTH".to_string(),
            user: username.to_string(),
            service: service.to_string(),
            timeout_ms,
        };

        let mut json_str = serde_json::to_string(&req)
            .map_err(|e| ClientError::SerializationError(e.to_string()))?;
        json_str.push('\n'); // Line delimiter

        stream
            .write_all(json_str.as_bytes())
            .map_err(|e| ClientError::IoError(e.to_string()))?;
        stream.flush().map_err(|e| ClientError::IoError(e.to_string()))?;

        let mut buffer = [0u8; 4096];
        let bytes_read = stream
            .read(&mut buffer)
            .map_err(|e| ClientError::IoError(e.to_string()))?;

        if bytes_read == 0 {
            return Err(ClientError::IoError("Daemon closed connection unexpectedly".to_string()));
        }

        let resp_slice = &buffer[..bytes_read];
        let resp_str = std::str::from_utf8(resp_slice)
            .map_err(|e| ClientError::SerializationError(e.to_string()))?;

        let resp: AuthResponse = serde_json::from_str(resp_str.trim())
            .map_err(|e| ClientError::SerializationError(e.to_string()))?;

        match resp.status {
            AuthStatus::APPROVED => Ok(resp),
            AuthStatus::DENIED => Err(ClientError::Denied),
            AuthStatus::TIMEOUT => Err(ClientError::Timeout),
            AuthStatus::ERROR => Err(ClientError::IoError(
                resp.reason.unwrap_or_else(|| "Unknown daemon error".to_string()),
            )),
        }
    }

    /// Cross-platform mock implementation for testing and development on Windows/macOS.
    #[cfg(not(target_os = "linux"))]
    pub fn verify_user(
        _socket_path: &str,
        username: &str,
        _service: &str,
        _timeout_ms: u64,
    ) -> Result<AuthResponse, ClientError> {
        // In simulation/dev mode, if username is "opentap_test", simulate approval!
        if username == "opentap_test" || username == "chara" {
            Ok(AuthResponse {
                status: AuthStatus::APPROVED,
                pc_uuid: Some("mock-linux-desktop-uuid".to_string()),
                device_name: Some("Pixel 8 Pro (Simulated)".to_string()),
                reason: None,
            })
        } else if username == "blocked_user" {
            Err(ClientError::Denied)
        } else {
            Err(ClientError::Timeout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_request_serialization() {
        let req = AuthRequest {
            cmd: "CHECK_AUTH".to_string(),
            user: "root".to_string(),
            service: "sudo".to_string(),
            timeout_ms: 5000,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("CHECK_AUTH"));
        assert!(json.contains("sudo"));
    }

    #[test]
    fn test_auth_response_deserialization() {
        let raw = r#"{"status":"APPROVED","pc_uuid":"pc-linux-box","device_name":"iPhone 16"}"#;
        let resp: AuthResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.status, AuthStatus::APPROVED);
        assert_eq!(resp.device_name.unwrap(), "iPhone 16");
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_mock_client_verification() {
        let ok = DaemonSocketClient::verify_user("/tmp/sock", "opentap_test", "sudo", 5000);
        assert!(ok.is_ok());

        let denied = DaemonSocketClient::verify_user("/tmp/sock", "blocked_user", "sudo", 5000);
        assert_eq!(denied, Err(ClientError::Denied));
    }
}
