use crate::guid::*;
use crate::pipe_client::{NamedPipeClient, DEFAULT_PIPE_NAME};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileState {
    /// Tile is idle on lock screen waiting for user selection
    Idle,
    /// Tile selected; waiting for user to perform Triple Tap on mobile device
    WaitingForTap,
    /// Biometric scan and Ed25519 signature verified! Ready to unlock Windows session
    Authenticated(String),
    /// Unlock denied or timed out
    Failed(String),
}

/// Represents a single user logon tile on the Windows Lock Screen or UAC elevation prompt.
pub struct OpenTapTile {
    pub username: String,
    pub state: Arc<Mutex<TileState>>,
    pub is_advised: Arc<AtomicBool>,
}

impl OpenTapTile {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_string(),
            state: Arc::new(Mutex::new(TileState::Idle)),
            is_advised: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns the UI text for a specific tile layout field index.
    pub fn get_string_value(&self, field_id: TileFieldId) -> String {
        let state = self.state.lock().unwrap();
        match field_id {
            TileFieldId::StatusText => match &*state {
                TileState::Idle => "OpenTap Biometric Mobile Unlock".to_string(),
                TileState::WaitingForTap => "Waiting for Triple Tap on phone...".to_string(),
                TileState::Authenticated(_) => "Biometric Verified! Unlocking...".to_string(),
                TileState::Failed(reason) => format!("Unlock Failed: {}", reason),
            },
            TileFieldId::ConnectionInfo => "Connected via mTLS / BLE GATT".to_string(),
            TileFieldId::SubmitButton => "Unlock with Phone".to_string(),
            TileFieldId::Logo => "".to_string(),
        }
    }

    /// Called by LogonUI when the user clicks or selects this tile on the lock screen.
    /// Spawns a background worker thread to listen for the mobile unlock event.
    pub fn advise_and_listen(&self) {
        self.is_advised.store(true, Ordering::SeqCst);
        {
            let mut state = self.state.lock().unwrap();
            *state = TileState::WaitingForTap;
        }

        let state_clone = self.state.clone();
        let is_advised = self.is_advised.clone();
        let username_clone = self.username.clone();

        thread::spawn(move || {
            // Communicate over Windows Named Pipe to opentapd background daemon
            match NamedPipeClient::request_biometric_unlock(DEFAULT_PIPE_NAME, &username_clone, 15_000) {
                Ok(resp) => {
                    if is_advised.load(Ordering::SeqCst) {
                        let mut st = state_clone.lock().unwrap();
                        let token = resp.token.unwrap_or_else(|| "default-win-auth-token".to_string());
                        *st = TileState::Authenticated(token);
                        // In COM execution, we would invoke pCPEvents->CredentialsChanged() here!
                    }
                }
                Err(e) => {
                    if is_advised.load(Ordering::SeqCst) {
                        let mut st = state_clone.lock().unwrap();
                        *st = TileState::Failed(e.to_string());
                    }
                }
            }
        });
    }

    /// Called by LogonUI when navigating away from the tile or when screen sleeps.
    pub fn unadvise(&self) {
        self.is_advised.store(false, Ordering::SeqCst);
        let mut state = self.state.lock().unwrap();
        *state = TileState::Idle;
    }

    /// Returns true if the tile is fully authenticated and ready to release logon credentials.
    pub fn is_ready_to_unlock(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        match &*state {
            TileState::Authenticated(token) => Some(token.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_opentap_tile_lifecycle() {
        let tile = OpenTapTile::new("chara");
        assert_eq!(tile.get_string_value(TileFieldId::StatusText), "OpenTap Biometric Mobile Unlock");
        assert!(tile.is_ready_to_unlock().is_none());

        tile.advise_and_listen();
        assert_eq!(tile.get_string_value(TileFieldId::StatusText), "Waiting for Triple Tap on phone...");

        // Wait brief moment for background pipe client simulation thread to finish
        thread::sleep(Duration::from_millis(200));
        assert_eq!(tile.get_string_value(TileFieldId::StatusText), "Biometric Verified! Unlocking...");
        assert!(tile.is_ready_to_unlock().is_some());

        tile.unadvise();
        assert_eq!(tile.get_string_value(TileFieldId::StatusText), "OpenTap Biometric Mobile Unlock");
    }
}
