use super::*;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, Mutex};

const ID: &str = "00112233445566778899aabbccddeeff";

fn service(
    instance: &str,
    properties: &[(&str, &str)],
    addresses: &[IpAddr],
    port: u16,
) -> ServiceInfo {
    ServiceInfo::new(
        DISCOVERY_SERVICE_TYPE,
        instance,
        "op-aabbccddeeff.local.",
        addresses,
        port,
        properties,
    )
    .unwrap()
}

fn valid_properties() -> [(&'static str, &'static str); 3] {
    [("id", ID), ("v", "1"), ("p", "45123")]
}

struct FakeBackend {
    events: Arc<Mutex<Vec<String>>>,
    fail_register_call: Option<usize>,
    register_calls: usize,
    fail_unregister: bool,
}

impl FakeBackend {
    fn push(&self, event: String) {
        self.events.lock().unwrap().push(event);
    }
}

impl PublisherBackend for FakeBackend {
    fn register(&mut self, service: ServiceInfo) -> Result<(), DiscoveryError> {
        self.register_calls += 1;
        if self.fail_register_call == Some(self.register_calls) {
            return Err(DiscoveryError::InvalidMetadata);
        }
        let id = service.get_property_val_str("id").unwrap_or_default();
        self.push(format!("register:{}:{id}", service.get_fullname()));
        Ok(())
    }

    fn unregister(&mut self, service: &ServiceInfo) -> Result<(), DiscoveryError> {
        if self.fail_unregister {
            return Err(DiscoveryError::UnregisterFailed);
        }
        self.push(format!("unregister:{}", service.get_fullname()));
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), DiscoveryError> {
        self.push("shutdown".to_owned());
        Ok(())
    }
}

#[test]
fn publisher_metadata_has_only_the_allowlisted_fields() {
    let addresses = [IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10))];
    let info = build_service_info(
        ID,
        45123,
        &addresses,
        "openpencil-aabbccddeeff",
        "op-ffeeddccbbaa.local.",
    )
    .unwrap();

    let mut keys: Vec<_> = info
        .get_properties()
        .iter()
        .map(|property| property.key())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["id", "p", "v"]);
    assert_eq!(info.get_property_val_str("id"), Some(ID));
    assert!(!info.get_hostname().contains(ID));
}

#[test]
fn listener_family_filter_matches_socket_reachability() {
    let v4: SocketAddr = "0.0.0.0:45123".parse().unwrap();
    let v6: SocketAddr = "[::]:45123".parse().unwrap();
    assert!(matches!(
        disabled_listener_family(v4, false),
        Some(IfKind::IPv6)
    ));
    assert!(matches!(
        disabled_listener_family(v6, false),
        Some(IfKind::IPv4)
    ));
    assert!(disabled_listener_family(v6, true).is_none());
}

#[test]
fn parser_returns_sorted_socket_addresses_and_ttl() {
    let addresses = [
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, 20)),
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)),
    ];
    let info = service("openpencil-a", &valid_properties(), &addresses, 45123);
    let now = Instant::now();
    let parsed = parse_service_info(&info, now).unwrap();

    assert_eq!(parsed.discovery_id(), ID);
    assert_eq!(parsed.protocol_version(), TRANSPORT_PROTOCOL_VERSION);
    assert_eq!(parsed.addresses().len(), 3);
    assert_eq!(
        parsed.primary_address(),
        Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)),
            45123
        ))
    );
    assert_eq!(
        parsed.expires_at(),
        now + Duration::from_secs(u64::from(info.get_host_ttl()))
    );
}

#[test]
fn parser_handles_versions_limits_and_unusable_addresses() {
    let address = [IpAddr::V4(Ipv4Addr::LOCALHOST)];
    for properties in [
        [("id", ID), ("v", "01"), ("p", "45123")],
        [("id", ID), ("v", "1"), ("p", "45124")],
        [("id", "not-a-random-id"), ("v", "1"), ("p", "45123")],
    ] {
        let info = service("openpencil-invalid", &properties, &address, 45123);
        assert!(parse_service_info(&info, Instant::now()).is_err());
    }
    let extra = [("id", ID), ("v", "1"), ("p", "45123"), ("name", "private")];
    let extra = service("extra", &extra, &address, 45123);
    assert!(parse_service_info(&extra, Instant::now()).is_err());
    let future = [("id", ID), ("v", "2"), ("p", "45123")];
    let future = service("future", &future, &address, 45123);
    assert_eq!(
        parse_service_info(&future, Instant::now())
            .unwrap()
            .protocol_version(),
        2
    );

    let unusable = [
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V4(Ipv4Addr::new(224, 0, 0, 251)),
        IpAddr::V6("fe80::1".parse().unwrap()),
    ];
    let info = service("openpencil-unusable", &valid_properties(), &unusable, 45123);
    assert!(parse_service_info(&info, Instant::now()).is_err());
    assert!(matches!(
        build_service_info(ID, 45123, &unusable, "instance", "host.local."),
        Err(DiscoveryError::InvalidAddress)
    ));
    assert_eq!(
        renamed_instance("instance (2)._openpencil-collab._tcp.local.").unwrap(),
        "instance (2)"
    );
    assert!(is_tracked_rename(
        "instance._openpencil-collab._tcp.local.",
        "INSTANCE._openpencil-collab._tcp.local.",
        "instance (2)._openpencil-collab._tcp.local."
    ));

    let many: Vec<_> = (1..=MAX_DISCOVERY_ADDRESSES + 1)
        .map(|last| IpAddr::V4(Ipv4Addr::new(198, 51, 100, last as u8)))
        .collect();
    assert!(validate_publish_addresses(&many[..MAX_DISCOVERY_ADDRESSES]).is_ok());
    let exact = service(
        "exact",
        &valid_properties(),
        &many[..MAX_DISCOVERY_ADDRESSES],
        45123,
    );
    assert_eq!(
        parse_service_info(&exact, Instant::now())
            .unwrap()
            .addresses()
            .len(),
        MAX_DISCOVERY_ADDRESSES
    );
    assert!(matches!(
        build_service_info(ID, 45123, &many, "instance", "host.local."),
        Err(DiscoveryError::InvalidAddress)
    ));
    let info = service("too-many", &valid_properties(), &many, 45123);
    assert!(parse_service_info(&info, Instant::now()).is_err());
}

#[test]
fn cache_is_bounded_expires_and_honors_removal() {
    let now = Instant::now();
    let mut cache = DiscoveryCache::default();
    for index in 0..(MAX_DISCOVERED_SESSIONS + 8) {
        let session = DiscoveredSession {
            discovery_id: format!("{index:032x}"),
            protocol_version: TRANSPORT_PROTOCOL_VERSION,
            addresses: vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 45123)],
            expires_at: now + Duration::from_secs(60),
        };
        cache.insert(format!("service-{index}"), session, now);
    }
    assert_eq!(cache.sessions(now).len(), MAX_DISCOVERED_SESSIONS);

    cache.process_event(
        ServiceEvent::ServiceRemoved(DISCOVERY_SERVICE_TYPE.to_owned(), "SERVICE-0".to_owned()),
        now,
    );
    assert_eq!(cache.sessions(now).len(), MAX_DISCOVERED_SESSIONS - 1);
}

#[test]
fn rotation_removes_old_ad_then_registers_new_id_and_drop_cleans_up() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let now = Instant::now();
    let addresses = [IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10))];
    let mut publisher = DiscoveryPublisher::start_with_backend(
        45123,
        &addresses,
        now,
        Box::new(FakeBackend {
            events: Arc::clone(&events),
            fail_register_call: None,
            register_calls: 0,
            fail_unregister: false,
        }),
    )
    .unwrap();
    let old_id = publisher.discovery_id().to_owned();
    let old_fullname = publisher.service.get_fullname().to_owned();

    assert!(!publisher
        .rotate_if_due(now + DISCOVERY_ROTATION_INTERVAL - Duration::from_millis(1))
        .unwrap());
    let overdue = now + DISCOVERY_ROTATION_INTERVAL + Duration::from_secs(900);
    assert!(publisher.rotate_if_due(overdue).unwrap());
    assert!(!publisher.rotate_if_due(overdue).unwrap());
    let new_id = publisher.discovery_id().to_owned();
    let new_fullname = publisher.service.get_fullname().to_owned();
    assert_ne!(old_id, new_id);
    drop(publisher);

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 5);
    assert!(events[0].ends_with(&old_id));
    assert_eq!(events[1], format!("unregister:{old_fullname}"));
    assert!(events[2].ends_with(&new_id));
    assert_eq!(events[3], format!("unregister:{new_fullname}"));
    assert_eq!(events[4], "shutdown");
}

#[test]
fn transient_rotation_failure_restores_old_ad_without_stopping() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let now = Instant::now();
    let mut publisher = DiscoveryPublisher::start_with_backend(
        45123,
        &[IpAddr::V4(Ipv4Addr::LOCALHOST)],
        now,
        Box::new(FakeBackend {
            events,
            fail_register_call: Some(2),
            register_calls: 0,
            fail_unregister: false,
        }),
    )
    .unwrap();
    let old_id = publisher.discovery_id().to_owned();
    let due = now + DISCOVERY_ROTATION_INTERVAL;
    assert!(publisher.rotate_if_due(due).is_err());
    assert!(!publisher.is_stopped());
    assert_eq!(publisher.discovery_id(), old_id);
    assert_eq!(
        publisher.next_rotation_at(),
        due + DISCOVERY_ROTATION_RETRY_INTERVAL
    );
}

#[test]
fn uncertain_unregister_failure_stops_publisher() {
    let now = Instant::now();
    let mut publisher = DiscoveryPublisher::start_with_backend(
        45123,
        &[IpAddr::V4(Ipv4Addr::LOCALHOST)],
        now,
        Box::new(FakeBackend {
            events: Arc::new(Mutex::new(Vec::new())),
            fail_register_call: None,
            register_calls: 0,
            fail_unregister: true,
        }),
    )
    .unwrap();
    assert!(publisher
        .rotate_if_due(now + DISCOVERY_ROTATION_INTERVAL)
        .is_err());
    assert!(publisher.is_stopped());
}
