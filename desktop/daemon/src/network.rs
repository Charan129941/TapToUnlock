use crate::config::{PairedDeviceInfo, PairedDeviceStore};
use crate::state::AuthStateMachine;
use log::{error, info, warn};
use opentap_core::{
    BinaryCodec, KeyPairManager, NonceValidator, SignedUnlockPayload, UnlockAction,
    UnlockPayloadBody, verify_unlock_payload,
};
use std::sync::{Arc, Mutex};

/// Coordinates BLE GATT, mDNS ZeroConf, and mTLS 1.3 network listeners and routes packets to crypto engine.
#[derive(Clone)]
pub struct NetworkCoordinator {
    store: Arc<Mutex<PairedDeviceStore>>,
    state_machine: AuthStateMachine,
    validator: Arc<NonceValidator>,
}

impl NetworkCoordinator {
    pub fn new(store: PairedDeviceStore, state_machine: AuthStateMachine) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            state_machine,
            validator: Arc::new(NonceValidator::new(5000, 60000)), // 5s drift, 60s TTL
        }
    }

    /// Evaluates an incoming binary packet from any network channel (BLE, Wi-Fi TLS, Wi-Fi Direct).
    pub fn handle_incoming_packet(&self, raw_bytes: &[u8]) -> Result<bool, String> {
        // 1. Decode outer postcard binary envelope
        let signed_payload: SignedUnlockPayload = match BinaryCodec::decode(raw_bytes) {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Failed to decode postcard binary envelope: {:?}", e);
                warn!("{}", msg);
                return Err(msg);
            }
        };

        // 2. Look up paired mobile device in disk keystore
        let store = self.store.lock().unwrap();
        let device_info = match store.find_device(&signed_payload.mobile_device_id) {
            Some(info) => info.clone(),
            None => {
                let msg = format!(
                    "SECURITY REJECTION: Received packet from unpaired device ID '{}'",
                    signed_payload.mobile_device_id
                );
                warn!("{}", msg);
                return Err(msg);
            }
        };

        let pub_key_bytes = match device_info.public_key_bytes() {
            Ok(b) => b,
            Err(_) => {
                let msg = "Stored public key is corrupted in device vault".to_string();
                error!("{}", msg);
                return Err(msg);
            }
        };

        // 3. Execute zero-trust cryptographic verification (Ed25519 signature + timestamp drift + nonce cache)
        match verify_unlock_payload(&pub_key_bytes, &signed_payload, &self.validator) {
            Ok(verified_body) => {
                info!(
                    "CRYPTOGRAPHIC SUCCESS: Verified Triple Tap from paired device '{}' (action: {:?})",
                    device_info.device_name, verified_body.action
                );

                // 4. Notify OS IPC State Machine to release lock screen token!
                let unlocked = self.state_machine.process_verified_mobile_payload(
                    &verified_body.target_pc_id,
                    &device_info.device_name,
                );

                if unlocked {
                    info!("-> Successfully unlocked active OS login challenge!");
                } else {
                    info!("-> Verified packet received, but no pending OS login challenge was waiting.");
                }
                Ok(true)
            }
            Err(crypto_err) => {
                let msg = format!(
                    "SECURITY DEFENSE TRIGGERED: Packet verification failed: {:?}",
                    crypto_err
                );
                error!("{}", msg);
                Err(msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_coordinator_packet_routing_and_rejection() {
        let mut store = PairedDeviceStore::default();
        let phone_keypair = KeyPairManager::generate();
        let phone_pub_key = phone_keypair.public_key().to_bytes();
        let pub_hex = hex::encode(phone_pub_key);

        let info = PairedDeviceInfo {
            device_uuid: "pixel-8-pro-uuid".to_string(),
            device_name: "Chara's Pixel".to_string(),
            public_key_hex: pub_hex,
            paired_at_utc: 1700000000,
        };
        store.add_device(info);

        let sm = AuthStateMachine::new();
        sm.register_os_challenge("chara", "sudo", 5000);

        let coordinator = NetworkCoordinator::new(store, sm.clone());

        // 1. Create valid signed unlock payload
        let unlock_body = UnlockPayloadBody {
            target_pc_id: "chara".to_string(),
            action: UnlockAction::UnlockSession,
            nonce: NonceValidator::generate_nonce().to_vec(),
            timestamp_millis: chrono::Utc::now().timestamp_millis(),
            counter: 1,
        };
        let serialized_body = BinaryCodec::encode(&unlock_body).unwrap();
        let signature = phone_keypair.sign(&serialized_body).to_vec();

        let signed_packet = SignedUnlockPayload {
            mobile_device_id: "pixel-8-pro-uuid".to_string(),
            serialized_body: serialized_body.clone(),
            signature: signature.clone(),
        };

        let raw_bytes = BinaryCodec::encode(&signed_packet).unwrap();

        // Send valid packet -> should verify and unlock!
        let ok = coordinator.handle_incoming_packet(&raw_bytes);
        assert!(ok.is_ok());
        assert_eq!(ok.unwrap(), true);

        // 2. Send same packet again immediately -> REPLAY ATTACK DETECTION MUST REJECT IT!
        let replay = coordinator.handle_incoming_packet(&raw_bytes);
        assert!(replay.is_err());
        assert!(replay.unwrap_err().contains("SECURITY DEFENSE TRIGGERED"));

        // 3. Send packet from unknown unpaired device ID -> MUST REJECT!
        let fake_packet = SignedUnlockPayload {
            mobile_device_id: "unpaired-hacker-phone".to_string(),
            serialized_body,
            signature,
        };
        let fake_bytes = BinaryCodec::encode(&fake_packet).unwrap();
        let rejected = coordinator.handle_incoming_packet(&fake_bytes);
        assert!(rejected.is_err());
        assert!(rejected.unwrap_err().contains("unpaired device ID"));
    }
}

mod hex {
    pub fn encode(bytes: [u8; 32]) -> String {
        let mut s = String::with_capacity(64);
        for &b in &bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}
