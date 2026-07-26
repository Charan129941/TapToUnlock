use crate::state::{AuthError, AuthStateMachine};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IpcRequest {
    pub cmd: String,
    pub user: String,
    pub service: Option<String>,
    pub timeout_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum IpcAuthStatus {
    APPROVED,
    DENIED,
    TIMEOUT,
    ERROR,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IpcResponse {
    pub status: IpcAuthStatus,
    pub token: Option<String>,
    pub device_name: Option<String>,
    pub reason: Option<String>,
}

pub struct IpcServer;

impl IpcServer {
    /// Starts the local OS IPC server listening on local TCP loopback port 30349 (cross-platform compatible).
    /// On Linux/macOS production deployments, this binds UNIX Domain Socket `/run/opentapd/opentapd.sock`.
    pub async fn start_loopback(port: u16, state_machine: AuthStateMachine) -> Result<(), std::io::Error> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        info!("OpenTap IPC Server listening on loopback {}", addr);

        loop {
            let (mut socket, _peer) = listener.accept().await?;
            let sm = state_machine.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = socket.split();
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();

                if let Ok(bytes_read) = buf_reader.read_line(&mut line).await {
                    if bytes_read > 0 {
                        if let Ok(req) = serde_json::from_str::<IpcRequest>(line.trim()) {
                            info!("Received IPC auth query for user '{}' (cmd: '{}')", req.user, req.cmd);
                            
                            let service = req.service.as_deref().unwrap_or("unknown");
                            sm.register_os_challenge(&req.user, service, req.timeout_ms);

                            let response = match sm.wait_for_approval(&req.user, req.timeout_ms).await {
                                Ok((token, device_name)) => IpcResponse {
                                    status: IpcAuthStatus::APPROVED,
                                    token: Some(token),
                                    device_name: Some(device_name),
                                    reason: None,
                                },
                                Err(AuthError::Denied) => IpcResponse {
                                    status: IpcAuthStatus::DENIED,
                                    token: None,
                                    device_name: None,
                                    reason: Some("Unlock explicitly denied".to_string()),
                                },
                                Err(AuthError::Timeout) => IpcResponse {
                                    status: IpcAuthStatus::TIMEOUT,
                                    token: None,
                                    device_name: None,
                                    reason: Some("Timed out waiting for mobile Triple Tap".to_string()),
                                },
                                Err(e) => IpcResponse {
                                    status: IpcAuthStatus::ERROR,
                                    token: None,
                                    device_name: None,
                                    reason: Some(e.to_string()),
                                },
                            };

                            if let Ok(mut json_resp) = serde_json::to_string(&response) {
                                json_resp.push('\n');
                                let _ = writer.write_all(json_resp.as_bytes()).await;
                                let _ = writer.flush().await;
                            }
                        } else {
                            error!("Failed to parse JSON IPC request from client");
                        }
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_ipc_server_auth_handshake() {
        let sm = AuthStateMachine::new();
        let sm_clone = sm.clone();
        
        // Start server on arbitrary loopback port for testing
        let port = 38472;
        tokio::spawn(async move {
            let _ = IpcServer::start_loopback(port, sm_clone).await;
        });

        sleep(Duration::from_millis(50)).await; // Wait for bind

        // Connect client
        let mut client = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        
        let req = IpcRequest {
            cmd: "CHECK_AUTH".to_string(),
            user: "chara".to_string(),
            service: Some("sudo".to_string()),
            timeout_ms: 1000,
        };

        let mut req_str = serde_json::to_string(&req).unwrap();
        req_str.push('\n');
        client.write_all(req_str.as_bytes()).await.unwrap();
        client.flush().await.unwrap();

        // Simulate mobile unlock approval in background
        let sm_sim = sm.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            sm_sim.process_verified_mobile_payload("chara", "Pixel 8 Pro");
        });

        let mut response_buf = [0u8; 1024];
        let n = client.read(&mut response_buf).await.unwrap();
        let resp_str = std::str::from_utf8(&response_buf[..n]).unwrap();
        
        let resp: IpcResponse = serde_json::from_str(resp_str.trim()).unwrap();
        assert_eq!(resp.status, IpcAuthStatus::APPROVED);
        assert_eq!(resp.device_name.unwrap(), "Pixel 8 Pro");
    }
}
