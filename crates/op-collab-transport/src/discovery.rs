use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

#[cfg(any(not(target_os = "macos"), test))]
use mdns_sd::ServiceEvent;
#[cfg(not(target_os = "macos"))]
use mdns_sd::{DaemonEvent, Receiver, ServiceDaemon, UnregisterStatus};
use mdns_sd::{IfKind, ServiceInfo};
use thiserror::Error;

use crate::TRANSPORT_PROTOCOL_VERSION;

#[cfg(target_os = "macos")]
#[path = "discovery_bonjour.rs"]
mod bonjour;
#[cfg(target_os = "macos")]
#[path = "discovery_bonjour_browser.rs"]
mod bonjour_browser;
#[cfg(target_os = "macos")]
#[path = "discovery_bonjour_ffi.rs"]
mod bonjour_ffi;
#[cfg(target_os = "macos")]
#[path = "discovery_bonjour_txt.rs"]
mod bonjour_txt;
#[cfg(target_os = "macos")]
use bonjour::BonjourPublisherBackend;
#[cfg(target_os = "macos")]
use bonjour_browser::{BonjourBrowserBackend, BonjourEvent, BonjourResolved};

pub const DISCOVERY_SERVICE_TYPE: &str = "_openpencil-collab._tcp.local.";
pub const MAX_DISCOVERY_ADDRESSES: usize = 16;

const DISCOVERY_ID_BYTES: usize = 16;
const DISCOVERY_ID_HEX_BYTES: usize = DISCOVERY_ID_BYTES * 2;
const RANDOM_LABEL_BYTES: usize = 12;
const MAX_DISCOVERED_SESSIONS: usize = 128;
const DISCOVERY_ROTATION_INTERVAL: Duration = Duration::from_secs(5 * 60);
const DISCOVERY_ROTATION_RETRY_INTERVAL: Duration = Duration::from_secs(30);
#[cfg(not(target_os = "macos"))]
const UNREGISTER_ACK_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("mDNS discovery is unavailable")]
    Mdns(#[from] mdns_sd::Error),
    #[cfg(target_os = "macos")]
    #[error("Bonjour discovery failed with system error {0}")]
    Bonjour(i32),
    #[cfg(target_os = "macos")]
    #[error("Bonjour discovery I/O failed")]
    BonjourIo(#[source] std::io::Error),
    #[cfg(target_os = "macos")]
    #[error("the Bonjour discovery worker stopped unexpectedly")]
    BonjourWorkerStopped,
    #[error("secure randomness is unavailable")]
    Random,
    #[error("the advertised port is invalid")]
    InvalidPort,
    #[error("the advertised address is invalid")]
    InvalidAddress,
    #[error("the discovery metadata is invalid")]
    InvalidMetadata,
    #[error("the discovery publisher has stopped")]
    PublisherStopped,
    #[error("the discovery advertisement could not be withdrawn")]
    UnregisterFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSession {
    discovery_id: String,
    protocol_version: u16,
    addresses: Vec<SocketAddr>,
    expires_at: Instant,
}

impl DiscoveredSession {
    pub fn discovery_id(&self) -> &str {
        &self.discovery_id
    }

    pub const fn protocol_version(&self) -> u16 {
        self.protocol_version
    }

    pub fn addresses(&self) -> &[SocketAddr] {
        &self.addresses
    }

    pub fn primary_address(&self) -> Option<SocketAddr> {
        self.addresses.first().copied()
    }

    pub const fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

/// Publishes an ephemeral endpoint using independent non-PII random labels.
pub struct DiscoveryPublisher {
    backend: Option<Box<dyn PublisherBackend>>,
    discovery_id: String,
    service: ServiceInfo,
    next_rotation_at: Instant,
}

impl DiscoveryPublisher {
    pub fn start(port: u16) -> Result<Self, DiscoveryError> {
        Self::start_with_addresses(port, &[])
    }

    pub fn start_with_addresses(port: u16, addresses: &[IpAddr]) -> Result<Self, DiscoveryError> {
        if port == 0 {
            return Err(DiscoveryError::InvalidPort);
        }

        Self::start_with_backend(
            port,
            addresses,
            Instant::now(),
            platform_publisher_backend(None)?,
        )
    }

    pub fn start_for_listener(
        endpoint: SocketAddr,
        dual_stack: bool,
    ) -> Result<Self, DiscoveryError> {
        if endpoint.port() == 0 {
            return Err(DiscoveryError::InvalidPort);
        }
        if !endpoint.ip().is_unspecified() {
            return Self::start_with_addresses(endpoint.port(), &[endpoint.ip()]);
        }
        Self::start_with_backend(
            endpoint.port(),
            &[],
            Instant::now(),
            platform_publisher_backend(disabled_listener_family(endpoint, dual_stack))?,
        )
    }

    fn start_with_backend(
        port: u16,
        addresses: &[IpAddr],
        now: Instant,
        mut backend: Box<dyn PublisherBackend>,
    ) -> Result<Self, DiscoveryError> {
        let discovery_id = random_hex(DISCOVERY_ID_BYTES)?;
        let instance_name = format!("openpencil-{}", random_hex(RANDOM_LABEL_BYTES)?);
        let hostname = format!("op-{}.local.", random_hex(RANDOM_LABEL_BYTES)?);
        let service =
            build_service_info(&discovery_id, port, addresses, &instance_name, &hostname)?;
        let next_rotation_at = next_rotation(now)?;
        if let Err(error) = backend.register(service.clone()) {
            let _ = backend.shutdown();
            return Err(error);
        }

        Ok(Self {
            backend: Some(backend),
            discovery_id,
            service,
            next_rotation_at,
        })
    }

    pub fn discovery_id(&self) -> &str {
        &self.discovery_id
    }

    pub fn is_stopped(&self) -> bool {
        self.backend.is_none()
    }

    pub const fn next_rotation_at(&self) -> Instant {
        self.next_rotation_at
    }

    /// Rotates once when due; failures restore the old ad or fail closed.
    pub fn rotate_if_due(&mut self, now: Instant) -> Result<bool, DiscoveryError> {
        if now < self.next_rotation_at {
            return Ok(false);
        }
        let Some(backend) = self.backend.as_mut() else {
            return Err(DiscoveryError::PublisherStopped);
        };

        let next_id = random_hex(DISCOVERY_ID_BYTES)?;
        let instance_name = format!("openpencil-{}", random_hex(RANDOM_LABEL_BYTES)?);
        let hostname = format!("op-{}.local.", random_hex(RANDOM_LABEL_BYTES)?);
        let addresses: Vec<_> = self.service.get_addresses().iter().copied().collect();
        let next_service = build_service_info(
            &next_id,
            self.service.get_port(),
            &addresses,
            &instance_name,
            &hostname,
        )?;
        let next_rotation_at = next_rotation(now)?;

        if let Err(error) = backend.unregister(&self.service) {
            if matches!(error, DiscoveryError::UnregisterFailed) {
                let _ = backend.shutdown();
                self.backend = None;
            } else {
                self.next_rotation_at = rotation_retry(now)?;
            }
            return Err(error);
        }
        if let Err(error) = backend.register(next_service.clone()) {
            if backend.register(self.service.clone()).is_err() {
                let _ = backend.shutdown();
                self.backend = None;
            } else {
                self.next_rotation_at = rotation_retry(now)?;
            }
            return Err(error);
        }

        self.discovery_id = next_id;
        self.service = next_service;
        self.next_rotation_at = next_rotation_at;
        Ok(true)
    }

    pub fn stop(mut self) -> Result<(), DiscoveryError> {
        self.stop_inner()
    }

    fn stop_inner(&mut self) -> Result<(), DiscoveryError> {
        let Some(mut backend) = self.backend.take() else {
            return Ok(());
        };
        let unregister_error = backend.unregister(&self.service).err();
        let shutdown_error = backend.shutdown().err();
        if let Some(error) = unregister_error.or(shutdown_error) {
            Err(error)
        } else {
            Ok(())
        }
    }
}

impl Drop for DiscoveryPublisher {
    fn drop(&mut self) {
        let _ = self.stop_inner();
    }
}

trait PublisherBackend {
    fn register(&mut self, service: ServiceInfo) -> Result<(), DiscoveryError>;
    fn unregister(&mut self, service: &ServiceInfo) -> Result<(), DiscoveryError>;
    fn shutdown(&mut self) -> Result<(), DiscoveryError>;
}

#[cfg(target_os = "macos")]
fn platform_publisher_backend(
    disabled_family: Option<IfKind>,
) -> Result<Box<dyn PublisherBackend>, DiscoveryError> {
    Ok(Box::new(BonjourPublisherBackend::new(disabled_family)))
}

#[cfg(not(target_os = "macos"))]
fn platform_publisher_backend(
    disabled_family: Option<IfKind>,
) -> Result<Box<dyn PublisherBackend>, DiscoveryError> {
    let daemon = ServiceDaemon::new()?;
    if let Some(family) = disabled_family {
        if let Err(error) = daemon.disable_interface(family) {
            let _ = daemon.shutdown();
            return Err(error.into());
        }
    }
    Ok(Box::new(MdnsPublisherBackend::new(daemon)?))
}

#[cfg(not(target_os = "macos"))]
struct MdnsPublisherBackend {
    daemon: ServiceDaemon,
    monitor: Receiver<DaemonEvent>,
}

#[cfg(not(target_os = "macos"))]
impl MdnsPublisherBackend {
    fn new(daemon: ServiceDaemon) -> Result<Self, DiscoveryError> {
        match daemon.monitor() {
            Ok(monitor) => Ok(Self { daemon, monitor }),
            Err(error) => {
                let _ = daemon.shutdown();
                Err(error.into())
            }
        }
    }

    fn unregister_one(&self, fullname: &str) -> Result<(), DiscoveryError> {
        let status = self
            .daemon
            .unregister(fullname)?
            .recv_timeout(UNREGISTER_ACK_TIMEOUT)
            .map_err(|_| DiscoveryError::UnregisterFailed)?;
        match status {
            UnregisterStatus::OK => Ok(()),
            UnregisterStatus::NotFound => Err(DiscoveryError::UnregisterFailed),
        }
    }

    fn renamed_fullnames(&self, original: &str) -> BTreeSet<String> {
        self.monitor
            .try_iter()
            .filter_map(|event| match event {
                DaemonEvent::NameChange(change)
                    if is_tracked_rename(original, &change.original, &change.new_name) =>
                {
                    Some(change.new_name)
                }
                _ => None,
            })
            .collect()
    }
}

#[cfg(not(target_os = "macos"))]
impl PublisherBackend for MdnsPublisherBackend {
    fn register(&mut self, service: ServiceInfo) -> Result<(), DiscoveryError> {
        self.daemon.register(service).map_err(Into::into)
    }

    fn unregister(&mut self, service: &ServiceInfo) -> Result<(), DiscoveryError> {
        let original = service.get_fullname();
        let mut renamed = self.renamed_fullnames(original);
        self.unregister_one(original)?;
        renamed.extend(self.renamed_fullnames(original));
        for fullname in renamed {
            let instance = renamed_instance(&fullname)?;
            let addresses: Vec<_> = service.get_addresses().iter().copied().collect();
            let id = property_value(service, "id").map_err(|_| DiscoveryError::UnregisterFailed)?;
            let mut tombstone = build_service_info(
                id,
                service.get_port(),
                &addresses,
                instance,
                service.get_hostname(),
            )
            .map_err(|_| DiscoveryError::UnregisterFailed)?;
            tombstone.set_requires_probe(false);
            self.daemon
                .register(tombstone.clone())
                .map_err(|_| DiscoveryError::UnregisterFailed)?;
            self.unregister_one(tombstone.get_fullname())
                .map_err(|_| DiscoveryError::UnregisterFailed)?;
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), DiscoveryError> {
        self.daemon.shutdown().map(|_| ()).map_err(Into::into)
    }
}

pub struct DiscoveryBrowser {
    #[cfg(not(target_os = "macos"))]
    daemon: Option<ServiceDaemon>,
    #[cfg(not(target_os = "macos"))]
    receiver: Receiver<ServiceEvent>,
    #[cfg(target_os = "macos")]
    backend: Option<BonjourBrowserBackend>,
    cache: DiscoveryCache,
}

impl DiscoveryBrowser {
    pub fn start() -> Result<Self, DiscoveryError> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                backend: Some(BonjourBrowserBackend::start()?),
                cache: DiscoveryCache::default(),
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            let daemon = ServiceDaemon::new()?;
            let receiver = match daemon.browse(DISCOVERY_SERVICE_TYPE) {
                Ok(receiver) => receiver,
                Err(error) => {
                    let _ = daemon.shutdown();
                    return Err(error.into());
                }
            };
            Ok(Self {
                daemon: Some(daemon),
                receiver,
                cache: DiscoveryCache::default(),
            })
        }
    }

    pub fn poll(&mut self) -> Vec<DiscoveredSession> {
        let now = Instant::now();
        #[cfg(target_os = "macos")]
        if let Some(backend) = self.backend.as_mut() {
            while let Some(event) = backend.try_recv() {
                self.cache.process_bonjour_event(event, now);
            }
        }
        #[cfg(not(target_os = "macos"))]
        while let Ok(event) = self.receiver.try_recv() {
            self.cache.process_event(event, now);
        }
        self.cache.sessions(now)
    }

    pub fn wait(&mut self, timeout: Duration) -> Vec<DiscoveredSession> {
        #[cfg(target_os = "macos")]
        if let Some(backend) = self.backend.as_mut() {
            if let Some(event) = backend.recv_timeout(timeout) {
                self.cache.process_bonjour_event(event, Instant::now());
            }
        }
        #[cfg(not(target_os = "macos"))]
        if let Ok(event) = self.receiver.recv_timeout(timeout) {
            self.cache.process_event(event, Instant::now());
        }
        self.poll()
    }

    pub fn sessions(&mut self) -> Vec<DiscoveredSession> {
        self.cache.sessions(Instant::now())
    }

    pub fn len(&mut self) -> usize {
        self.sessions().len()
    }

    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    pub fn stop(mut self) -> Result<(), DiscoveryError> {
        self.stop_inner()
    }

    fn stop_inner(&mut self) -> Result<(), DiscoveryError> {
        #[cfg(target_os = "macos")]
        {
            let Some(backend) = self.backend.take() else {
                return Ok(());
            };
            backend.stop()
        }

        #[cfg(not(target_os = "macos"))]
        {
            let Some(daemon) = self.daemon.take() else {
                return Ok(());
            };
            let browse_error = daemon.stop_browse(DISCOVERY_SERVICE_TYPE).err();
            let shutdown_error = daemon.shutdown().err();
            if let Some(error) = browse_error.or(shutdown_error) {
                Err(error.into())
            } else {
                Ok(())
            }
        }
    }
}

impl Drop for DiscoveryBrowser {
    fn drop(&mut self) {
        let _ = self.stop_inner();
    }
}

#[derive(Default)]
struct DiscoveryCache {
    by_fullname: BTreeMap<String, DiscoveredSession>,
}

impl DiscoveryCache {
    #[cfg(any(not(target_os = "macos"), test))]
    fn process_event(&mut self, event: ServiceEvent, now: Instant) {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                if let Ok(session) = parse_service_info(&info, now) {
                    self.insert(normalize_fullname(info.get_fullname()), session, now);
                }
            }
            ServiceEvent::ServiceRemoved(service_type, fullname)
                if service_type.eq_ignore_ascii_case(DISCOVERY_SERVICE_TYPE) =>
            {
                self.by_fullname.remove(&normalize_fullname(&fullname));
            }
            _ => {}
        }
    }

    #[cfg(target_os = "macos")]
    fn process_bonjour_event(&mut self, event: BonjourEvent, now: Instant) {
        match event {
            BonjourEvent::Resolved(resolved) => {
                let fullname = normalize_fullname(&resolved.fullname);
                match parse_bonjour_resolved(&resolved, now) {
                    Ok(session) => self.insert(fullname, session, now),
                    Err(_) => {
                        self.by_fullname.remove(&fullname);
                    }
                }
            }
            BonjourEvent::Removed { fullname } => {
                self.by_fullname.remove(&normalize_fullname(&fullname));
            }
        }
    }

    fn insert(&mut self, fullname: String, session: DiscoveredSession, now: Instant) {
        self.expire(now);
        if self.by_fullname.contains_key(&fullname)
            || self.by_fullname.len() < MAX_DISCOVERED_SESSIONS
        {
            self.by_fullname.insert(fullname, session);
        }
    }

    fn sessions(&mut self, now: Instant) -> Vec<DiscoveredSession> {
        self.expire(now);
        let mut sessions: Vec<_> = self.by_fullname.values().cloned().collect();
        sessions.sort_by(|left, right| {
            left.discovery_id
                .cmp(&right.discovery_id)
                .then_with(|| left.addresses.cmp(&right.addresses))
        });
        sessions
    }

    fn expire(&mut self, now: Instant) {
        self.by_fullname
            .retain(|_, session| session.expires_at > now);
    }
}

fn build_service_info(
    discovery_id: &str,
    port: u16,
    addresses: &[IpAddr],
    instance_name: &str,
    hostname: &str,
) -> Result<ServiceInfo, DiscoveryError> {
    validate_discovery_id(discovery_id)?;
    if port == 0 {
        return Err(DiscoveryError::InvalidPort);
    }
    validate_publish_addresses(addresses)?;
    let version = TRANSPORT_PROTOCOL_VERSION.to_string();
    let advertised_port = port.to_string();
    let properties = [
        ("id", discovery_id),
        ("v", version.as_str()),
        ("p", advertised_port.as_str()),
    ];
    let service = ServiceInfo::new(
        DISCOVERY_SERVICE_TYPE,
        instance_name,
        hostname,
        addresses,
        port,
        &properties[..],
    )?;
    Ok(if addresses.is_empty() {
        service.enable_addr_auto()
    } else {
        service
    })
}

#[cfg(any(not(target_os = "macos"), test))]
fn parse_service_info(
    info: &ServiceInfo,
    now: Instant,
) -> Result<DiscoveredSession, DiscoveryError> {
    if !info.get_type().eq_ignore_ascii_case(DISCOVERY_SERVICE_TYPE) {
        return Err(DiscoveryError::InvalidMetadata);
    }
    let properties = info.get_properties();
    if properties.len() != 3 {
        return Err(DiscoveryError::InvalidMetadata);
    }
    for property in properties.iter() {
        if !matches!(property.key(), "id" | "v" | "p") || property.val().is_none() {
            return Err(DiscoveryError::InvalidMetadata);
        }
        std::str::from_utf8(property.val().unwrap_or_default())
            .map_err(|_| DiscoveryError::InvalidMetadata)?;
    }

    let ttl_seconds = info.get_host_ttl().min(info.get_other_ttl());
    build_discovered_session(
        property_value(info, "id")?,
        property_value(info, "v")?,
        property_value(info, "p")?,
        info.get_port(),
        info.get_addresses().iter().copied(),
        ttl_seconds,
        now,
    )
}

#[cfg(target_os = "macos")]
fn parse_bonjour_resolved(
    resolved: &BonjourResolved,
    now: Instant,
) -> Result<DiscoveredSession, DiscoveryError> {
    if !resolved
        .service_type
        .eq_ignore_ascii_case(DISCOVERY_SERVICE_TYPE)
    {
        return Err(DiscoveryError::InvalidMetadata);
    }
    let properties = bonjour_txt::decode_txt_record(&resolved.txt)?;
    build_discovered_session(
        bonjour_txt::property_from_pairs(&properties, "id")?,
        bonjour_txt::property_from_pairs(&properties, "v")?,
        bonjour_txt::property_from_pairs(&properties, "p")?,
        resolved.port,
        resolved.addresses.iter().copied(),
        resolved.ttl_seconds,
        now,
    )
}

fn build_discovered_session(
    discovery_id: &str,
    protocol_version: &str,
    advertised_port: &str,
    service_port: u16,
    addresses: impl IntoIterator<Item = IpAddr>,
    ttl_seconds: u32,
    now: Instant,
) -> Result<DiscoveredSession, DiscoveryError> {
    validate_discovery_id(discovery_id)?;
    let protocol_version = parse_canonical_u16(protocol_version)?;
    if protocol_version == 0 {
        return Err(DiscoveryError::InvalidMetadata);
    }
    let port = parse_canonical_u16(advertised_port)?;
    if port == 0 || port != service_port || ttl_seconds == 0 {
        return Err(DiscoveryError::InvalidMetadata);
    }

    let mut addresses: Vec<_> = addresses
        .into_iter()
        .filter(is_usable_address)
        .map(|address| SocketAddr::new(address, port))
        .collect();
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() || addresses.len() > MAX_DISCOVERY_ADDRESSES {
        return Err(DiscoveryError::InvalidMetadata);
    }

    let expires_at = now
        .checked_add(Duration::from_secs(u64::from(ttl_seconds)))
        .ok_or(DiscoveryError::InvalidMetadata)?;
    Ok(DiscoveredSession {
        discovery_id: discovery_id.to_owned(),
        protocol_version,
        addresses,
        expires_at,
    })
}

fn disabled_listener_family(endpoint: SocketAddr, dual_stack: bool) -> Option<IfKind> {
    match (endpoint.is_ipv4(), dual_stack) {
        (true, _) => Some(IfKind::IPv6),
        (false, false) => Some(IfKind::IPv4),
        (false, true) => None,
    }
}

fn is_usable_address(address: &IpAddr) -> bool {
    !address.is_unspecified()
        && !address.is_multicast()
        && !matches!(address, IpAddr::V6(address) if address.is_unicast_link_local())
}

fn validate_publish_addresses(addresses: &[IpAddr]) -> Result<(), DiscoveryError> {
    if addresses.iter().any(|address| !is_usable_address(address)) {
        return Err(DiscoveryError::InvalidAddress);
    }
    let unique: BTreeSet<_> = addresses.iter().collect();
    if unique.len() > MAX_DISCOVERY_ADDRESSES {
        return Err(DiscoveryError::InvalidAddress);
    }
    Ok(())
}

fn renamed_instance(fullname: &str) -> Result<&str, DiscoveryError> {
    fullname
        .strip_suffix(DISCOVERY_SERVICE_TYPE)
        .and_then(|prefix| prefix.strip_suffix('.'))
        .filter(|instance| !instance.is_empty())
        .ok_or(DiscoveryError::UnregisterFailed)
}

#[cfg(any(not(target_os = "macos"), test))]
fn is_tracked_rename(original: &str, changed: &str, new_name: &str) -> bool {
    changed.eq_ignore_ascii_case(original)
        && new_name
            .to_ascii_lowercase()
            .ends_with(DISCOVERY_SERVICE_TYPE)
}

fn property_value<'a>(info: &'a ServiceInfo, key: &str) -> Result<&'a str, DiscoveryError> {
    let property = info
        .get_properties()
        .get(key)
        .ok_or(DiscoveryError::InvalidMetadata)?;
    let value = property.val().ok_or(DiscoveryError::InvalidMetadata)?;
    std::str::from_utf8(value).map_err(|_| DiscoveryError::InvalidMetadata)
}

fn parse_canonical_u16(value: &str) -> Result<u16, DiscoveryError> {
    let parsed = value
        .parse::<u16>()
        .map_err(|_| DiscoveryError::InvalidMetadata)?;
    if parsed.to_string() != value {
        return Err(DiscoveryError::InvalidMetadata);
    }
    Ok(parsed)
}

fn validate_discovery_id(value: &str) -> Result<(), DiscoveryError> {
    if value.len() != DISCOVERY_ID_HEX_BYTES
        || !value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(DiscoveryError::InvalidMetadata);
    }
    Ok(())
}

fn random_hex(byte_count: usize) -> Result<String, DiscoveryError> {
    let mut bytes = vec![0_u8; byte_count];
    getrandom::fill(&mut bytes).map_err(|_| DiscoveryError::Random)?;
    let mut encoded = String::with_capacity(byte_count * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").map_err(|_| DiscoveryError::Random)?;
    }
    Ok(encoded)
}

fn next_rotation(now: Instant) -> Result<Instant, DiscoveryError> {
    now.checked_add(DISCOVERY_ROTATION_INTERVAL)
        .ok_or(DiscoveryError::InvalidMetadata)
}

fn rotation_retry(now: Instant) -> Result<Instant, DiscoveryError> {
    now.checked_add(DISCOVERY_ROTATION_RETRY_INTERVAL)
        .ok_or(DiscoveryError::InvalidMetadata)
}

fn normalize_fullname(fullname: &str) -> String {
    fullname.to_ascii_lowercase()
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
