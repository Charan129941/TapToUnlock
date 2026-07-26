pub mod crypto;
pub mod pairing;
pub mod protocol;
pub mod transport;

pub use crypto::{CryptoError, EcdhManager, KeyPairManager, NonceValidator, SymmetricCipher};
pub use pairing::{PairingError, PairingState, PairingStateMachine, QrPairingManager, QrPairingPayload};
pub use protocol::{
    BinaryCodec, ChallengeRequest, ChallengeResponse, DeviceType, JsonCodec, PairingRequest,
    PairingResponse, PairingStatus, ProtocolError, SignedUnlockPayload, UnlockAction,
    UnlockPayloadBody, UnlockResponse, UnlockResultCode,
};
pub use transport::{
    ble::{BleTransport, ProximityFilter, OPENTAP_CHAR_NOTIFY_UUID, OPENTAP_CHAR_WRITE_UUID, OPENTAP_SERVICE_UUID},
    mdns::{DiscoveredPeer, MdnsDiscoveryEngine, ServiceTxtRecord, OPENTAP_MDNS_SERVICE},
    routing::{LinkScorer, RoutingEngine, RoutingEngineConfig, RoutingMethod},
    tls::{EphemeralCertBundle, TlsTransport},
    ChannelType, TransportChannel, TransportError,
};
pub use ed25519_dalek;

/// High-level facade for verifying an incoming signed unlock payload against a trusted public key.
pub fn verify_unlock_payload(
    trusted_pub_key_bytes: &[u8],
    signed_payload: &SignedUnlockPayload,
    nonce_validator: &NonceValidator,
) -> Result<UnlockPayloadBody, CryptoError> {
    // 1. Verify Ed25519 signature over the serialized body
    KeyPairManager::verify_slice(
        trusted_pub_key_bytes,
        &signed_payload.serialized_body,
        &signed_payload.signature,
    )?;

    // 2. Decode binary Postcard body
    let body: UnlockPayloadBody = BinaryCodec::decode(&signed_payload.serialized_body)
        .map_err(|_| CryptoError::SignatureVerificationFailed)?;

    // 3. Validate Nonce replay and Timestamp drift
    nonce_validator.validate(&body.nonce, body.timestamp_millis)?;

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_facade_verify_unlock_payload_success() {
        let manager = KeyPairManager::generate();
        let pk_bytes = manager.public_key().to_bytes();
        let validator = NonceValidator::new(5000, 60000);

        let body = UnlockPayloadBody {
            target_pc_id: "pc-desk-master".to_string(),
            action: UnlockAction::UnlockSession,
            nonce: NonceValidator::generate_nonce().to_vec(),
            timestamp_millis: Utc::now().timestamp_millis(),
            counter: 1,
        };

        let serialized_body = BinaryCodec::encode(&body).unwrap();
        let signature = manager.sign(&serialized_body).to_vec();

        let signed_payload = SignedUnlockPayload {
            mobile_device_id: "pixel-8-pro".to_string(),
            serialized_body,
            signature,
        };

        let result = verify_unlock_payload(&pk_bytes, &signed_payload, &validator);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().target_pc_id, "pc-desk-master");
    }

    #[test]
    fn test_facade_verify_unlock_payload_rejects_replay() {
        let manager = KeyPairManager::generate();
        let pk_bytes = manager.public_key().to_bytes();
        let validator = NonceValidator::new(5000, 60000);

        let body = UnlockPayloadBody {
            target_pc_id: "pc-desk-master".to_string(),
            action: UnlockAction::UnlockSession,
            nonce: NonceValidator::generate_nonce().to_vec(),
            timestamp_millis: Utc::now().timestamp_millis(),
            counter: 2,
        };

        let serialized_body = BinaryCodec::encode(&body).unwrap();
        let signature = manager.sign(&serialized_body).to_vec();

        let signed_payload = SignedUnlockPayload {
            mobile_device_id: "pixel-8-pro".to_string(),
            serialized_body,
            signature,
        };

        // First verification succeeds
        assert!(verify_unlock_payload(&pk_bytes, &signed_payload, &validator).is_ok());

        // Second verification with exact same payload must fail due to Replay protection!
        assert_eq!(
            verify_unlock_payload(&pk_bytes, &signed_payload, &validator),
            Err(CryptoError::ReplayAttackDetected)
        );
    }
}
