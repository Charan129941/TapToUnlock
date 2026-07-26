use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("Postcard binary serialization error: {0}")]
    BinarySerializationError(String),
    #[error("JSON serialization error: {0}")]
    JsonSerializationError(String),
    #[error("Malformed message payload")]
    MalformedPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DeviceType {
    Android,
    Ios,
    Windows,
    Linux,
    MacOs,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum UnlockAction {
    /// Unlock user desktop session (logon screen or screensaver)
    UnlockSession,
    /// Elevate Windows UAC prompt
    ElevateUac,
    /// Elevate Linux sudo / polkit prompt
    ElevateSudo,
    /// Lock the target desktop immediately
    LockSession,
    /// Put target workstation to sleep / suspend
    SleepDevice,
    /// Mute system audio output
    MuteAudio,
    /// Emergency revocation: kill session and revoke paired device
    RevokeDevice,
}

/// Initial handshake request sent by the mobile device after scanning the PC QR code.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PairingRequest {
    /// 32-byte Ed25519 public key of the mobile device
    pub mobile_pub_key: Vec<u8>,
    /// Human-readable device name (e.g., "Pixel 8 Pro")
    pub device_name: String,
    /// Operating system of the client
    pub device_type: DeviceType,
    /// Signature of the PC's public key (scanned from QR) to prove key ownership
    pub qr_challenge_signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PairingStatus {
    Success,
    InvalidSignature,
    DeviceAlreadyPaired,
    PinRequired,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PairingResponse {
    pub status: PairingStatus,
    pub server_name: String,
    pub assigned_device_id: String,
    pub message: Option<String>,
}

/// Request sent by phone to obtain a fresh cryptographic challenge (nonce + timestamp).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ChallengeRequest {
    pub mobile_device_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ChallengeResponse {
    /// 32-byte random cryptographic nonce
    pub nonce: Vec<u8>,
    /// Current desktop server UTC timestamp in milliseconds
    pub server_timestamp_millis: i64,
}

/// Core Zero-Trust unlock payload sent by mobile after successful biometric scan.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UnlockPayloadBody {
    /// Target PC unique ID
    pub target_pc_id: String,
    /// Action to perform
    pub action: UnlockAction,
    /// 32-byte nonce received from ChallengeResponse
    pub nonce: Vec<u8>,
    /// Timestamp when biometric scan completed (UTC millis)
    pub timestamp_millis: i64,
    /// Monotonically increasing session counter to prevent replay
    pub counter: u64,
}

/// Wrapper containing the signed payload and the mobile device's identifier.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SignedUnlockPayload {
    pub mobile_device_id: String,
    /// Postcard-serialized `UnlockPayloadBody`
    pub serialized_body: Vec<u8>,
    /// 64-byte Ed25519 signature of `serialized_body` produced by Keystore / Secure Enclave
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum UnlockResultCode {
    Success,
    SignatureVerificationFailed,
    ReplayDetected,
    TimestampDriftExceeded,
    RateLimited,
    OsAuthenticationError,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UnlockResponse {
    pub code: UnlockResultCode,
    pub detail: String,
}

/// Utility serializer for high-speed, compact binary transmission over BLE GATT / UDP.
pub struct BinaryCodec;

impl BinaryCodec {
    pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
        postcard::to_stdvec(value)
            .map_err(|e| ProtocolError::BinarySerializationError(e.to_string()))
    }

    pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, ProtocolError> {
        postcard::from_bytes(bytes)
            .map_err(|e| ProtocolError::BinarySerializationError(e.to_string()))
    }
}

/// Utility serializer for JSON logging, WebSocket, and debugging.
pub struct JsonCodec;

impl JsonCodec {
    pub fn encode<T: Serialize>(value: &T) -> Result<String, ProtocolError> {
        serde_json::to_string(value)
            .map_err(|e| ProtocolError::JsonSerializationError(e.to_string()))
    }

    pub fn decode<'a, T: Deserialize<'a>>(json: &'a str) -> Result<T, ProtocolError> {
        serde_json::from_str(json)
            .map_err(|e| ProtocolError::JsonSerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_codec_roundtrip() {
        let body = UnlockPayloadBody {
            target_pc_id: "pc-desk-001".to_string(),
            action: UnlockAction::UnlockSession,
            nonce: vec![1, 2, 3, 4, 5, 6, 7, 8],
            timestamp_millis: 1721920050123,
            counter: 42,
        };

        let encoded = BinaryCodec::encode(&body).unwrap();
        assert!(!encoded.is_empty());

        let decoded: UnlockPayloadBody = BinaryCodec::decode(&encoded).unwrap();
        assert_eq!(body, decoded);
    }

    #[test]
    fn test_signed_unlock_payload_json_roundtrip() {
        let payload = SignedUnlockPayload {
            mobile_device_id: "dev-pixel8-pro".to_string(),
            serialized_body: vec![10, 20, 30],
            signature: vec![0xAA; 64],
        };

        let json = JsonCodec::encode(&payload).unwrap();
        assert!(json.contains("dev-pixel8-pro"));

        let decoded: SignedUnlockPayload = JsonCodec::decode(&json).unwrap();
        assert_eq!(payload, decoded);
    }
}
