use jni::objects::{JClass, JString};
use jni::sys::{jbyteArray, jlong, jstring};
use jni::JNIEnv;
use opentap_core::{
    BinaryCodec, KeyPairManager, NonceValidator, QrPairingManager, SignedUnlockPayload,
    UnlockAction, UnlockPayloadBody,
};
use serde_json::json;

#[no_mangle]
pub extern "system" fn Java_org_opentapunlock_app_jni_OpentapJni_generateKeyPair(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let keypair = KeyPairManager::generate();
    let pub_hex = hex::encode(keypair.public_key().to_bytes());
    let priv_hex = hex::encode(keypair.secret_key_bytes());

    let json_res = json!({
        "public_key_hex": pub_hex,
        "private_key_hex": priv_hex
    });

    let output = env
        .new_string(json_res.to_string())
        .expect("Couldn't create java string!");
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_org_opentapunlock_app_jni_OpentapJni_signUnlockPayload(
    mut env: JNIEnv,
    _class: JClass,
    mobile_device_uuid: JString,
    private_key_hex: JString,
    target_pc_id: JString,
    action_str: JString,
    counter: jlong,
) -> jbyteArray {
    let uuid_str: String = env.get_string(&mobile_device_uuid).expect("Invalid UUID").into();
    let priv_hex_str: String = env.get_string(&private_key_hex).expect("Invalid Key").into();
    let pc_id_str: String = env.get_string(&target_pc_id).expect("Invalid PC ID").into();
    let act_str: String = env.get_string(&action_str).expect("Invalid Action").into();

    let priv_bytes = match hex::decode(&priv_hex_str) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => return std::ptr::null_mut(),
    };

    let keypair = match KeyPairManager::from_secret_bytes(&priv_bytes) {
        Ok(k) => k,
        Err(_) => return std::ptr::null_mut(),
    };

    let action = match act_str.to_uppercase().as_str() {
        "LOCK" => UnlockAction::LockSession,
        "SLEEP" => UnlockAction::SleepDevice,
        "MUTE" => UnlockAction::MuteAudio,
        _ => UnlockAction::UnlockSession,
    };

    let body = UnlockPayloadBody {
        target_pc_id: pc_id_str,
        action,
        nonce: NonceValidator::generate_nonce().to_vec(),
        timestamp_millis: chrono::Utc::now().timestamp_millis(),
        counter: counter as u64,
    };

    let serialized_body = match BinaryCodec::encode(&body) {
        Ok(b) => b,
        Err(_) => return std::ptr::null_mut(),
    };

    let signature = keypair.sign(&serialized_body).to_vec();

    let signed_payload = SignedUnlockPayload {
        mobile_device_id: uuid_str,
        serialized_body,
        signature,
    };

    let packet_bytes = match BinaryCodec::encode(&signed_payload) {
        Ok(b) => b,
        Err(_) => return std::ptr::null_mut(),
    };

    let jbyte_array = env.new_byte_array(packet_bytes.len() as i32).expect("Byte array fail");
    let slice: &[i8] = unsafe { std::mem::transmute(packet_bytes.as_slice()) };
    env.set_byte_array_region(&jbyte_array, 0, slice).expect("Copy fail");
    jbyte_array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_org_opentapunlock_app_jni_OpentapJni_parseQrUri(
    mut env: JNIEnv,
    _class: JClass,
    uri_str: JString,
) -> jstring {
    let uri: String = env.get_string(&uri_str).expect("Invalid URI string").into();
    
    let res = match QrPairingManager::decode_from_uri(&uri) {
        Ok(payload) => {
            let pub_hex = hex::encode(payload.desktop_public_key);
            json!({
                "status": "SUCCESS",
                "pc_uuid": payload.pc_uuid,
                "desktop_public_key_hex": pub_hex,
                "host_ip": payload.host_ip,
                "tls_port": payload.tls_port,
                "ble_service_uuid": payload.ble_service_uuid,
                "verification_pin": payload.verification_pin
            })
        }
        Err(e) => json!({
            "status": "ERROR",
            "message": format!("{:?}", e)
        }),
    };

    let output = env
        .new_string(res.to_string())
        .expect("Couldn't create java string!");
    output.into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jni_logic_simulation() {
        let keypair = KeyPairManager::generate();
        assert_eq!(keypair.public_key().to_bytes().len(), 32);

        let qr = QrPairingManager::generate_payload(
            "pc-123",
            &keypair.public_key().to_bytes(),
            "192.168.1.50",
            8765,
            "6f70656e-7461-702d-756e-6c6f636b3031",
            300,
        );
        let uri = QrPairingManager::encode_to_uri(&qr).unwrap();
        let decoded = QrPairingManager::decode_from_uri(&uri);
        assert!(decoded.is_ok());
    }
}
