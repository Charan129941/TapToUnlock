use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AuthError {
    #[error("Authentication challenge timed out")]
    Timeout,
    #[error("Authentication request explicitly denied")]
    Denied,
    #[error("No active session or challenge found")]
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeStatus {
    Pending,
    Approved { session_token: String, device_name: String },
    Denied,
    TimedOut,
}

#[derive(Debug, Clone)]
pub struct PendingAuthRequest {
    pub request_id: String,
    pub username: String,
    pub service: String,
    pub created_at_utc: i64,
    pub timeout_ms: u64,
    pub status: ChallengeStatus,
}

/// Central state machine synchronizing OS login challenges with wireless mobile approvals.
#[derive(Clone, Default)]
pub struct AuthStateMachine {
    requests: Arc<Mutex<HashMap<String, PendingAuthRequest>>>,
}

impl AuthStateMachine {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Called by the local IPC server when a PAM, COM, or macOS Authorization Plugin requests unlock.
    pub fn register_os_challenge(&self, username: &str, service: &str, timeout_ms: u64) -> String {
        let req_id = Uuid::new_v4().to_string();
        let req = PendingAuthRequest {
            request_id: req_id.clone(),
            username: username.to_string(),
            service: service.to_string(),
            created_at_utc: Utc::now().timestamp_millis(),
            timeout_ms,
            status: ChallengeStatus::Pending,
        };

        let mut map = self.requests.lock().unwrap();
        map.insert(username.to_string(), req);
        req_id
    }

    /// Asynchronously waits until the pending challenge transitions to Approved, Denied, or TimedOut.
    pub async fn wait_for_approval(&self, username: &str, timeout_ms: u64) -> Result<(String, String), AuthError> {
        let start_time = Utc::now().timestamp_millis();
        let timeout_limit = start_time + timeout_ms as i64;

        loop {
            {
                let mut map = self.requests.lock().unwrap();
                if let Some(req) = map.get_mut(username) {
                    match &req.status {
                        ChallengeStatus::Approved { session_token, device_name } => {
                            let token = session_token.clone();
                            let dev = device_name.clone();
                            map.remove(username); // Consume challenge
                            return Ok((token, dev));
                        }
                        ChallengeStatus::Denied => {
                            map.remove(username);
                            return Err(AuthError::Denied);
                        }
                        ChallengeStatus::TimedOut => {
                            map.remove(username);
                            return Err(AuthError::Timeout);
                        }
                        ChallengeStatus::Pending => {
                            if Utc::now().timestamp_millis() > timeout_limit {
                                req.status = ChallengeStatus::TimedOut;
                                map.remove(username);
                                return Err(AuthError::Timeout);
                            }
                        }
                    }
                } else {
                    return Err(AuthError::NotFound);
                }
            }

            // Poll every 50ms without blocking runtime threads
            sleep(Duration::from_millis(50)).await;
        }
    }

    /// Called by the BLE/TLS network listener when a valid Ed25519-signed packet arrives from a paired phone.
    pub fn process_verified_mobile_payload(&self, username: &str, device_name: &str) -> bool {
        let mut map = self.requests.lock().unwrap();
        if let Some(req) = map.get_mut(username) {
            let token = format!("opentap-session-{}-{}", username, Uuid::new_v4());
            req.status = ChallengeStatus::Approved {
                session_token: token,
                device_name: device_name.to_string(),
            };
            true
        } else {
            false
        }
    }

    /// Denies a pending challenge (e.g., if user taps "Deny" on mobile screen).
    pub fn deny_challenge(&self, username: &str) {
        let mut map = self.requests.lock().unwrap();
        if let Some(req) = map.get_mut(username) {
            req.status = ChallengeStatus::Denied;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_auth_state_machine_approval_flow() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let sm = AuthStateMachine::new();
            let _id = sm.register_os_challenge("chara", "sudo", 5000);

            // Simulate wireless network receiving Triple Tap approval in another thread
            let sm_clone = sm.clone();
            tokio::spawn(async move {
                sleep(Duration::from_millis(100)).await;
                sm_clone.process_verified_mobile_payload("chara", "Pixel 8 Pro");
            });

            let res = sm.wait_for_approval("chara", 5000).await;
            assert!(res.is_ok());
            let (token, dev) = res.unwrap();
            assert!(token.starts_with("opentap-session-chara-"));
            assert_eq!(dev, "Pixel 8 Pro");
        });
    }

    #[test]
    fn test_auth_state_machine_timeout_flow() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let sm = AuthStateMachine::new();
            sm.register_os_challenge("timeout_user", "sudo", 150);

            let res = sm.wait_for_approval("timeout_user", 200).await;
            assert_eq!(res, Err(AuthError::Timeout));
        });
    }
}
