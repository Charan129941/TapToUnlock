use windows_core::GUID;

/// COM Class ID (CLSID) for OpenTap Credential Provider: {6F70656E-7461-702D-756E-6C6F636B3034}
pub const CLSID_OPENTAP_CRED_PROVIDER: GUID = GUID::from_u128(0x6f70656e_7461_702d_756e_6c6f636b3034);

/// String representation of COM CLSID for Registry keys
pub const CLSID_STRING: &str = "{6F70656E-7461-702D-756E-6C6F636B3034}";

/// Registry path for COM Class registration
pub const CLSID_REG_PATH: &str = "CLSID\\{6F70656E-7461-702D-756E-6C6F636B3034}";

/// Registry path for Winlogon Credential Provider registration
pub const AUTH_REG_PATH: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\{6F70656E-7461-702D-756E-6C6F636B3034}";

/// Field identifiers for user tile UI components on Windows Lock Screen / LogonUI
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFieldId {
    /// Tile icon / logo image
    Logo = 0,
    /// Primary large status string (e.g., "Triple Tap Phone to Unlock")
    StatusText = 1,
    /// Secondary small info string (e.g., "Connected via BLE / mTLS")
    ConnectionInfo = 2,
    /// Manual retry / submit action button
    SubmitButton = 3,
}

pub const FIELD_COUNT: u32 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clsid_guid_formatting() {
        let formatted = format!("{:?}", CLSID_OPENTAP_CRED_PROVIDER);
        assert!(!formatted.is_empty());
        assert_eq!(FIELD_COUNT, 4);
    }
}
