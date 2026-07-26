#![allow(non_camel_case_types, dead_code)]
use libc::{c_int, c_char, c_void};
use std::ffi::CStr;

/// Opaque handle to the PAM authentication transaction and state.
#[repr(C)]
pub struct pam_handle_t {
    _private: [u8; 0],
}

// Standard PAM return codes (from <security/_pam_types.h>)
pub const PAM_SUCCESS: c_int = 0;
pub const PAM_OPEN_ERR: c_int = 1;
pub const PAM_SYMBOL_ERR: c_int = 2;
pub const PAM_SERVICE_ERR: c_int = 3;
pub const PAM_SYSTEM_ERR: c_int = 4;
pub const PAM_BUF_ERR: c_int = 5;
pub const PAM_PERM_DENIED: c_int = 6;
pub const PAM_AUTH_ERR: c_int = 7;
pub const PAM_CRED_INSUFFICIENT: c_int = 8;
pub const PAM_AUTHINFO_UNAVAIL: c_int = 9;
pub const PAM_USER_UNKNOWN: c_int = 10;
pub const PAM_MAXTRIES: c_int = 11;
pub const PAM_NEW_AUTHTOK_REQD: c_int = 12;
pub const PAM_ACCT_EXPIRED: c_int = 13;
pub const PAM_SESSION_ERR: c_int = 14;
pub const PAM_CRED_UNAVAIL: c_int = 15;
pub const PAM_CRED_EXPIRED: c_int = 16;
pub const PAM_CRED_ERR: c_int = 17;
pub const PAM_NO_MODULE_DATA: c_int = 18;
pub const PAM_CONV_ERR: c_int = 19;
pub const PAM_AUTHTOK_ERR: c_int = 20;
pub const PAM_AUTHTOK_RECOVERY_ERR: c_int = 21;
pub const PAM_AUTHTOK_LOCK_BUSY: c_int = 22;
pub const PAM_AUTHTOK_DISABLE_AGING: c_int = 23;
pub const PAM_TRY_AGAIN: c_int = 24;
pub const PAM_IGNORE: c_int = 25;
pub const PAM_ABORT: c_int = 26;
pub const PAM_AUTHTOK_EXPIRED: c_int = 27;
pub const PAM_MODULE_UNKNOWN: c_int = 28;
pub const PAM_BAD_ITEM: c_int = 29;

// Standard PAM item flags
pub const PAM_SERVICE: c_int = 1;
pub const PAM_USER: c_int = 2;
pub const PAM_TTY: c_int = 3;
pub const PAM_RHOST: c_int = 4;
pub const PAM_CONV: c_int = 5;
pub const PAM_AUTHTOK: c_int = 6;

// PAM flags
pub const PAM_SILENT: c_int = 0x8000;
pub const PAM_DISALLOW_NULL_AUTHTOK: c_int = 0x0001;
pub const PAM_ESTABLISH_CRED: c_int = 0x0002;
pub const PAM_DELETE_CRED: c_int = 0x0004;
pub const PAM_REINITIALIZE_CRED: c_int = 0x0008;
pub const PAM_REFRESH_CRED: c_int = 0x0010;

extern "C" {
    /// Retrieves a PAM item from the active transaction handle.
    pub fn pam_get_item(
        pamh: *const pam_handle_t,
        item_type: c_int,
        item: *mut *const c_void,
    ) -> c_int;

    /// Sets a PAM item in the active transaction handle.
    pub fn pam_set_item(
        pamh: *mut pam_handle_t,
        item_type: c_int,
        item: *const c_void,
    ) -> c_int;
}

/// Safely extracts the target username (`PAM_USER`) from the PAM handle.
pub unsafe fn extract_pam_user(pamh: *const pam_handle_t) -> Result<String, c_int> {
    let mut item_ptr: *const c_void = std::ptr::null();
    let status = pam_get_item(pamh, PAM_USER, &mut item_ptr);

    if status != PAM_SUCCESS {
        return Err(status);
    }
    if item_ptr.is_null() {
        return Err(PAM_USER_UNKNOWN);
    }

    let c_str = CStr::from_ptr(item_ptr as *const c_char);
    match c_str.to_str() {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Err(PAM_CONV_ERR),
    }
}

/// Safely extracts the service name (`PAM_SERVICE`) from the PAM handle (e.g., "sudo" or "gdm").
pub unsafe fn extract_pam_service(pamh: *const pam_handle_t) -> Result<String, c_int> {
    let mut item_ptr: *const c_void = std::ptr::null();
    let status = pam_get_item(pamh, PAM_SERVICE, &mut item_ptr);

    if status != PAM_SUCCESS || item_ptr.is_null() {
        return Ok("unknown_service".to_string());
    }

    let c_str = CStr::from_ptr(item_ptr as *const c_char);
    match c_str.to_str() {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Ok("unknown_service".to_string()),
    }
}
