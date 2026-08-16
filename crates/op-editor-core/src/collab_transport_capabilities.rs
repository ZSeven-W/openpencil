//! Host transport capabilities projected into collaboration chrome.

use crate::CollabUiAction;

/// Collaboration paths the embedding platform can support safely.
///
/// Manual address/invite joins are intentionally independent from these
/// switches: mobile can connect to a known LAN endpoint without advertising
/// or browsing through multicast DNS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollabTransportCapabilities {
    pub lan_hosting: bool,
    pub nearby_discovery: bool,
}

impl CollabTransportCapabilities {
    pub const ALL: Self = Self {
        lan_hosting: true,
        nearby_discovery: true,
    };

    pub const RELAY_AND_MANUAL_JOIN: Self = Self {
        lan_hosting: false,
        nearby_discovery: false,
    };

    pub fn supports(self, action: &CollabUiAction) -> bool {
        match action {
            CollabUiAction::StartLan => self.lan_hosting,
            CollabUiAction::BeginDiscovery | CollabUiAction::JoinDiscovered { .. } => {
                self.nearby_discovery
            }
            _ => true,
        }
    }
}

impl Default for CollabTransportCapabilities {
    fn default() -> Self {
        Self::ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_only_keeps_manual_join_but_rejects_multicast_paths() {
        let capabilities = CollabTransportCapabilities::RELAY_AND_MANUAL_JOIN;
        assert!(capabilities.supports(&CollabUiAction::Start));
        assert!(capabilities.supports(&CollabUiAction::JoinAddress {
            endpoint: "192.168.1.8:43120".into(),
        }));
        assert!(!capabilities.supports(&CollabUiAction::StartLan));
        assert!(!capabilities.supports(&CollabUiAction::BeginDiscovery));
        assert!(!capabilities.supports(&CollabUiAction::JoinDiscovered {
            discovery_id: "local".into(),
        }));
    }
}
