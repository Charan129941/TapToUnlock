pub mod ble;
pub mod mdns;
pub mod routing;
pub mod tls;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Transmission error: {0}")]
    TransmissionFailed(String),
    #[error("Receive timeout or channel closed")]
    ReceiveTimeout,
    #[error("TLS Certificate verification or pinning failed: {0}")]
    TlsSecurityError(String),
    #[error("BLE GATT operation error: {0}")]
    BleGattError(String),
    #[error("mDNS Discovery error: {0}")]
    MdnsError(String),
    #[error("Channel unavailable or offline")]
    ChannelUnavailable,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelType {
    /// High-speed local LAN/Wi-Fi TCP connection encrypted with TLS 1.3
    TlsLocal,
    /// Point-to-Point Wi-Fi Direct or AWDL socket
    WifiDirect,
    /// Bluetooth Low Energy GATT Server/Client connection
    BleGatt,
    /// Wake-on-LAN UDP Magic Packet broadcast
    WakeOnLan,
    /// Direct USB wired tether connection
    UsbTether,
}

impl ChannelType {
    /// Returns the default security weight factor (1.0 = maximum cryptographic hardware backing)
    pub fn security_weight(&self) -> f32 {
        match self {
            ChannelType::TlsLocal => 1.0,
            ChannelType::WifiDirect => 0.95,
            ChannelType::BleGatt => 0.90,
            ChannelType::UsbTether => 1.0,
            ChannelType::WakeOnLan => 0.50, // WoL is unencrypted trigger only
        }
    }

    /// Returns the nominal bandwidth factor for link scoring
    pub fn bandwidth_factor(&self) -> f32 {
        match self {
            ChannelType::TlsLocal => 1.0,
            ChannelType::WifiDirect => 0.9,
            ChannelType::UsbTether => 1.0,
            ChannelType::BleGatt => 0.3,    // BLE has lower throughput (~1-2 Mbps max)
            ChannelType::WakeOnLan => 0.1,  // Trigger packet only
        }
    }
}

/// Core async trait implemented by all transport mechanisms (TLS, BLE, Wi-Fi Direct).
#[async_trait]
pub trait TransportChannel: Send + Sync {
    /// Transmits a raw byte payload (typically Postcard-encoded `SignedUnlockPayload`).
    async fn send_payload(&self, payload: &[u8]) -> Result<(), TransportError>;

    /// Waits for and receives an incoming byte payload from the peer.
    async fn receive_payload(&self) -> Result<Vec<u8>, TransportError>;

    /// Returns the type of transport channel.
    fn channel_type(&self) -> ChannelType;

    /// Returns current measured round-trip latency in milliseconds.
    fn latency_ms(&self) -> u32;

    /// Returns whether the channel is currently connected and active.
    fn is_connected(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_factors() {
        assert_eq!(ChannelType::TlsLocal.security_weight(), 1.0);
        assert!(ChannelType::BleGatt.bandwidth_factor() < ChannelType::TlsLocal.bandwidth_factor());
    }
}
