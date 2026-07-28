use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

/// Selects one owner-only address for the manual-join fallback.
///
/// A UDP connect performs route selection without transmitting a packet. It
/// avoids preferring a Docker/VPN interface merely because its address sorts
/// first. Interface enumeration remains the fallback for isolated LANs with
/// no default route.
pub(super) fn select_share_endpoint(listener: SocketAddr, dual_stack: bool) -> Option<SocketAddr> {
    if !listener.ip().is_unspecified() {
        return is_shareable_address(listener.ip()).then_some(listener);
    }
    let preferred = preferred_route_address(listener, dual_stack);
    let addresses = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .map(|interface| interface.ip());
    select_from_addresses(listener, dual_stack, preferred, addresses)
}

fn preferred_route_address(listener: SocketAddr, dual_stack: bool) -> Option<IpAddr> {
    if listener.is_ipv4() || dual_stack {
        route_address(false).or_else(|| {
            if listener.is_ipv6() {
                route_address(true)
            } else {
                None
            }
        })
    } else {
        route_address(true)
    }
}

fn route_address(ipv6: bool) -> Option<IpAddr> {
    let (bind, probe) = if ipv6 {
        (
            SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
            SocketAddr::new(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1)),
                9,
            ),
        )
    } else {
        (
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 9),
        )
    };
    let socket = UdpSocket::bind(bind).ok()?;
    socket.connect(probe).ok()?;
    let address = socket.local_addr().ok()?.ip();
    is_shareable_address(address).then_some(address)
}

fn select_from_addresses(
    listener: SocketAddr,
    dual_stack: bool,
    preferred: Option<IpAddr>,
    addresses: impl IntoIterator<Item = IpAddr>,
) -> Option<SocketAddr> {
    if !listener.ip().is_unspecified() {
        return is_shareable_address(listener.ip()).then_some(listener);
    }
    let accepts = |address: IpAddr| {
        is_shareable_address(address)
            && ((listener.is_ipv4() && address.is_ipv4())
                || (listener.is_ipv6() && (address.is_ipv6() || dual_stack && address.is_ipv4())))
    };
    if let Some(preferred) = preferred.filter(|address| accepts(*address)) {
        return Some(SocketAddr::new(preferred, listener.port()));
    }
    let mut addresses = addresses
        .into_iter()
        .filter(|address| accepts(*address))
        .collect::<Vec<_>>();
    addresses.sort_unstable_by_key(|address| share_address_rank(*address));
    addresses.dedup();
    addresses
        .first()
        .copied()
        .map(|address| SocketAddr::new(address, listener.port()))
}

fn is_shareable_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                && !address.is_link_local()
                && !address.is_broadcast()
        }
        IpAddr::V6(address) => {
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                && !address.is_unicast_link_local()
        }
    }
}

fn share_address_rank(address: IpAddr) -> (u8, IpAddr) {
    let rank = match address {
        IpAddr::V4(address) if address.is_private() => 0,
        IpAddr::V4(_) => 1,
        IpAddr::V6(address) if address.is_unique_local() => 2,
        IpAddr::V6(_) => 3,
    };
    (rank, address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_listener_is_returned_only_when_shareable() {
        let lan = "192.168.1.20:43120".parse().unwrap();
        assert_eq!(select_from_addresses(lan, false, None, []), Some(lan));
        assert_eq!(
            select_from_addresses("127.0.0.1:43120".parse().unwrap(), false, None, []),
            None
        );
    }

    #[test]
    fn dual_stack_prefers_the_route_selected_ipv4_address() {
        let listener = "[::]:43120".parse().unwrap();
        let preferred = "192.168.50.4".parse().unwrap();
        let addresses = ["10.0.0.2".parse().unwrap(), "2001:db8::20".parse().unwrap()];
        assert_eq!(
            select_from_addresses(listener, true, Some(preferred), addresses),
            Some("192.168.50.4:43120".parse().unwrap())
        );
    }

    #[test]
    fn fallback_is_bounded_to_the_listener_family_and_usable_lan_addresses() {
        let listener = "0.0.0.0:43120".parse().unwrap();
        let addresses = [
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            "169.254.1.20".parse().unwrap(),
            "2001:db8::20".parse().unwrap(),
            "203.0.113.20".parse().unwrap(),
            "192.168.1.20".parse().unwrap(),
        ];
        assert_eq!(
            select_from_addresses(listener, false, None, addresses),
            Some("192.168.1.20:43120".parse().unwrap())
        );
    }
}
