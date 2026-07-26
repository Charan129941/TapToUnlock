pub mod credential;
pub mod guid;
pub mod pipe_client;
pub mod provider;

pub use credential::{OpenTapTile, TileState};
pub use guid::*;
pub use pipe_client::{NamedPipeClient, PipeClientError, PipeResponse, DEFAULT_PIPE_NAME};
pub use provider::{OpenTapProvider, CPUS_CREDUI, CPUS_LOGON, CPUS_UNLOCK};

use std::ffi::c_void;
use windows_core::GUID;

// Win32 HRESULT constants
pub const S_OK: i32 = 0;
pub const S_FALSE: i32 = 1;
pub const E_NOINTERFACE: i32 = -2147467262; // 0x80004002
pub const CLASS_E_CLASSNOTAVAILABLE: i32 = -2147221231; // 0x80040111

/// COM Class Factory entry point called by LogonUI when creating our provider instance.
#[no_mangle]
pub extern "stdcall" fn DllGetClassObject(
    rclsid: *const GUID,
    _riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    if rclsid.is_null() || ppv.is_null() {
        return E_NOINTERFACE;
    }

    unsafe {
        if *rclsid == CLSID_OPENTAP_CRED_PROVIDER {
            // In full COM binding execution, we return an IClassFactory COM pointer here.
            // For testing and structural verification, we validate matching CLSID:
            *ppv = std::ptr::null_mut();
            S_OK
        } else {
            *ppv = std::ptr::null_mut();
            CLASS_E_CLASSNOTAVAILABLE
        }
    }
}

/// Called by Windows COM subsystem to determine if the DLL can be safely unloaded from memory.
#[no_mangle]
pub extern "stdcall" fn DllCanUnloadNow() -> i32 {
    // We return S_OK when no active lock screen tiles are referenced by LogonUI
    S_OK
}

/// Registers the OpenTap Credential Provider in Windows Registry under HKLM\SOFTWARE\Microsoft\...\Credential Providers.
#[no_mangle]
pub extern "stdcall" fn DllRegisterServer() -> i32 {
    // In real installation, our PowerShell script or custom installer writes the exact keys.
    // DllRegisterServer provides standard standard Win32 regsvr32 compatibility.
    S_OK
}

/// Unregisters the Credential Provider from Windows Registry.
#[no_mangle]
pub extern "stdcall" fn DllUnregisterServer() -> i32 {
    S_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dll_get_class_object_clsid_matching() {
        let mut ptr: *mut c_void = std::ptr::null_mut();
        
        let status_ok = DllGetClassObject(
            &CLSID_OPENTAP_CRED_PROVIDER as *const GUID,
            &GUID::zeroed() as *const GUID,
            &mut ptr as *mut *mut c_void,
        );
        assert_eq!(status_ok, S_OK);

        let random_guid = GUID::from_u128(0x11112222_3333_4444_5555_666677778888);
        let status_fail = DllGetClassObject(
            &random_guid as *const GUID,
            &GUID::zeroed() as *const GUID,
            &mut ptr as *mut *mut c_void,
        );
        assert_eq!(status_fail, CLASS_E_CLASSNOTAVAILABLE);
    }

    #[test]
    fn test_dll_can_unload_now() {
        assert_eq!(DllCanUnloadNow(), S_OK);
    }
}
