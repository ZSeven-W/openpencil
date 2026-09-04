//! Reserved-address screening for provider-endpoint dials — moved verbatim
//! from `op-host-services/src/web_credentials.rs` (which re-exports
//! [`is_restricted_ip`]) so the connect-time dial guard
//! (`crate::provider_dial`) links on mobile hosts too.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn is_restricted_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_restricted_ipv4(ip),
        IpAddr::V6(ip) => is_restricted_ipv6(ip),
    }
}

pub fn is_restricted_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, d] = ip.octets();
    a == 0
        || a == 10
        || (a == 100 && (64..=127).contains(&b))
        || a == 127
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 168)
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224
        || [a, b, c, d] == [168, 63, 129, 16]
}

pub fn is_restricted_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    if segments[..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
        let [a, b] = segments[6].to_be_bytes();
        let [c, d] = segments[7].to_be_bytes();
        return is_restricted_ipv4(Ipv4Addr::new(a, b, c, d));
    }
    (segments[0] & 0xe000) != 0x2000
        || segments[0] == 0x2002
        || segments[0] == 0x3fff
        || (segments[0] == 0x2001
            && (matches!(segments[1], 0 | 2 | 0x0db8)
                || (segments[1] & 0xfff0) == 0x0010
                || (segments[1] & 0xfff0) == 0x0020))
}
