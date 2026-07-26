use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to resolve system config directory")]
    DirResolutionFailed,
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Invalid hex encoded public key")]
    InvalidHex,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PairedDeviceInfo {
    pub device_uuid: String,
    pub device_name: String,
    pub public_key_hex: String,
    pub paired_at_utc: i64,
}

impl PairedDeviceInfo {
    pub fn public_key_bytes(&self) -> Result<[u8; 32], ConfigError> {
        let mut bytes = [0u8; 32];
        let hex_bytes = hex_decode(&self.public_key_hex)?;
        if hex_bytes.len() != 32 {
            return Err(ConfigError::InvalidHex);
        }
        bytes.copy_from_slice(&hex_bytes);
        Ok(bytes)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PairedDeviceStore {
    pub devices: HashMap<String, PairedDeviceInfo>,
}

impl PairedDeviceStore {
    /// Resolves the canonical config file path across Linux, macOS, and Windows.
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let proj_dirs = ProjectDirs::from("org", "opentapunlock", "opentap")
            .ok_or(ConfigError::DirResolutionFailed)?;
        let config_dir = proj_dirs.config_dir();
        create_dir_all(config_dir)?;
        Ok(config_dir.join("paired_devices.json"))
    }

    /// Loads the paired device vault from disk. Returns default empty store if file is missing.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let store: Self = serde_json::from_str(&contents)?;
        Ok(store)
    }

    /// Saves the paired device vault to disk with secure restrictive permissions.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(&path)?;
        file.write_all(json.as_bytes())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    pub fn add_device(&mut self, info: PairedDeviceInfo) {
        self.devices.insert(info.device_uuid.clone(), info);
    }

    pub fn remove_device(&mut self, uuid: &str) -> bool {
        self.devices.remove(uuid).is_some()
    }

    pub fn find_device(&self, uuid: &str) -> Option<&PairedDeviceInfo> {
        self.devices.get(uuid)
    }

    pub fn all_devices(&self) -> Vec<&PairedDeviceInfo> {
        self.devices.values().collect()
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ConfigError> {
    if s.len() % 2 != 0 {
        return Err(ConfigError::InvalidHex);
    }
    let mut bytes = Vec::new();
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ConfigError::InvalidHex)?;
        bytes.push(byte);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_decoding() {
        let hex = "6f70656e7461702d756e6c6f636b303132333435363738393061626364656667";
        assert_eq!(hex.len(), 64);
        let decoded = hex_decode(hex).unwrap();
        assert_eq!(decoded.len(), 32);
    }

    #[test]
    fn test_paired_device_store_operations() {
        let mut store = PairedDeviceStore::default();
        let info = PairedDeviceInfo {
            device_uuid: "pixel-8-pro-uuid".to_string(),
            device_name: "Chara's Pixel 8 Pro".to_string(),
            public_key_hex: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff".to_string(),
            paired_at_utc: 1700000000,
        };

        store.add_device(info.clone());
        assert!(store.find_device("pixel-8-pro-uuid").is_some());
        assert_eq!(store.all_devices().len(), 1);

        assert!(store.remove_device("pixel-8-pro-uuid"));
        assert!(store.find_device("pixel-8-pro-uuid").is_none());
    }
}
