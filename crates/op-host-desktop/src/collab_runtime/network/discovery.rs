use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use op_collab_transport::{DiscoveryBrowser, MAX_DISCOVERY_ADDRESSES, TRANSPORT_PROTOCOL_VERSION};

use super::super::types::{
    CollabRuntimeFailure, DiscoveredEndpoint, NetworkEvent, TerminalNetworkEvent,
};
use super::{EventSendError, EventSink};

const DISCOVERY_WAIT: Duration = Duration::from_millis(500);

pub(super) fn run(sink: EventSink, stop: Receiver<()>) {
    let mut browser = match DiscoveryBrowser::start() {
        Ok(browser) => browser,
        Err(_) => {
            let _ = sink.send_terminal(TerminalNetworkEvent::Failed(
                CollabRuntimeFailure::Transport,
            ));
            return;
        }
    };
    let mut previous = Vec::new();
    loop {
        match stop.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => return,
            Err(TryRecvError::Empty) => {}
        }
        let next = browser
            .wait(DISCOVERY_WAIT)
            .into_iter()
            .filter_map(|session| {
                let addresses = bounded_addresses(session.addresses());
                (!addresses.is_empty()).then(|| DiscoveredEndpoint {
                    discovery_id: session.discovery_id().to_owned(),
                    addresses,
                    compatible: session.protocol_version() == TRANSPORT_PROTOCOL_VERSION,
                })
            })
            .collect::<Vec<_>>();
        if next != previous {
            previous.clone_from(&next);
            match sink.try_send(NetworkEvent::Discovery { sessions: next }) {
                Ok(()) => {}
                Err(EventSendError::Full) => {
                    let _ = sink.send_terminal(TerminalNetworkEvent::Failed(
                        CollabRuntimeFailure::ResourceLimit,
                    ));
                    return;
                }
                Err(EventSendError::Disconnected) => return,
            }
        }
    }
}

fn bounded_addresses(addresses: &[std::net::SocketAddr]) -> Vec<std::net::SocketAddr> {
    addresses
        .iter()
        .copied()
        .take(MAX_DISCOVERY_ADDRESSES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn desktop_projection_retains_all_bounded_transport_addresses() {
        let addresses = (1..=MAX_DISCOVERY_ADDRESSES + 1)
            .map(|port| {
                SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::LOCALHOST),
                    u16::try_from(port).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let projected = bounded_addresses(&addresses);
        assert_eq!(projected.len(), MAX_DISCOVERY_ADDRESSES);
        assert_eq!(projected, addresses[..MAX_DISCOVERY_ADDRESSES]);
    }
}
