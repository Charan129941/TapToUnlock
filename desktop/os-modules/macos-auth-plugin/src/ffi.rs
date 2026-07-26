#![allow(non_camel_case_types, non_snake_case, dead_code)]
use libc::{c_int, c_void, size_t};

pub type OSStatus = i32;
pub type AuthorizationResult = u32;

// Apple OSStatus error codes
pub const errAuthorizationSuccess: OSStatus = 0;
pub const errAuthorizationInternal: OSStatus = -60005;

// AuthorizationResult values (from <Security/AuthorizationPlugin.h>)
pub const kAuthorizationResultAllow: AuthorizationResult = 0;
pub const kAuthorizationResultDeny: AuthorizationResult = 1;
pub const kAuthorizationResultUserCanceled: AuthorizationResult = 2;

/// Opaque handle to the macOS Authorization Engine (loginwindow / screensaver)
#[repr(C)]
pub struct OpaqueAuthorizationEngine {
    _private: [u8; 0],
}
pub type AuthorizationEngineRef = *const OpaqueAuthorizationEngine;

/// Opaque handle to our plugin instance
#[repr(C)]
pub struct OpaqueAuthorizationPlugin {
    _private: [u8; 0],
}
pub type AuthorizationPluginRef = *mut OpaqueAuthorizationPlugin;

/// Opaque handle to our specific authentication mechanism instance
#[repr(C)]
pub struct OpaqueAuthorizationMechanism {
    _private: [u8; 0],
}
pub type AuthorizationMechanismRef = *mut OpaqueAuthorizationMechanism;

/// Apple AuthorizationCallbacks vtable provided by macOS loginwindow when initializing our plugin.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AuthorizationCallbacks {
    pub version: u32,
    pub SetResult: extern "C" fn(inEngine: AuthorizationEngineRef, inResult: AuthorizationResult) -> OSStatus,
    pub RequestSupplementaryOutput: *const c_void,
    pub GetContextValue: *const c_void,
    pub SetContextValue: *const c_void,
    pub GetHintValue: *const c_void,
    pub SetHintValue: *const c_void,
    pub GetArguments: *const c_void,
    pub GetSessionInfo: *const c_void,
}

/// vtable exported by our plugin for Plugin-level lifecycle events.
#[repr(C)]
pub struct AuthorizationPluginInterface {
    pub version: u32,
    pub PluginDestroy: extern "C" fn(inPlugin: AuthorizationPluginRef) -> OSStatus,
    pub MechanismCreate: extern "C" fn(
        inPlugin: AuthorizationPluginRef,
        inEngine: AuthorizationEngineRef,
        inMechanismId: *const c_void,
        outMechanism: *mut AuthorizationMechanismRef,
    ) -> OSStatus,
}

/// vtable exported by our plugin for Mechanism-level execution events (lock screen unlock).
#[repr(C)]
pub struct AuthorizationMechanismInterface {
    pub version: u32,
    pub MechanismInvoke: extern "C" fn(inMechanism: AuthorizationMechanismRef) -> OSStatus,
    pub MechanismDeactivate: extern "C" fn(inMechanism: AuthorizationMechanismRef) -> OSStatus,
    pub MechanismDestroy: extern "C" fn(inMechanism: AuthorizationMechanismRef) -> OSStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_auth_constants() {
        assert_eq!(errAuthorizationSuccess, 0);
        assert_eq!(kAuthorizationResultAllow, 0);
        assert_eq!(kAuthorizationResultUserCanceled, 2);
    }
}
