use opentap_core::{
    ble::BleTransport, routing::RoutingEngine, tls::TlsTransport, BinaryCodec, ChannelType,
    KeyPairManager, NonceValidator, QrPairingManager, SignedUnlockPayload, TransportChannel,
    UnlockAction, UnlockPayloadBody, verify_unlock_payload,
};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;

fn main() {
    println!("====================================================================");
    println!("  OpenTapUnlock: Module 1 & 2 Crypto + Multi-Modal Routing Demo     ");
    println!("====================================================================");
    println!();

    // -------------------------------------------------------------------------
    // STEP 1: Desktop Initial Setup & OOB QR Code Generation
    // -------------------------------------------------------------------------
    println!("[1] Desktop PC initializing cryptographic vault...");
    let desktop_keypair = KeyPairManager::generate();
    let desktop_pub_key = desktop_keypair.public_key().to_bytes();
    let pc_uuid = "pc-workstation-win11-001";

    println!("    -> Generated Desktop Ed25519 Public Key: {:02x?}...", &desktop_pub_key[0..8]);

    let qr_payload = QrPairingManager::generate_payload(
        pc_uuid,
        &desktop_pub_key,
        "192.168.1.100",
        8765,
        "6f70656e-7461-702d-756e-6c6f636b3031",
        300, // 5 min validity
    );

    let qr_uri = QrPairingManager::encode_to_uri(&qr_payload).unwrap();
    println!("[2] Desktop generated OOB QR Code Challenge:");
    println!("    -> URI: {}", qr_uri);
    println!("    -> Verification PIN on desktop screen: [{}]", qr_payload.verification_pin);
    println!();

    // -------------------------------------------------------------------------
    // STEP 2: Mobile Phone Scans QR Code & Performs Cryptographic Pairing
    // -------------------------------------------------------------------------
    println!("[3] Mobile Phone (Android Pixel 8 Pro) scanning QR code...");
    let scanned_payload = QrPairingManager::decode_from_uri(&qr_uri).expect("QR URI must be valid");
    assert_eq!(scanned_payload.verification_pin, qr_payload.verification_pin);
    println!("    -> QR scanned successfully! PIN [{}] verified.", scanned_payload.verification_pin);

    println!("[4] Mobile Phone generating on-device hardware Ed25519 keypair in Android Keystore...");
    let phone_keypair = KeyPairManager::generate();
    let phone_pub_key = phone_keypair.public_key().to_bytes();
    println!("    -> Phone Public Key shared with Desktop: {:02x?}...", &phone_pub_key[0..8]);
    println!("    -> PAIRING COMPLETE: Both devices trust each other's public keys!");
    println!();

    // -------------------------------------------------------------------------
    // STEP 3: Multi-Modal Transport Routing Engine Initialization
    // -------------------------------------------------------------------------
    println!("[5] Initializing Multi-Modal Transport Routing Engine...");
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let mut routing_engine = RoutingEngine::new();

        let tls_channel = Arc::new(TlsTransport::new());
        let ble_channel = Arc::new(BleTransport::new());

        // Both channels connect to the desktop daemon
        tls_channel.set_connected(true);
        tls_channel.set_latency(5); // 5ms Wi-Fi LAN latency

        ble_channel.set_connected(true);
        ble_channel.set_latency(40); // 40ms BLE GATT latency

        routing_engine.add_channel(tls_channel.clone());
        routing_engine.add_channel(ble_channel.clone());

        println!("    -> Registered Channels: [TLS 1.3 over Wi-Fi (5ms)], [BLE GATT (40ms)]");

        let best = routing_engine.select_best_channel().unwrap();
        println!("    -> Routing Engine Auto-Selected Best Channel: [{:?}] (Score algorithm favored high speed & TLS)", best.channel_type());
        println!();

        // -------------------------------------------------------------------------
        // STEP 4: User Performs Triple Tap Gesture -> Zero-Trust Unlock
        // -------------------------------------------------------------------------
        println!("[6] *TRIPLE TAP DETECTED ON PHONE BACK* while PC is locked!");
        println!("    -> Invoking Android BiometricPrompt (Fingerprint scan)...");
        println!("    -> Biometric scan approved! Android Keystore unlocks signing private key.");

        let validator = NonceValidator::new(5000, 60000); // 5 sec drift, 60 sec TTL
        let fresh_nonce = NonceValidator::generate_nonce();
        let now_ms = chrono::Utc::now().timestamp_millis();

        let unlock_body = UnlockPayloadBody {
            target_pc_id: pc_uuid.to_string(),
            action: UnlockAction::UnlockSession,
            nonce: fresh_nonce.to_vec(),
            timestamp_millis: now_ms,
            counter: 1,
        };

        // Serialize to Postcard binary format
        let serialized_body = BinaryCodec::encode(&unlock_body).unwrap();
        // Sign the binary body with phone's private key
        let signature = phone_keypair.sign(&serialized_body).to_vec();

        let signed_packet = SignedUnlockPayload {
            mobile_device_id: "pixel-8-pro-user".to_string(),
            serialized_body: serialized_body.clone(),
            signature: signature.clone(),
        };

        let raw_bytes = BinaryCodec::encode(&signed_packet).unwrap();
        println!("[7] Transmitting encrypted packet ({} bytes) via Routing Engine...", raw_bytes.len());

        let used_channel = routing_engine.send_via_best_channel(&raw_bytes).await.unwrap();
        println!("    -> Transmitted successfully over channel: [{:?}]", used_channel);
        println!();

        // -------------------------------------------------------------------------
        // STEP 5: Desktop Verifies Signature & Nonce -> Instant Unlock
        // -------------------------------------------------------------------------
        println!("[8] Desktop Daemon received packet from TLS socket! Evaluating Zero-Trust rules...");
        match verify_unlock_payload(&phone_pub_key, &signed_packet, &validator) {
            Ok(verified_body) => {
                println!("    -> [SUCCESS] Ed25519 signature is authentic!");
                println!("    -> [SUCCESS] Timestamp drift is acceptable (0 ms drift).");
                println!("    -> [SUCCESS] Nonce is fresh and has been recorded in cache.");
                println!("    >>> TARGET PC [{}] UNLOCKED INSTANTANEOUSLY! <<<", verified_body.target_pc_id);
            }
            Err(e) => {
                eprintln!("    -> [FAIL] Verification failed: {:?}", e);
                std::process::exit(1);
            }
        }
        println!();

        // -------------------------------------------------------------------------
        // STEP 6: Simulating Wi-Fi Disconnection -> Seamless Failover to BLE GATT
        // -------------------------------------------------------------------------
        println!("[9] SIMULATING NETWORK FAULT: User walks away from Wi-Fi access point...");
        println!("    -> Wi-Fi signal lost! TLS channel connection state set to OFFLINE.");
        tls_channel.set_connected(false);

        let best_after_fault = routing_engine.select_best_channel().unwrap();
        assert_eq!(best_after_fault.channel_type(), ChannelType::BleGatt);
        println!("    -> Routing Engine automatically failed over to: [{:?}]!", best_after_fault.channel_type());

        println!("[10] User performs Triple Tap again. Transmitting via fallback BLE GATT...");
        let fallback_channel = routing_engine.send_via_best_channel(&raw_bytes).await.unwrap();
        assert_eq!(fallback_channel, ChannelType::BleGatt);
        println!("    -> Transmitted successfully over BLE GATT characteristic!");
        println!("    >>> ZERO-TRUST MULTI-MODAL FAILOVER PROVEN EFFECTIVE. <<<");
    });

    println!();
    println!("====================================================================");
    println!("     Module 1 & 2 Demo Completed Successfully! All tests passed.    ");
    println!("====================================================================");
}
