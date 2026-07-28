use std::net::{IpAddr, Ipv4Addr};
use std::ptr;
use std::sync::mpsc;

use super::super::bonjour_txt::{decode_txt_record, property_from_pairs};
use super::*;

fn service_key() -> ServiceKey {
    ServiceKey {
        name: "openpencil-aabbccddeeff".to_owned(),
        registration_type: "_openpencil-collab._tcp.".to_owned(),
        domain: "local.".to_owned(),
    }
}

fn metadata() -> ResolvedMetadata {
    ResolvedMetadata {
        fullname: "openpencil-aabbccddeeff._openpencil-collab._tcp.local.".to_owned(),
        hostname: "op-aabbccddeeff.local.".to_owned(),
        port: 45_123,
        txt: vec![
            35, b'i', b'd', b'=', b'0', b'0', b'1', b'1', b'2', b'2', b'3', b'3', b'4', b'4', b'5',
            b'5', b'6', b'6', b'7', b'7', b'8', b'8', b'9', b'9', b'a', b'a', b'b', b'b', b'c',
            b'c', b'd', b'd', b'e', b'e', b'f', b'f', 3, b'v', b'=', b'1', 7, b'p', b'=', b'4',
            b'5', b'1', b'2', b'3',
        ],
    }
}

fn browser_state(now: Instant) -> (BrowserState, ServiceKey) {
    let service = service_key();
    let key = InterfaceKey {
        service: service.clone(),
        interface_index: 4,
    };
    let mut interface = InterfaceState::new(now);
    interface.metadata = Some(metadata());
    interface
        .addresses
        .insert(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 20)), 120);
    interface.retry_at = None;
    let mut state = BrowserState::new(ptr::null_mut());
    state.next_lease_refresh_at = now + LEASE_REFRESH_INTERVAL;
    state.interfaces.insert(key, interface);
    (state, service)
}

#[test]
fn txt_decoder_requires_exact_unique_allowlisted_pairs() {
    let metadata = metadata();
    let properties = decode_txt_record(&metadata.txt).unwrap();
    assert_eq!(
        property_from_pairs(&properties, "id").unwrap(),
        "00112233445566778899aabbccddeeff"
    );
    assert_eq!(property_from_pairs(&properties, "v").unwrap(), "1");
    assert_eq!(property_from_pairs(&properties, "p").unwrap(), "45123");

    for malformed in [
        vec![],
        vec![0],
        vec![3, b'v', b'=', b'1', 7, b'p', b'=', b'4'],
        vec![
            3, b'v', b'=', b'1', 3, b'v', b'=', b'1', 3, b'p', b'=', b'1',
        ],
    ] {
        assert!(decode_txt_record(&malformed)
            .and_then(|pairs| property_from_pairs(&pairs, "id").map(str::to_owned))
            .is_err());
    }
}

#[test]
fn aggregate_is_sorted_uses_bounded_lease_and_removes_last_result() {
    let service = service_key();
    let metadata = metadata();
    let first_key = InterfaceKey {
        service: service.clone(),
        interface_index: 4,
    };
    let second_key = InterfaceKey {
        service: service.clone(),
        interface_index: 7,
    };
    let mut first = InterfaceState::new(Instant::now());
    first.metadata = Some(metadata.clone());
    first
        .addresses
        .insert(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 20)), 120);
    let mut second = InterfaceState::new(Instant::now());
    second.metadata = Some(metadata);
    second
        .addresses
        .insert(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)), 60);

    let mut state = BrowserState::new(ptr::null_mut());
    state.interfaces.insert(first_key, first);
    state.interfaces.insert(second_key, second);
    let (sender, receiver) = mpsc::sync_channel(4);
    assert!(state.emit_service(&service, &sender));
    let BonjourEvent::Resolved(resolved) = receiver.try_recv().unwrap() else {
        panic!("expected a resolved event");
    };
    assert_eq!(resolved.ttl_seconds, CACHE_LEASE_SECONDS);
    assert_eq!(
        resolved.addresses,
        [
            IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)),
            IpAddr::V4(Ipv4Addr::new(192, 0, 2, 20))
        ]
    );

    state.interfaces.clear();
    assert!(state.emit_service(&service, &sender));
    assert!(matches!(
        receiver.try_recv().unwrap(),
        BonjourEvent::Removed { fullname } if fullname == resolved.fullname
    ));
}

#[test]
fn resolved_metadata_rejects_nonlocal_or_unbounded_records() {
    let valid = metadata();
    assert!(valid_resolved_metadata(&valid));

    let mut nonlocal = valid.clone();
    nonlocal.hostname = "op-aabbccddeeff.example.com.".to_owned();
    assert!(!valid_resolved_metadata(&nonlocal));

    let mut oversized = valid;
    oversized.txt = vec![0; MAX_TXT_BYTES + 1];
    assert!(!valid_resolved_metadata(&oversized));
}

#[test]
fn long_lived_query_renews_the_cache_lease_and_removal_clears_it() {
    let now = Instant::now();
    let lease = Duration::from_secs(u64::from(CACHE_LEASE_SECONDS));
    let refresh_at = now + LEASE_REFRESH_INTERVAL;
    let (mut state, service) = browser_state(now);
    let (sender, receiver) = mpsc::sync_channel(4);
    let mut cache = super::super::DiscoveryCache::default();

    assert!(state.emit_service(&service, &sender));
    cache.process_bonjour_event(receiver.recv().unwrap(), now);
    assert_eq!(cache.sessions(now)[0].expires_at(), now + lease);

    assert!(state.refresh_leases(refresh_at, &sender));
    cache.process_bonjour_event(receiver.recv().unwrap(), refresh_at);
    assert_eq!(
        cache.sessions(now + lease + Duration::from_millis(1)).len(),
        1,
        "a live DNSServiceGetAddrInfo query must outlive its first DNS TTL"
    );

    state.interfaces.clear();
    assert!(state.emit_service(&service, &sender));
    cache.process_bonjour_event(receiver.recv().unwrap(), refresh_at);
    assert!(cache.sessions(refresh_at).is_empty());
}

#[test]
fn full_event_lane_drops_a_burst_but_heartbeat_recovers() {
    let now = Instant::now();
    let refresh_at = now + LEASE_REFRESH_INTERVAL;
    let (mut state, service) = browser_state(now);
    let (sender, receiver) = mpsc::sync_channel(1);
    sender
        .try_send(BonjourEvent::Removed {
            fullname: "occupied._openpencil-collab._tcp.local.".to_owned(),
        })
        .unwrap();

    for _ in 0..MAX_QUEUED_EVENTS + 32 {
        assert!(
            state.emit_service(&service, &sender),
            "a full bounded lane must not terminate the Bonjour worker"
        );
    }
    assert!(matches!(
        receiver.recv().unwrap(),
        BonjourEvent::Removed { .. }
    ));

    assert!(state.refresh_leases(refresh_at, &sender));
    assert!(matches!(
        receiver.recv().unwrap(),
        BonjourEvent::Resolved(BonjourResolved { ttl_seconds, .. })
            if ttl_seconds == CACHE_LEASE_SECONDS
    ));

    state.interfaces.clear();
    assert!(state.emit_service(&service, &sender));
    assert!(matches!(
        receiver.recv().unwrap(),
        BonjourEvent::Removed { .. }
    ));
}
