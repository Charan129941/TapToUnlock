use libc::{c_char, size_t};
use opentap_core::{
    BinaryCodec, KeyPairManager, NonceValidator, QrPairingManager, SignedUnlockPayload,
    UnlockAction, UnlockPayloadBody,
};
use serde_json::json;
use std::ffi::{CStr, CString};
use std::ptr;

#[no_mangle]
pub extern "C" fn opentap_ffi_generate_keypair(
    out_pub_hex: *mut c_char,
    pub_max_len: size_t,
    out_priv_hex: *mut c_char,
    priv_max_len: size_t,
) -> i32 {
    if out_pub_hex.is_null() || out_priv_hex.is_null() {
        return -1;
    }

    let keypair = KeyPairManager::generate();
    let pub_str = hex::encode(keypair.public_key().to_bytes());
    let priv_str = hex::encode(keypair.secret_key_bytes());

    if pub_str.len() >= pub_max_len || priv_str.len() >= priv_max_len {
        return -2; // Buffer too small
    }

    unsafe {
        let pub_c = CString::new(pub_str).unwrap();
        ptr::copy_nonoverlapping(pub_c.as_ptr(), out_pub_hex, pub_c.as_bytes_with_nul().len());

        let priv_c = CString::new(priv_str).unwrap();
        ptr::copy_nonoverlapping(priv_c.as_ptr(), out_priv_hex, priv_c.as_bytes_with_nul().len());
    }

    0
}

#[no_mangle]
pub extern "C" fn opentap_ffi_sign_payload(
    uuid_str: *const c_char,
    priv_hex: *const c_char,
    pc_id: *const c_char,
    action_str: *const c_char,
    counter: u64,
    out_buf: *mut u8,
    out_max_len: size_t,
    out_actual_len: *mut size_t,
) -> i32 {
    if uuid_str.is_null() || priv_hex.is_null() || pc_id.is_null() || action_str.is_null() || out_buf.is_null() || out_actual_len.is_null() {
        return -1;
    }

    let uuid = unsafe { CStr::from_ptr(uuid_str) }.to_string_lossy().to_string();
    let priv_hex_s = unsafe { CStr::from_ptr(priv_hex) }.to_string_lossy();
    let pc = unsafe { CStr::from_ptr(pc_id) }.to_string_lossy().to_string();
    let act = unsafe { CStr::from_ptr(action_str) }.to_string_lossy();

    let priv_bytes = match hex::decode(priv_hex_s.as_ref()) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => return -3,
    };

    let keypair = match KeyPairManager::from_secret_bytes(&priv_bytes) {
        Ok(k) => k,
        Err(_) => return -3,
    };

    let action = match act.to_uppercase().as_str() {
        "LOCK" => UnlockAction::LockSession,
        "SLEEP" => UnlockAction::SleepDevice,
        "MUTE" => UnlockAction::MuteAudio,
        _ => UnlockAction::UnlockSession,
    };

    let body = UnlockPayloadBody {
        target_pc_id: pc,
        action,
        nonce: NonceValidator::generate_nonce().to_vec(),
        timestamp_millis: chrono::Utc::now().timestamp_millis(),
        counter,
    };

    let serialized_body = match BinaryCodec::encode(&body) {
        Ok(b) => b,
        Err(_) => return -4,
    };

    let signature = keypair.sign(&serialized_body).to_vec();

    let signed_payload = SignedUnlockPayload {
        mobile_device_id: uuid,
        serialized_body,
        signature,
    };

    let packet_bytes = match BinaryCodec::encode(&signed_payload) {
        Ok(b) => b,
        Err(_) => return -5,
    };

    if packet_bytes.len() > out_max_len {
        return -2;
    }

    unsafe {
        ptr::copy_nonoverlapping(packet_bytes.as_ptr(), out_buf, packet_bytes.len());
        *out_actual_len = packet_bytes.len();
    }

    0
}

#[no_mangle]
pub extern "C" fn opentap_ffi_parse_qr_uri(
    uri_str: *const c_char,
    out_json: *mut c_char,
    max_len: size_t,
) -> i32 {
    if uri_str.is_null() || out_json.is_null() {
        return -1;
    }

    let uri = unsafe { CStr::from_ptr(uri_str) }.to_string_lossy();

    let res_json = match QrPairingManager::decode_from_uri(uri.as_ref()) {
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

    let json_s = res_json.to_string();
    let c_str = match CString::new(json_s) {
        Ok(s) => s,
        Err(_) => return -6,
    };

    if c_str.as_bytes_with_nul().len() > max_len {
        return -2;
    }

    unsafe {
        ptr::copy_nonoverlapping(c_str.as_ptr(), out_json, c_str.as_bytes_with_nul().len());
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_keypair_generation() {
        let mut pub_buf = [0i8; 128];
        let mut priv_buf = [0i8; 128];

        let status = opentap_ffi_generate_keypair(
            pub_buf.as_mut_ptr(),
            pub_buf.len(),
            priv_buf.as_mut_ptr(),
            priv_buf.len(),
        );

        assert_eq!(status, 0);
    }
}
