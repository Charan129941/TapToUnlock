use crate::credential::OpenTapTile;
use std::sync::Arc;

/// Winlogon Usage Scenarios (from <credentialprovider.h>)
pub const CPUS_LOGON: u32 = 0;
pub const CPUS_UNLOCK: u32 = 1;
pub const CPUS_CHANGE_PASSWORD: u32 = 2;
pub const CPUS_CREDUI: u32 = 3;

/// Core Credential Provider factory class instantiated by Winlogon upon lock screen wake.
pub struct OpenTapProvider {
    tiles: Vec<Arc<OpenTapTile>>,
    current_scenario: u32,
}

impl OpenTapProvider {
    pub fn new() -> Self {
        Self {
            tiles: Vec::new(),
            current_scenario: CPUS_UNLOCK,
        }
    }

    /// Called by LogonUI to inform the provider of the current authentication scenario.
    pub fn set_usage_scenario(&mut self, scenario: u32) -> bool {
        self.current_scenario = scenario;
        // We only provide tiles for Windows Logon, Lock Screen Unlock, and UAC elevation (CredUI)
        match scenario {
            CPUS_LOGON | CPUS_UNLOCK | CPUS_CREDUI => {
                // In production, we enumerate local Windows user accounts from SAM / Active Directory.
                // Here we initialize our active desktop user tile:
                if self.tiles.is_empty() {
                    self.tiles.push(Arc::new(OpenTapTile::new("chara")));
                }
                true
            }
            _ => {
                self.tiles.clear();
                false
            }
        }
    }

    /// Returns the number of user tiles to display on the Lock Screen.
    pub fn get_credential_count(&self) -> u32 {
        self.tiles.len() as u32
    }

    /// Returns the specific user tile handle at the requested index.
    pub fn get_credential_at(&self, index: u32) -> Option<Arc<OpenTapTile>> {
        self.tiles.get(index as usize).cloned()
    }

    pub fn current_scenario(&self) -> u32 {
        self.current_scenario
    }
}

impl Default for OpenTapProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guid::TileFieldId;

    #[test]
    fn test_provider_scenario_filtering() {
        let mut provider = OpenTapProvider::new();
        assert_eq!(provider.get_credential_count(), 0);

        // Scenario: Windows Lock Screen Unlock
        let ok = provider.set_usage_scenario(CPUS_UNLOCK);
        assert!(ok);
        assert_eq!(provider.get_credential_count(), 1);

        let tile = provider.get_credential_at(0).unwrap();
        assert_eq!(tile.username, "chara");
        assert_eq!(tile.get_string_value(TileFieldId::StatusText), "OpenTap Biometric Mobile Unlock");

        // Scenario: Change Password (we do not handle password changes via tap)
        let ignored = provider.set_usage_scenario(CPUS_CHANGE_PASSWORD);
        assert!(!ignored);
        assert_eq!(provider.get_credential_count(), 0);
    }
}
