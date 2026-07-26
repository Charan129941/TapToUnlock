use crate::crypto::CryptoError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PairingError {
    #[error("QR payload expired: the QR code is older than the allowed time window")]
    QrExpired,
    #[error("Invalid QR URI scheme or malformed payload")]
    InvalidQrUri,
    #[error("PIN mismatch: user verification failed")]
    PinMismatch,
    #[error("Invalid pairing state transition: {0}")]
    InvalidStateTransition(String),
    #[error("Crypto or base64 error: {0}")]
    CryptoError(#[from] CryptoError),
}

/// Out-of-band (OOB) payload encoded inside the QR code displayed on the desktop screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QrPairingPayload {
    /// Unique Workstation Identifier
    pub pc_uuid: String,
    /// Base64url encoded Ed25519 public key
    pub pc_pub_key_base64: String,
    /// Decoded raw bytes alias for mobile client bindings
    #[serde(default)]
    pub desktop_public_key: Vec<u8>,
    /// Local network IP address
    pub local_ip: String,
    /// Alias for local_ip in test matrices
    #[serde(default)]
    pub host_ip: String,
    /// TCP/TLS listening port
    pub tcp_port: u16,
    /// Alias for tcp_port in mobile client bindings
    #[serde(default)]
    pub tls_port: u16,
    /// Custom 128-bit BLE GATT Service UUID
    pub ble_service_uuid: String,
    /// 6-digit verification PIN displayed on screen
    pub verification_pin: String,
    /// Expiry timestamp in UTC milliseconds
    pub expiry_timestamp_millis: i64,
    /// Expiry in seconds timestamp alias
    #[serde(default)]
    pub expires_at: i64,
}

pub struct QrPairingManager;

impl QrPairingManager {
    /// Generates a new QR pairing payload valid for `validity_seconds` (default 300s = 5m).
    pub fn generate_payload(
        pc_uuid: &str,
        pc_pub_key_bytes: &[u8],
        local_ip: &str,
        tcp_port: u16,
        ble_service_uuid: &str,
        validity_seconds: i64,
    ) -> QrPairingPayload {
        let pc_pub_key_base64 = URL_SAFE_NO_PAD.encode(pc_pub_key_bytes);
        let mut rng = rand::thread_rng();
        let pin_num: u32 = rng.gen_range(100_000..999_999);
        let verification_pin = format!("{:06}", pin_num);
        let expiry = Utc::now().timestamp_millis() + (validity_seconds * 1000);

        QrPairingPayload {
            pc_uuid: pc_uuid.to_string(),
            pc_pub_key_base64,
            desktop_public_key: pc_pub_key_bytes.to_vec(),
            local_ip: local_ip.to_string(),
            host_ip: local_ip.to_string(),
            tcp_port,
            tls_port: tcp_port,
            ble_service_uuid: ble_service_uuid.to_string(),
            verification_pin,
            expiry_timestamp_millis: expiry,
            expires_at: expiry / 1000,
        }
    }

    /// Encodes a `QrPairingPayload` into a deep-link URI: `opentap://pair?data=<base64_json>`.
    pub fn encode_to_uri(payload: &QrPairingPayload) -> Result<String, PairingError> {
        let json_bytes = serde_json::to_vec(payload)
            .map_err(|_| PairingError::InvalidQrUri)?;
        let encoded = URL_SAFE_NO_PAD.encode(json_bytes);
        Ok(format!("opentap://pair?data={}", encoded))
    }

    /// Decodes and validates a QR URI string scanned by the mobile application.
    pub fn decode_from_uri(uri: &str) -> Result<QrPairingPayload, PairingError> {
        const PREFIX: &str = "opentap://pair?data=";
        if !uri.starts_with(PREFIX) {
            return Err(PairingError::InvalidQrUri);
        }

        let base64_str = &uri[PREFIX.len()..];
        let json_bytes = URL_SAFE_NO_PAD
            .decode(base64_str)
            .map_err(|_| PairingError::InvalidQrUri)?;

        let mut payload: QrPairingPayload = serde_json::from_slice(&json_bytes)
            .map_err(|_| PairingError::InvalidQrUri)?;

        if payload.host_ip.is_empty() {
            payload.host_ip = payload.local_ip.clone();
        }
        if payload.desktop_public_key.is_empty() && !payload.pc_pub_key_base64.is_empty() {
            if let Ok(bytes) = URL_SAFE_NO_PAD.decode(&payload.pc_pub_key_base64) {
                payload.desktop_public_key = bytes;
            }
        }
        if payload.tls_port == 0 {
            payload.tls_port = payload.tcp_port;
        }
        if payload.expires_at == 0 {
            payload.expires_at = payload.expiry_timestamp_millis / 1000;
        }

        let now_ms = Utc::now().timestamp_millis();
        if now_ms > payload.expiry_timestamp_millis {
            return Err(PairingError::QrExpired);
        }

        Ok(payload)
    }

    /// Verifies that a user-provided 6-digit PIN matches the OOB PIN in the QR code.
    pub fn verify_pin(expected_pin: &str, input_pin: &str) -> Result<(), PairingError> {
        if expected_pin == input_pin {
            Ok(())
        } else {
            Err(PairingError::PinMismatch)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingState {
    Unpaired,
    QrDisplayed { pin: String, expiry: i64 },
    HandshakeReceived { mobile_device_id: String, mobile_pub_key: Vec<u8> },
    PairedTrusted { mobile_device_id: String },
    Failed(String),
}

/// State machine governing the out-of-band device pairing flow.
pub struct PairingStateMachine {
    state: PairingState,
}

impl PairingStateMachine {
    pub fn new() -> Self {
        Self {
            state: PairingState::Unpaired,
        }
    }

    pub fn current_state(&self) -> &PairingState {
        &self.state
    }

    pub fn start_pairing(&mut self, pin: String, validity_seconds: i64) {
        let expiry = Utc::now().timestamp_millis() + (validity_seconds * 1000);
        self.state = PairingState::QrDisplayed { pin, expiry };
    }

    pub fn receive_handshake(&mut self, mobile_device_id: String, mobile_pub_key: Vec<u8>) -> Result<(), PairingError> {
        match &self.state {
            PairingState::QrDisplayed { expiry, .. } => {
                if Utc::now().timestamp_millis() > *expiry {
                    self.state = PairingState::Failed("QR Code Expired".to_string());
                    return Err(PairingError::QrExpired);
                }
                self.state = PairingState::HandshakeReceived { mobile_device_id, mobile_pub_key };
                Ok(())
            }
            _ => Err(PairingError::InvalidStateTransition("Must be in QrDisplayed state".to_string())),
        }
    }

    pub fn confirm_pin(&mut self, expected_pin: &str, input_pin: &str) -> Result<(), PairingError> {
        QrPairingManager::verify_pin(expected_pin, input_pin)?;
        match self.state.clone() {
            PairingState::HandshakeReceived { mobile_device_id, .. } => {
                self.state = PairingState::PairedTrusted { mobile_device_id };
                Ok(())
            }
            _ => Err(PairingError::InvalidStateTransition("Must receive handshake before PIN confirm".to_string())),
        }
    }
}

impl Default for PairingStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_payload_generation_and_uri_roundtrip() {
        let pk = [77u8; 32];
        let payload = QrPairingManager::generate_payload(
            "pc-test-01",
            &pk,
            "192.168.1.100",
            8080,
            "6f70656e-7461-702d-756e-6c6f636b3031",
            300,
        );

        assert_eq!(payload.verification_pin.len(), 6);

        let uri = QrPairingManager::encode_to_uri(&payload).unwrap();
        assert!(uri.starts_with("opentap://pair?data="));

        let decoded = QrPairingManager::decode_from_uri(&uri).unwrap();
        assert_eq!(payload, decoded);
    }

    #[test]
    fn test_expired_qr_uri_rejection() {
        let pk = [0u8; 32];
        let payload = QrPairingManager::generate_payload(
            "pc-test-02",
            &pk,
            "10.0.0.5",
            443,
            "6f70656e-7461-702d-756e-6c6f636b3031",
            -10, // Expired 10 seconds ago
        );

        let uri = QrPairingManager::encode_to_uri(&payload).unwrap();
        assert_eq!(
            QrPairingManager::decode_from_uri(&uri),
            Err(PairingError::QrExpired)
        );
    }

    #[test]
    fn test_pairing_state_machine() {
        let mut sm = PairingStateMachine::new();
        assert_eq!(*sm.current_state(), PairingState::Unpaired);

        sm.start_pairing("123456".to_string(), 300);
        assert!(matches!(sm.current_state(), PairingState::QrDisplayed { .. }));

        sm.receive_handshake("pixel-8-pro".to_string(), vec![1, 2, 3]).unwrap();
        assert!(matches!(sm.current_state(), PairingState::HandshakeReceived { .. }));

        assert_eq!(sm.confirm_pin("123456", "999999"), Err(PairingError::PinMismatch));

        sm.confirm_pin("123456", "123456").unwrap();
        assert!(matches!(sm.current_state(), PairingState::PairedTrusted { .. }));
    }
}
