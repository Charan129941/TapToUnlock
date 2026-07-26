use super::{ChannelType, TransportChannel, TransportError};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

/// 128-bit Service UUID for OpenTapUnlock BLE GATT service: "6f70656e-7461-702d-756e-6c6f636b3031"
pub const OPENTAP_SERVICE_UUID: &str = "6f70656e-7461-702d-756e-6c6f636b3031";

/// 128-bit Characteristic UUID for receiving incoming signed unlock requests from mobile: "...3032"
pub const OPENTAP_CHAR_WRITE_UUID: &str = "6f70656e-7461-702d-756e-6c6f636b3032";

/// 128-bit Characteristic UUID for desktop sending nonces and unlock responses back to mobile: "...3033"
pub const OPENTAP_CHAR_NOTIFY_UUID: &str = "6f70656e-7461-702d-756e-6c6f636b3033";

/// Maximum acceptable distance threshold in RSSI dBm (e.g., -75 dBm is approx 3-5 meters).
pub const DEFAULT_RSSI_UNLOCK_THRESHOLD: i16 = -75;

/// Evaluates RSSI (Received Signal Strength Indicator) to prevent relay attacks from distant rooms.
pub struct ProximityFilter {
    min_rssi_dbm: i16,
}

impl ProximityFilter {
    pub fn new(min_rssi_dbm: i16) -> Self {
        Self { min_rssi_dbm }
    }

    /// Returns true if the device is physically within the safe proximity zone.
    pub fn is_in_range(&self, current_rssi: i16) -> bool {
        current_rssi >= self.min_rssi_dbm
    }

    /// Estimates approximate distance in meters using Free-Space Path Loss (FSPL) model.
    pub fn estimate_distance_meters(rssi: i16, tx_power_dbm: i16) -> f32 {
        if rssi == 0 {
            return -1.0; // Unknown
        }
        let ratio = (tx_power_dbm - rssi) as f32 / 20.0;
        10.0_f32.powf(ratio)
    }
}

impl Default for ProximityFilter {
    fn default() -> Self {
        Self::new(DEFAULT_RSSI_UNLOCK_THRESHOLD)
    }
}

/// BLE Transport Channel wrapper handling GATT client/server packet framing over BLE.
pub struct BleTransport {
    is_connected: Arc<AtomicBool>,
    latency_ms: Arc<AtomicU32>,
    incoming_buffer: Arc<AsyncMutex<Vec<u8>>>,
    outgoing_buffer: Arc<AsyncMutex<Vec<u8>>>,
}

impl BleTransport {
    pub fn new() -> Self {
        Self {
            is_connected: Arc::new(AtomicBool::new(false)),
            latency_ms: Arc::new(AtomicU32::new(35)), // Typical BLE GATT latency ~30-50ms
            incoming_buffer: Arc::new(AsyncMutex::new(Vec::new())),
            outgoing_buffer: Arc::new(AsyncMutex::new(Vec::new())),
        }
    }

    pub fn set_connected(&self, connected: bool) {
        self.is_connected.store(connected, Ordering::SeqCst);
    }

    pub fn set_latency(&self, latency: u32) {
        self.latency_ms.store(latency, Ordering::SeqCst);
    }

    /// Simulates injecting a received GATT characteristic write from the BLE hardware driver.
    pub async fn inject_incoming_packet(&self, data: &[u8]) {
        let mut buf = self.incoming_buffer.lock().await;
        buf.extend_from_slice(data);
    }

    /// Retrieves outgoing bytes waiting to be transmitted over GATT characteristic write/notify.
    pub async fn drain_outgoing_packet(&self) -> Vec<u8> {
        let mut buf = self.outgoing_buffer.lock().await;
        let drained = buf.clone();
        buf.clear();
        drained
    }

    pub fn parse_uuids() -> Result<(Uuid, Uuid, Uuid), TransportError> {
        let s_uuid = Uuid::parse_str(OPENTAP_SERVICE_UUID)
            .map_err(|_| TransportError::BleGattError("Invalid service UUID".to_string()))?;
        let w_uuid = Uuid::parse_str(OPENTAP_CHAR_WRITE_UUID)
            .map_err(|_| TransportError::BleGattError("Invalid write char UUID".to_string()))?;
        let n_uuid = Uuid::parse_str(OPENTAP_CHAR_NOTIFY_UUID)
            .map_err(|_| TransportError::BleGattError("Invalid notify char UUID".to_string()))?;
        Ok((s_uuid, w_uuid, n_uuid))
    }
}

impl Default for BleTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportChannel for BleTransport {
    async fn send_payload(&self, payload: &[u8]) -> Result<(), TransportError> {
        if !self.is_connected() {
            return Err(TransportError::ChannelUnavailable);
        }
        let mut out = self.outgoing_buffer.lock().await;
        out.extend_from_slice(payload);
        Ok(())
    }

    async fn receive_payload(&self) -> Result<Vec<u8>, TransportError> {
        if !self.is_connected() {
            return Err(TransportError::ChannelUnavailable);
        }
        let mut inc = self.incoming_buffer.lock().await;
        if inc.is_empty() {
            return Err(TransportError::ReceiveTimeout);
        }
        let data = inc.clone();
        inc.clear();
        Ok(data)
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::BleGatt
    }

    fn latency_ms(&self) -> u32 {
        self.latency_ms.load(Ordering::SeqCst)
    }

    fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_uuid_parsing_validity() {
        let uuids = BleTransport::parse_uuids();
        assert!(uuids.is_ok());
        let (s, w, n) = uuids.unwrap();
        assert_ne!(s, w);
        assert_ne!(w, n);
    }

    #[test]
    fn test_proximity_filter_rssi_evaluation() {
        let filter = ProximityFilter::new(-75);
        assert!(filter.is_in_range(-50)); // Strong signal, 1 meter away
        assert!(filter.is_in_range(-75)); // Exact boundary
        assert!(!filter.is_in_range(-85)); // Weak signal, another room or hallway!

        let dist = ProximityFilter::estimate_distance_meters(-60, -59);
        assert!(dist > 0.0 && dist < 5.0);
    }

    #[test]
    fn test_ble_transport_channel_trait() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let ble = BleTransport::new();
            assert_eq!(ble.channel_type(), ChannelType::BleGatt);
            assert!(!ble.is_connected());

            // Sending while disconnected must fail
            assert_eq!(
                ble.send_payload(b"test").await,
                Err(TransportError::ChannelUnavailable)
            );

            // Connect and exchange data
            ble.set_connected(true);
            assert!(ble.is_connected());

            ble.send_payload(b"signed_packet_data").await.unwrap();
            let outgoing = ble.drain_outgoing_packet().await;
            assert_eq!(outgoing, b"signed_packet_data");

            ble.inject_incoming_packet(b"challenge_response").await;
            let incoming = ble.receive_payload().await.unwrap();
            assert_eq!(incoming, b"challenge_response");
        });
    }
}
