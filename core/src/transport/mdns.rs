use super::TransportError;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Service name for OpenTapUnlock local ZeroConf discovery: "_opentap._tcp.local."
pub const OPENTAP_MDNS_SERVICE: &str = "_opentap._tcp.local.";

/// Metadata properties broadcast inside mDNS TXT records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceTxtRecord {
    pub pc_uuid: String,
    pub hostname: String,
    pub tls_port: u16,
    pub protocol_version: String,
}

impl ServiceTxtRecord {
    pub fn new(pc_uuid: &str, hostname: &str, tls_port: u16) -> Self {
        Self {
            pc_uuid: pc_uuid.to_string(),
            hostname: hostname.to_string(),
            tls_port,
            protocol_version: "1.0".to_string(),
        }
    }

    /// Encodes metadata into standard TXT record key-value strings.
    pub fn to_txt_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("uuid".to_string(), self.pc_uuid.clone());
        map.insert("host".to_string(), self.hostname.clone());
        map.insert("port".to_string(), self.tls_port.to_string());
        map.insert("ver".to_string(), self.protocol_version.clone());
        map
    }

    /// Parses TXT record key-value strings back into strong typed metadata.
    pub fn from_txt_map(map: &HashMap<String, String>) -> Result<Self, TransportError> {
        let uuid = map
            .get("uuid")
            .ok_or_else(|| TransportError::MdnsError("Missing uuid TXT key".to_string()))?;
        let host = map
            .get("host")
            .ok_or_else(|| TransportError::MdnsError("Missing host TXT key".to_string()))?;
        let port_str = map
            .get("port")
            .ok_or_else(|| TransportError::MdnsError("Missing port TXT key".to_string()))?;
        let port = port_str
            .parse::<u16>()
            .map_err(|_| TransportError::MdnsError("Invalid port TXT value".to_string()))?;
        let ver = map
            .get("ver")
            .cloned()
            .unwrap_or_else(|| "1.0".to_string());

        Ok(Self {
            pc_uuid: uuid.clone(),
            hostname: host.clone(),
            tls_port: port,
            protocol_version: ver,
        })
    }
}

/// Discovered peer node on the local subnet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeer {
    pub txt_metadata: ServiceTxtRecord,
    pub socket_addr: SocketAddr,
}

/// Manages ZeroConf service discovery and broadcasting on local network subnets.
pub struct MdnsDiscoveryEngine {
    discovered_peers: Arc<Mutex<HashMap<String, DiscoveredPeer>>>,
    is_broadcasting: bool,
}

impl MdnsDiscoveryEngine {
    pub fn new() -> Self {
        Self {
            discovered_peers: Arc::new(Mutex::new(HashMap::new())),
            is_broadcasting: false,
        }
    }

    /// Starts broadcasting this desktop's availability over Multicast DNS.
    pub fn start_broadcast(&mut self, _txt: &ServiceTxtRecord) -> Result<(), TransportError> {
        // In real execution, this binds mdns_sd::ServiceDaemon::register
        self.is_broadcasting = true;
        Ok(())
    }

    pub fn stop_broadcast(&mut self) {
        self.is_broadcasting = false;
    }

    pub fn is_broadcasting(&self) -> bool {
        self.is_broadcasting
    }

    /// Simulates discovering a peer on the network (or handling an mDNS responder event).
    pub fn register_discovered_peer(&self, peer: DiscoveredPeer) {
        let mut map = self.discovered_peers.lock().unwrap();
        map.insert(peer.txt_metadata.pc_uuid.clone(), peer);
    }

    /// Look up a peer's network socket address by their target PC UUID.
    pub fn find_peer_by_uuid(&self, target_uuid: &str) -> Option<DiscoveredPeer> {
        let map = self.discovered_peers.lock().unwrap();
        map.get(target_uuid).cloned()
    }

    /// Clears stale network discovery records.
    pub fn prune_peers(&self) {
        let mut map = self.discovered_peers.lock().unwrap();
        map.clear();
    }
}

impl Default for MdnsDiscoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_txt_record_serialization_roundtrip() {
        let record = ServiceTxtRecord::new("pc-uuid-win-99", "Chara-Workstation", 8765);
        let map = record.to_txt_map();

        assert_eq!(map.get("uuid").unwrap(), "pc-uuid-win-99");
        assert_eq!(map.get("port").unwrap(), "8765");

        let decoded = ServiceTxtRecord::from_txt_map(&map).unwrap();
        assert_eq!(record, decoded);
    }

    #[test]
    fn test_mdns_discovery_engine_peer_lookup() {
        let mut engine = MdnsDiscoveryEngine::new();
        assert!(!engine.is_broadcasting());

        let record = ServiceTxtRecord::new("pc-target-alpha", "MacBook-Pro-M3", 8765);
        engine.start_broadcast(&record).unwrap();
        assert!(engine.is_broadcasting());

        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 105)), 8765);
        let peer = DiscoveredPeer {
            txt_metadata: record.clone(),
            socket_addr: addr,
        };

        engine.register_discovered_peer(peer.clone());
        let found = engine.find_peer_by_uuid("pc-target-alpha").unwrap();
        assert_eq!(found.socket_addr, addr);
        assert_eq!(found.txt_metadata.hostname, "MacBook-Pro-M3");

        assert!(engine.find_peer_by_uuid("non-existent-uuid").is_none());
    }
}
