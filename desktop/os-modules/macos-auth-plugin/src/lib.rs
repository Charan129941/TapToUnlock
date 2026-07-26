pub mod ffi;
pub mod keychain;
pub mod mechanism;
pub mod socket_client;

pub use ffi::*;
pub use keychain::{KeychainError, MacKeychainHelper, KEYCHAIN_SERVICE_NAME};
pub use mechanism::get_mechanism_interface;
pub use socket_client::{DaemonUnixClient, MacAuthResponse, MacClientError, DEFAULT_MACOS_SOCKET_PATH};

use libc::c_void;

extern "C" fn plugin_destroy(_inPlugin: AuthorizationPluginRef) -> OSStatus {
    errAuthorizationSuccess
}

extern "C" fn mechanism_create(
    _inPlugin: AuthorizationPluginRef,
    _inEngine: AuthorizationEngineRef,
    _inMechanismId: *const c_void,
    outMechanism: *mut AuthorizationMechanismRef,
) -> OSStatus {
    if outMechanism.is_null() {
        return errAuthorizationInternal;
    }
    unsafe {
        *outMechanism = std::ptr::null_mut();
    }
    errAuthorizationSuccess
}

static PLUGIN_INTERFACE: AuthorizationPluginInterface = AuthorizationPluginInterface {
    version: 0,
    PluginDestroy: plugin_destroy,
    MechanismCreate: mechanism_create,
};

/// Primary entry point called by macOS SecurityAgent / loginwindow when loading OpenTapAuthPlugin.bundle.
#[no_mangle]
pub extern "C" fn AuthorizationPluginCreate(
    _callbacks: *const AuthorizationCallbacks,
    outPlugin: *mut AuthorizationPluginRef,
    outInterface: *mut *const AuthorizationPluginInterface,
) -> OSStatus {
    if outPlugin.is_null() || outInterface.is_null() {
        return errAuthorizationInternal;
    }

    unsafe {
        *outPlugin = std::ptr::null_mut();
        *outInterface = &PLUGIN_INTERFACE as *const AuthorizationPluginInterface;
    }

    errAuthorizationSuccess
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_plugin_create() {
        let mut plugin_ptr: AuthorizationPluginRef = std::ptr::null_mut();
        let mut iface_ptr: *const AuthorizationPluginInterface = std::ptr::null();

        let status = AuthorizationPluginCreate(
            std::ptr::null(),
            &mut plugin_ptr as *mut AuthorizationPluginRef,
            &mut iface_ptr as *mut *const AuthorizationPluginInterface,
        );

        assert_eq!(status, errAuthorizationSuccess);
        assert!(!iface_ptr.is_null());

        unsafe {
            let ver = (*iface_ptr).version;
            assert_eq!(ver, 0);
        }
    }
}
