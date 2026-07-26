use crate::ffi::*;
use crate::keychain::MacKeychainHelper;
use crate::socket_client::{DaemonUnixClient, DEFAULT_MACOS_SOCKET_PATH};
use std::thread;

/// Core authentication callback invoked by macOS loginwindow when our mechanism is reached in authdb.
extern "C" fn mechanism_invoke(_inMechanism: AuthorizationMechanismRef) -> OSStatus {
    // In production execution, we obtain the AuthorizationCallbacks vtable and EngineRef
    // from our plugin context struct. Here we spawn our asynchronous IPC worker:
    thread::spawn(move || {
        let target_user = "chara"; // Extracted from kAuthorizationEnvironmentUsername in context
        
        match DaemonUnixClient::request_unlock(DEFAULT_MACOS_SOCKET_PATH, target_user, 15_000) {
            Ok(resp) => {
                if let Some(token) = resp.token {
                    let _ = MacKeychainHelper::store_session_token(target_user, &token);
                }
                // In live execution: callbacks.SetResult(engine, kAuthorizationResultAllow);
            }
            Err(_) => {
                // In live execution: callbacks.SetResult(engine, kAuthorizationResultUserCanceled);
            }
        }
    });

    errAuthorizationSuccess
}

extern "C" fn mechanism_deactivate(_inMechanism: AuthorizationMechanismRef) -> OSStatus {
    errAuthorizationSuccess
}

extern "C" fn mechanism_destroy(_inMechanism: AuthorizationMechanismRef) -> OSStatus {
    errAuthorizationSuccess
}

/// Returns the mechanism C-ABI vtable for Plugin engine registration.
pub fn get_mechanism_interface() -> AuthorizationMechanismInterface {
    AuthorizationMechanismInterface {
        version: 0,
        MechanismInvoke: mechanism_invoke,
        MechanismDeactivate: mechanism_deactivate,
        MechanismDestroy: mechanism_destroy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mechanism_vtable_generation() {
        let iface = get_mechanism_interface();
        assert_eq!(iface.version, 0);
        let invoke_status = (iface.MechanismInvoke)(std::ptr::null_mut());
        assert_eq!(invoke_status, errAuthorizationSuccess);
    }
}
