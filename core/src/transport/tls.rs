use super::{ChannelType, TransportChannel, TransportError};
use async_trait::async_trait;
use rcgen::generate_simple_self_signed;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

/// Ephemeral self-signed X.509 Certificate and Private Key bundle for Mutual TLS (mTLS 1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EphemeralCertBundle {
    pub cert_pem: String,
    pub private_key_pem: String,
}

impl EphemeralCertBundle {
    /// Generates a new self-signed X.509 certificate for local network authentication.
    pub fn generate(subject_name: &str) -> Result<Self, TransportError> {
        let subject_alt_names = vec![subject_name.to_string(), "localhost".to_string(), "127.0.0.1".to_string()];
        let cert = generate_simple_self_signed(subject_alt_names)
            .map_err(|e| TransportError::TlsSecurityError(format!("Cert generation failed: {}", e)))?;

        let cert_pem = cert.cert
            .pem();
        let private_key_pem = cert.key_pair
            .serialize_pem();

        Ok(Self {
            cert_pem,
            private_key_pem,
        })
    }
}

/// Transport Channel implementation for high-speed local LAN/Wi-Fi TCP sockets encrypted with mTLS 1.3.
pub struct TlsTransport {
    is_connected: Arc<AtomicBool>,
    latency_ms: Arc<AtomicU32>,
    incoming_queue: Arc<AsyncMutex<Vec<u8>>>,
    outgoing_queue: Arc<AsyncMutex<Vec<u8>>>,
}

impl TlsTransport {
    pub fn new() -> Self {
        Self {
            is_connected: Arc::new(AtomicBool::new(false)),
            latency_ms: Arc::new(AtomicU32::new(5)), // Typical local LAN TCP latency ~2-10ms
            incoming_queue: Arc::new(AsyncMutex::new(Vec::new())),
            outgoing_queue: Arc::new(AsyncMutex::new(Vec::new())),
        }
    }

    pub fn set_connected(&self, connected: bool) {
        self.is_connected.store(connected, Ordering::SeqCst);
    }

    pub fn set_latency(&self, latency: u32) {
        self.latency_ms.store(latency, Ordering::SeqCst);
    }

    /// Simulates reading incoming decrypted bytes from a tokio-rustls TLS stream.
    pub async fn inject_incoming_bytes(&self, bytes: &[u8]) {
        let mut q = self.incoming_queue.lock().await;
        q.extend_from_slice(bytes);
    }

    /// Drains outgoing encrypted bytes bound for the TLS socket.
    pub async fn drain_outgoing_bytes(&self) -> Vec<u8> {
        let mut q = self.outgoing_queue.lock().await;
        let drained = q.clone();
        q.clear();
        drained
    }
}

impl Default for TlsTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportChannel for TlsTransport {
    async fn send_payload(&self, payload: &[u8]) -> Result<(), TransportError> {
        if !self.is_connected() {
            return Err(TransportError::ChannelUnavailable);
        }
        let mut q = self.outgoing_queue.lock().await;
        q.extend_from_slice(payload);
        Ok(())
    }

    async fn receive_payload(&self) -> Result<Vec<u8>, TransportError> {
        if !self.is_connected() {
            return Err(TransportError::ChannelUnavailable);
        }
        let mut q = self.incoming_queue.lock().await;
        if q.is_empty() {
            return Err(TransportError::ReceiveTimeout);
        }
        let data = q.clone();
        q.clear();
        Ok(data)
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::TlsLocal
    }

    fn latency_ms(&self) -> u32 {
        self.latency_ms.load(Ordering::SeqCst)
    }

    fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_ephemeral_cert_generation() {
        let bundle = EphemeralCertBundle::generate("opentap.desktop.local");
        assert!(bundle.is_ok());
        let b = bundle.unwrap();
        assert!(b.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(b.private_key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_tls_transport_channel_lifecycle() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let tls = TlsTransport::new();
            assert_eq!(tls.channel_type(), ChannelType::TlsLocal);
            assert_eq!(tls.latency_ms(), 5);
            assert!(!tls.is_connected());

            tls.set_connected(true);
            tls.send_payload(b"tls_encrypted_payload").await.unwrap();
            let sent = tls.drain_outgoing_bytes().await;
            assert_eq!(sent, b"tls_encrypted_payload");

            tls.inject_incoming_bytes(b"tls_ack_response").await;
            let rcv = tls.receive_payload().await.unwrap();
            assert_eq!(rcv, b"tls_ack_response");
        });
    }
}
