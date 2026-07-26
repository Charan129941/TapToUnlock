pub mod client;
pub mod ffi;

use client::{DaemonSocketClient, DEFAULT_SOCKET_PATH};
use ffi::*;
use libc::{c_char, c_int};

/// Helper for logging security audit messages to syslog on Linux or stderr on other targets.
fn audit_log(level: &str, msg: &str) {
    #[cfg(target_os = "linux")]
    {
        use syslog::{Facility, Formatter3164};
        if let Ok(mut writer) = syslog::unix(Formatter3164 {
            facility: Facility::LOG_AUTH,
            hostname: None,
            process: "pam_opentap".into(),
            pid: 0,
        }) {
            match level {
                "ERR" => let _ = writer.err(msg),
                "WARN" => let _ = writer.warning(msg),
                _ => let _ = writer.info(msg),
            };
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("[pam_opentap][{}] {}", level, msg);
    }
}

/// Core PAM entry point invoked by login managers (GDM, SDDM), screen lockers, and sudo.
#[no_mangle]
pub extern "C" fn pam_sm_authenticate(
    pamh: *mut pam_handle_t,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    // 1. Safely extract target username
    let username = match unsafe { extract_pam_user(pamh) } {
        Ok(u) => u,
        Err(status) => {
            audit_log("ERR", "Failed to extract PAM_USER from handle");
            return status;
        }
    };

    // 2. Extract requesting service name (e.g., "sudo", "gdm-password", "kscreenlocker")
    let service = unsafe { extract_pam_service(pamh) }.unwrap_or_else(|_| "unknown".to_string());

    audit_log(
        "INFO",
        &format!("Initiating biometric mobile unlock for user '{}' (service: '{}')", username, service),
    );

    // 3. Connect to background OpenTap daemon and request biometric verification (10 sec timeout)
    match DaemonSocketClient::verify_user(DEFAULT_SOCKET_PATH, &username, &service, 10_000) {
        Ok(resp) => {
            let dev_name = resp.device_name.as_deref().unwrap_or("Paired Mobile Device");
            audit_log(
                "INFO",
                &format!("Successfully unlocked user '{}' via Triple Tap on [{}]", username, dev_name),
            );
            PAM_SUCCESS
        }
        Err(client::ClientError::Denied) => {
            audit_log("WARN", &format!("Unlock explicitly denied for user '{}'", username));
            PAM_AUTH_ERR
        }
        Err(e) => {
            // If phone is offline, dead battery, out of range, or daemon is stopped,
            // we return PAM_IGNORE (25). This instructs the Linux PAM stack to seamlessly
            // fall back to standard password prompt (pam_unix.so) without locking user out!
            audit_log(
                "WARN",
                &format!("Mobile unlock unavailable for '{}': {:?}. Falling back to password.", username, e),
            );
            PAM_IGNORE
        }
    }
}

/// PAM credential setting entry point. Called after successful authentication.
#[no_mangle]
pub extern "C" fn pam_sm_setcred(
    _pamh: *mut pam_handle_t,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    PAM_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pam_sm_setcred_always_succeeds() {
        let status = pam_sm_setcred(std::ptr::null_mut(), 0, 0, std::ptr::null());
        assert_eq!(status, PAM_SUCCESS);
    }

    #[test]
    fn test_audit_log_execution() {
        // Must execute cleanly without panicking across Linux and non-Linux targets
        audit_log("INFO", "Test audit log entry");
        audit_log("ERR", "Test error log entry");
    }
}
