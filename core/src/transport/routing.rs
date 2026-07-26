use super::{ChannelType, TransportChannel, TransportError};
use std::sync::Arc;

/// Evaluates real-time transport channel quality and computes an optimal link score.
pub struct LinkScorer;

impl LinkScorer {
    /// Computes a composite link score based on latency, bandwidth, and security weighting.
    /// Higher score = preferred channel.
    pub fn score(channel: &dyn TransportChannel) -> f32 {
        if !channel.is_connected() {
            return 0.0;
        }

        let latency_ms = channel.latency_ms() as f32;
        // Cap latency deduction at 990ms so score doesn't become negative for valid links
        let latency_factor = (1000.0 - latency_ms.min(990.0)).max(10.0);
        
        let channel_type = channel.channel_type();
        let bandwidth_factor = channel_type.bandwidth_factor();
        let security_weight = channel_type.security_weight();

        latency_factor * bandwidth_factor * security_weight
    }
}

#[derive(Debug, Clone)]
pub struct RoutingEngineConfig {
    pub enable_ble: bool,
    pub enable_mdns: bool,
    pub enable_wifi: bool,
}

impl Default for RoutingEngineConfig {
    fn default() -> Self {
        Self {
            enable_ble: true,
            enable_mdns: true,
            enable_wifi: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingMethod {
    WifiTls,
    BleGatt,
}

/// Coordinates multi-modal communication, automatically selecting the fastest secure channel.
pub struct RoutingEngine {
    channels: Vec<Arc<dyn TransportChannel>>,
    config: RoutingEngineConfig,
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingEngine {
    pub fn new() -> Self {
        Self::from_config(RoutingEngineConfig::default())
    }

    pub fn from_config(config: RoutingEngineConfig) -> Self {
        Self {
            channels: Vec::new(),
            config,
        }
    }

    pub fn is_enabled(&self, method: &RoutingMethod) -> bool {
        match method {
            RoutingMethod::WifiTls => self.config.enable_wifi,
            RoutingMethod::BleGatt => self.config.enable_ble,
        }
    }

    pub fn get_priority(&self, method: &RoutingMethod) -> u32 {
        match method {
            RoutingMethod::WifiTls => 100,
            RoutingMethod::BleGatt => 50,
        }
    }

    /// Registers a new active transport channel with the routing engine.
    pub fn add_channel(&mut self, channel: Arc<dyn TransportChannel>) {
        self.channels.push(channel);
    }

    /// Returns all currently registered channels.
    pub fn channels(&self) -> &[Arc<dyn TransportChannel>] {
        &self.channels
    }

    /// Evaluates all registered channels and returns the highest-scoring connected channel.
    pub fn select_best_channel(&self) -> Option<Arc<dyn TransportChannel>> {
        self.channels
            .iter()
            .filter(|c| c.is_connected())
            .max_by(|a, b| {
                let score_a = LinkScorer::score(a.as_ref());
                let score_b = LinkScorer::score(b.as_ref());
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Transmits a payload via the best available channel, automatically falling back if needed.
    pub async fn send_via_best_channel(&self, payload: &[u8]) -> Result<ChannelType, TransportError> {
        // Sort connected channels by score descending
        let mut connected_channels: Vec<_> = self
            .channels
            .iter()
            .filter(|c| c.is_connected())
            .cloned()
            .collect();

        if connected_channels.is_empty() {
            return Err(TransportError::ChannelUnavailable);
        }

        connected_channels.sort_by(|a, b| {
            let score_a = LinkScorer::score(a.as_ref());
            let score_b = LinkScorer::score(b.as_ref());
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        for channel in connected_channels {
            if channel.send_payload(payload).await.is_ok() {
                return Ok(channel.channel_type());
            }
        }

        Err(TransportError::TransmissionFailed("All fallback channels failed to send payload".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::ble::BleTransport;
    use crate::transport::tls::TlsTransport;
    use tokio::runtime::Runtime;

    #[test]
    fn test_link_scorer_prefers_tls_over_ble() {
        let tls = TlsTransport::new();
        tls.set_connected(true);
        tls.set_latency(5);

        let ble = BleTransport::new();
        ble.set_connected(true);
        ble.set_latency(35);

        let score_tls = LinkScorer::score(&tls);
        let score_ble = LinkScorer::score(&ble);

        assert!(score_tls > score_ble, "TLS score ({}) should exceed BLE score ({})", score_tls, score_ble);
    }

    #[test]
    fn test_routing_engine_auto_selection() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let mut engine = RoutingEngine::new();
            let tls = Arc::new(TlsTransport::new());
            let ble = Arc::new(BleTransport::new());

            engine.add_channel(tls.clone());
            engine.add_channel(ble.clone());

            // Both disconnected -> no channel selected
            assert!(engine.select_best_channel().is_none());

            // Connect BLE only -> selects BLE
            ble.set_connected(true);
            let best = engine.select_best_channel().unwrap();
            assert_eq!(best.channel_type(), ChannelType::BleGatt);

            // Connect TLS -> selects TLS over BLE due to higher score
            tls.set_connected(true);
            let best = engine.select_best_channel().unwrap();
            assert_eq!(best.channel_type(), ChannelType::TlsLocal);

            // Test transmission via best channel
            let chosen_type = engine.send_via_best_channel(b"zero_trust_payload").await.unwrap();
            assert_eq!(chosen_type, ChannelType::TlsLocal);

            let sent_bytes = tls.drain_outgoing_bytes().await;
            assert_eq!(sent_bytes, b"zero_trust_payload");
        });
    }
}
