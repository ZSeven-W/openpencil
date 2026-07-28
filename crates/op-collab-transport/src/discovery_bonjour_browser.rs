use std::collections::{BTreeMap, VecDeque};
use std::ffi::{c_char, c_void, CStr, CString};
use std::net::IpAddr;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::bonjour::{poll_connection, socket_address_ip, DnsConnection};
use super::bonjour_ffi as ffi;
use super::{
    is_usable_address, DiscoveryError, DISCOVERY_SERVICE_TYPE, MAX_DISCOVERED_SESSIONS,
    MAX_DISCOVERY_ADDRESSES,
};

const REGISTRATION_TYPE: &[u8] = b"_openpencil-collab._tcp\0";
const LOCAL_DOMAIN: &[u8] = b"local.\0";
const START_TIMEOUT: Duration = Duration::from_secs(5);
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_INTERVAL: Duration = Duration::from_secs(1);
const MAX_CALLBACK_STRING_BYTES: usize = 1_008;
const MAX_TXT_BYTES: usize = 1_024;
const MAX_INTERFACE_STATES: usize = MAX_DISCOVERED_SESSIONS * 4;
const MAX_CALLBACK_ACTIONS: usize = MAX_INTERFACE_STATES * 4;
const MAX_QUEUED_EVENTS: usize = MAX_DISCOVERED_SESSIONS * 2;
// DNSServiceGetAddrInfo remains active after an address TTL elapses. Refresh a
// short upper-layer lease while the query is alive so dropped events recover,
// while a dead worker still ages out of the cache promptly.
const CACHE_LEASE_SECONDS: u32 = 30;
const LEASE_REFRESH_INTERVAL: Duration = Duration::from_secs(10);

pub(super) enum BonjourEvent {
    Resolved(BonjourResolved),
    Removed { fullname: String },
}

pub(super) struct BonjourResolved {
    pub(super) fullname: String,
    pub(super) service_type: String,
    pub(super) port: u16,
    pub(super) txt: Vec<u8>,
    pub(super) addresses: Vec<IpAddr>,
    pub(super) ttl_seconds: u32,
}

pub(super) struct BonjourBrowserBackend {
    events: Receiver<BonjourEvent>,
    stop_sender: Sender<()>,
    join: Option<JoinHandle<Result<(), DiscoveryError>>>,
}

impl BonjourBrowserBackend {
    pub(super) fn start() -> Result<Self, DiscoveryError> {
        let (event_sender, events) = mpsc::sync_channel(MAX_QUEUED_EVENTS);
        let (stop_sender, stop_receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let join = thread::Builder::new()
            .name("op-bonjour-browser".to_owned())
            .spawn(move || browser_worker(event_sender, stop_receiver, ready_sender))
            .map_err(DiscoveryError::BonjourIo)?;
        let mut backend = Self {
            events,
            stop_sender,
            join: Some(join),
        };
        match ready_receiver.recv_timeout(START_TIMEOUT) {
            Ok(Ok(())) => Ok(backend),
            Ok(Err(error)) => {
                let _ = backend.stop_inner();
                Err(DiscoveryError::Bonjour(error))
            }
            Err(_) => {
                let _ = backend.stop_inner();
                Err(DiscoveryError::BonjourWorkerStopped)
            }
        }
    }

    pub(super) fn try_recv(&mut self) -> Option<BonjourEvent> {
        self.events.try_recv().ok()
    }

    pub(super) fn recv_timeout(&mut self, timeout: Duration) -> Option<BonjourEvent> {
        self.events.recv_timeout(timeout).ok()
    }

    pub(super) fn stop(mut self) -> Result<(), DiscoveryError> {
        self.stop_inner()
    }

    fn stop_inner(&mut self) -> Result<(), DiscoveryError> {
        let _ = self.stop_sender.send(());
        let Some(join) = self.join.take() else {
            return Ok(());
        };
        join.join()
            .map_err(|_| DiscoveryError::BonjourWorkerStopped)?
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ServiceKey {
    name: String,
    registration_type: String,
    domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct InterfaceKey {
    service: ServiceKey,
    interface_index: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedMetadata {
    fullname: String,
    hostname: String,
    port: u16,
    txt: Vec<u8>,
}

struct InterfaceState {
    metadata: Option<ResolvedMetadata>,
    addresses: BTreeMap<IpAddr, u32>,
    overflowed: bool,
    resolve_ref: Option<ffi::ServiceRef>,
    resolve_started_at: Option<Instant>,
    address_ref: Option<ffi::ServiceRef>,
    retry_at: Option<Instant>,
}

impl InterfaceState {
    fn new(now: Instant) -> Self {
        Self {
            metadata: None,
            addresses: BTreeMap::new(),
            overflowed: false,
            resolve_ref: None,
            resolve_started_at: None,
            address_ref: None,
            retry_at: Some(now),
        }
    }
}

enum BrowserAction {
    Browse {
        added: bool,
        interface_index: u32,
        name: String,
        registration_type: String,
        domain: String,
    },
    BrowseError(i32),
    Resolve {
        operation: ffi::ServiceRef,
        result: Result<ResolvedMetadata, i32>,
    },
    Address {
        operation: ffi::ServiceRef,
        result: Result<AddressChange, i32>,
    },
}

struct AddressChange {
    added: bool,
    address: Option<IpAddr>,
    ttl_seconds: u32,
}

#[derive(Default)]
struct BrowserCallbacks {
    actions: VecDeque<BrowserAction>,
}

impl BrowserCallbacks {
    fn push(&mut self, action: BrowserAction) {
        if self.actions.len() < MAX_CALLBACK_ACTIONS {
            self.actions.push_back(action);
        } else {
            self.actions.clear();
            self.actions.push_back(BrowserAction::BrowseError(-1));
        }
    }
}

struct BrowserState {
    callback_context: *mut c_void,
    interfaces: BTreeMap<InterfaceKey, InterfaceState>,
    resolve_operations: BTreeMap<ffi::ServiceRef, InterfaceKey>,
    address_operations: BTreeMap<ffi::ServiceRef, InterfaceKey>,
    last_emitted: BTreeMap<ServiceKey, String>,
    next_lease_refresh_at: Instant,
}

fn browser_worker(
    events: SyncSender<BonjourEvent>,
    stop: Receiver<()>,
    ready: mpsc::SyncSender<Result<(), i32>>,
) -> Result<(), DiscoveryError> {
    let mut callbacks = Box::<BrowserCallbacks>::default();
    let context = (&mut *callbacks as *mut BrowserCallbacks).cast::<c_void>();
    let connection = DnsConnection::create()?;
    let mut browse_ref = connection.raw;
    let error = unsafe {
        ffi::browse(
            &mut browse_ref,
            ffi::FLAG_SHARE_CONNECTION,
            0,
            REGISTRATION_TYPE.as_ptr().cast(),
            LOCAL_DOMAIN.as_ptr().cast(),
            Some(browse_callback),
            context,
        )
    };
    if error != ffi::NO_ERROR {
        let _ = ready.send(Err(error));
        return Err(DiscoveryError::Bonjour(error));
    }
    let _ = ready.send(Ok(()));

    let descriptor = connection.socket_fd()?;
    let mut state = BrowserState::new(context);
    loop {
        if stop.try_recv().is_ok() {
            return Ok(());
        }
        poll_connection(&connection, descriptor)?;
        while let Some(action) = callbacks.actions.pop_front() {
            if !state.handle_action(action, connection.raw, &events)? {
                return Ok(());
            }
        }
        if !state.run_housekeeping(connection.raw, &events)? {
            return Ok(());
        }
    }
}

impl BrowserState {
    fn new(callback_context: *mut c_void) -> Self {
        Self {
            callback_context,
            interfaces: BTreeMap::new(),
            resolve_operations: BTreeMap::new(),
            address_operations: BTreeMap::new(),
            last_emitted: BTreeMap::new(),
            next_lease_refresh_at: Instant::now() + LEASE_REFRESH_INTERVAL,
        }
    }

    fn handle_action(
        &mut self,
        action: BrowserAction,
        connection: ffi::ServiceRef,
        events: &SyncSender<BonjourEvent>,
    ) -> Result<bool, DiscoveryError> {
        match action {
            BrowserAction::Browse {
                added,
                interface_index,
                name,
                registration_type,
                domain,
            } => {
                let service = ServiceKey {
                    name,
                    registration_type,
                    domain,
                };
                if !valid_service_key(&service) || interface_index == 0 {
                    return Ok(true);
                }
                let key = InterfaceKey {
                    service: service.clone(),
                    interface_index,
                };
                if added {
                    if !self.interfaces.contains_key(&key)
                        && self.interfaces.len() < MAX_INTERFACE_STATES
                        && (self.has_service(&service)
                            || self.unique_service_count() < MAX_DISCOVERED_SESSIONS)
                    {
                        self.interfaces
                            .insert(key.clone(), InterfaceState::new(Instant::now()));
                        self.start_resolve(connection, &key);
                    }
                } else if let Some(state) = self.interfaces.remove(&key) {
                    self.cancel_state_operations(&key, &state);
                    if !self.emit_service(&service, events) {
                        return Ok(false);
                    }
                }
            }
            BrowserAction::BrowseError(error) => return Err(DiscoveryError::Bonjour(error)),
            BrowserAction::Resolve { operation, result } => {
                let Some(key) = self.resolve_operations.remove(&operation) else {
                    return Ok(true);
                };
                unsafe { ffi::deallocate(operation) };
                if let Some(state) = self.interfaces.get_mut(&key) {
                    state.resolve_ref = None;
                    state.resolve_started_at = None;
                    match result {
                        Ok(metadata) if valid_resolved_metadata(&metadata) => {
                            state.metadata = Some(metadata);
                            state.addresses.clear();
                            state.overflowed = false;
                            state.retry_at = None;
                            self.start_address_query(connection, &key);
                        }
                        _ => {
                            state.metadata = None;
                            state.retry_at = Instant::now().checked_add(RETRY_INTERVAL);
                        }
                    }
                }
            }
            BrowserAction::Address { operation, result } => {
                let Some(key) = self.address_operations.get(&operation).cloned() else {
                    return Ok(true);
                };
                match result {
                    Ok(change) => {
                        if let Some(state) = self.interfaces.get_mut(&key) {
                            if let Some(address) = change.address {
                                if change.added
                                    && change.ttl_seconds > 0
                                    && is_usable_address(&address)
                                {
                                    if state.addresses.contains_key(&address)
                                        || state.addresses.len() < MAX_DISCOVERY_ADDRESSES + 1
                                    {
                                        state.addresses.insert(address, change.ttl_seconds);
                                    } else {
                                        state.overflowed = true;
                                    }
                                } else {
                                    state.addresses.remove(&address);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        self.address_operations.remove(&operation);
                        unsafe { ffi::deallocate(operation) };
                        if let Some(state) = self.interfaces.get_mut(&key) {
                            state.address_ref = None;
                            state.metadata = None;
                            state.addresses.clear();
                            state.retry_at = Instant::now().checked_add(RETRY_INTERVAL);
                        }
                    }
                }
                if !self.emit_service(&key.service, events) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn start_resolve(&mut self, connection: ffi::ServiceRef, key: &InterfaceKey) {
        let Some(state) = self.interfaces.get(key) else {
            return;
        };
        if state.resolve_ref.is_some() || state.address_ref.is_some() {
            return;
        }
        let Ok(name) = CString::new(key.service.name.as_str()) else {
            return;
        };
        let Ok(registration_type) = CString::new(key.service.registration_type.as_str()) else {
            return;
        };
        let Ok(domain) = CString::new(key.service.domain.as_str()) else {
            return;
        };
        let mut operation = connection;
        let error = unsafe {
            ffi::resolve(
                &mut operation,
                ffi::FLAG_SHARE_CONNECTION | ffi::FLAG_FORCE_MULTICAST,
                key.interface_index,
                name.as_ptr(),
                registration_type.as_ptr(),
                domain.as_ptr(),
                Some(resolve_callback),
                self.callback_context,
            )
        };
        if error == ffi::NO_ERROR {
            self.resolve_operations.insert(operation, key.clone());
            if let Some(state) = self.interfaces.get_mut(key) {
                state.resolve_ref = Some(operation);
                state.resolve_started_at = Some(Instant::now());
                state.retry_at = None;
            }
        } else if let Some(state) = self.interfaces.get_mut(key) {
            state.retry_at = Instant::now().checked_add(RETRY_INTERVAL);
        }
    }

    fn start_address_query(&mut self, connection: ffi::ServiceRef, key: &InterfaceKey) {
        let Some(hostname) = self
            .interfaces
            .get(key)
            .and_then(|state| state.metadata.as_ref())
            .map(|metadata| metadata.hostname.clone())
        else {
            return;
        };
        let Ok(hostname) = CString::new(hostname) else {
            return;
        };
        let mut operation = connection;
        let error = unsafe {
            ffi::get_addr_info(
                &mut operation,
                ffi::FLAG_SHARE_CONNECTION | ffi::FLAG_FORCE_MULTICAST,
                key.interface_index,
                ffi::PROTOCOL_IPV4 | ffi::PROTOCOL_IPV6,
                hostname.as_ptr(),
                Some(address_callback),
                self.callback_context,
            )
        };
        if error == ffi::NO_ERROR {
            self.address_operations.insert(operation, key.clone());
            if let Some(state) = self.interfaces.get_mut(key) {
                state.address_ref = Some(operation);
            }
        } else if let Some(state) = self.interfaces.get_mut(key) {
            state.metadata = None;
            state.retry_at = Instant::now().checked_add(RETRY_INTERVAL);
        }
    }

    fn run_housekeeping(
        &mut self,
        connection: ffi::ServiceRef,
        events: &SyncSender<BonjourEvent>,
    ) -> Result<bool, DiscoveryError> {
        let now = Instant::now();
        let timed_out: Vec<_> = self
            .interfaces
            .iter()
            .filter_map(|(key, state)| {
                state
                    .resolve_started_at
                    .filter(|started| now.duration_since(*started) >= RESOLVE_TIMEOUT)
                    .map(|_| key.clone())
            })
            .collect();
        for key in timed_out {
            if let Some(operation) = self
                .interfaces
                .get_mut(&key)
                .and_then(|state| state.resolve_ref.take())
            {
                self.resolve_operations.remove(&operation);
                unsafe { ffi::deallocate(operation) };
            }
            if let Some(state) = self.interfaces.get_mut(&key) {
                state.resolve_started_at = None;
                state.retry_at = now.checked_add(RETRY_INTERVAL);
            }
            if !self.emit_service(&key.service, events) {
                return Ok(false);
            }
        }
        let due: Vec<_> = self
            .interfaces
            .iter()
            .filter_map(|(key, state)| {
                (state.resolve_ref.is_none()
                    && state.address_ref.is_none()
                    && state.retry_at.is_some_and(|retry| retry <= now))
                .then_some(key.clone())
            })
            .collect();
        for key in due {
            self.start_resolve(connection, &key);
        }
        if !self.refresh_leases(now, events) {
            return Ok(false);
        }
        Ok(true)
    }

    fn refresh_leases(&mut self, now: Instant, events: &SyncSender<BonjourEvent>) -> bool {
        if now < self.next_lease_refresh_at {
            return true;
        }
        let services: Vec<_> = self.last_emitted.keys().cloned().collect();
        if services
            .iter()
            .any(|service| !self.emit_service(service, events))
        {
            return false;
        }
        self.next_lease_refresh_at = now + LEASE_REFRESH_INTERVAL;
        true
    }

    fn emit_service(&mut self, service: &ServiceKey, events: &SyncSender<BonjourEvent>) -> bool {
        let chosen = self
            .interfaces
            .iter()
            .filter(|(key, state)| {
                &key.service == service && state.metadata.is_some() && !state.addresses.is_empty()
            })
            .find_map(|(_, state)| state.metadata.clone());
        let Some(metadata) = chosen else {
            return self.remove_last(service, events);
        };

        let mut addresses = BTreeMap::<IpAddr, u32>::new();
        let mut overflowed = false;
        for (key, state) in &self.interfaces {
            if &key.service != service || state.metadata.as_ref() != Some(&metadata) {
                continue;
            }
            overflowed |= state.overflowed;
            for (address, ttl) in &state.addresses {
                if addresses.contains_key(address) || addresses.len() < MAX_DISCOVERY_ADDRESSES + 1
                {
                    addresses.insert(*address, *ttl);
                } else {
                    overflowed = true;
                }
            }
        }
        if addresses.is_empty() {
            return self.remove_last(service, events);
        }
        let mut addresses: Vec<_> = addresses.into_iter().collect();
        if overflowed && addresses.len() <= MAX_DISCOVERY_ADDRESSES {
            return self.remove_last(service, events);
        }
        addresses.sort_unstable_by_key(|(address, _)| *address);
        let addresses = addresses.into_iter().map(|(address, _)| address).collect();

        if let Some(old) = self
            .last_emitted
            .insert(service.clone(), metadata.fullname.clone())
        {
            if !old.eq_ignore_ascii_case(&metadata.fullname)
                && !try_send_event(events, BonjourEvent::Removed { fullname: old })
            {
                return false;
            }
        }
        try_send_event(
            events,
            BonjourEvent::Resolved(BonjourResolved {
                fullname: metadata.fullname,
                service_type: DISCOVERY_SERVICE_TYPE.to_owned(),
                port: metadata.port,
                txt: metadata.txt,
                addresses,
                ttl_seconds: CACHE_LEASE_SECONDS,
            }),
        )
    }

    fn remove_last(&mut self, service: &ServiceKey, events: &SyncSender<BonjourEvent>) -> bool {
        match self.last_emitted.remove(service) {
            Some(fullname) => try_send_event(events, BonjourEvent::Removed { fullname }),
            None => true,
        }
    }

    fn cancel_state_operations(&mut self, key: &InterfaceKey, state: &InterfaceState) {
        if let Some(operation) = state.resolve_ref {
            self.resolve_operations.remove(&operation);
            unsafe { ffi::deallocate(operation) };
        }
        if let Some(operation) = state.address_ref {
            self.address_operations.remove(&operation);
            unsafe { ffi::deallocate(operation) };
        }
        debug_assert!(!self.interfaces.contains_key(key));
    }

    fn has_service(&self, service: &ServiceKey) -> bool {
        self.interfaces
            .keys()
            .any(|candidate| &candidate.service == service)
    }

    fn unique_service_count(&self) -> usize {
        self.interfaces
            .keys()
            .map(|key| &key.service)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }
}

fn try_send_event(events: &SyncSender<BonjourEvent>, event: BonjourEvent) -> bool {
    match events.try_send(event) {
        // Housekeeping re-emits current state, so saturation is recoverable
        // without growing an unbounded queue or killing the browser worker.
        Ok(()) | Err(TrySendError::Full(_)) => true,
        Err(TrySendError::Disconnected(_)) => false,
    }
}

fn valid_service_key(key: &ServiceKey) -> bool {
    key.name.len() <= 63
        && key
            .registration_type
            .trim_end_matches('.')
            .eq_ignore_ascii_case("_openpencil-collab._tcp")
        && key
            .domain
            .trim_end_matches('.')
            .eq_ignore_ascii_case("local")
}

fn valid_resolved_metadata(metadata: &ResolvedMetadata) -> bool {
    metadata.port != 0
        && metadata.fullname.len() <= MAX_CALLBACK_STRING_BYTES
        && metadata
            .fullname
            .to_ascii_lowercase()
            .ends_with(DISCOVERY_SERVICE_TYPE)
        && metadata.hostname.to_ascii_lowercase().ends_with(".local.")
        && !metadata.txt.is_empty()
        && metadata.txt.len() <= MAX_TXT_BYTES
}

unsafe extern "C" fn browse_callback(
    _service_ref: ffi::ServiceRef,
    flags: ffi::Flags,
    interface_index: u32,
    error: ffi::Error,
    name: *const c_char,
    registration_type: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    let Some(callbacks) = (unsafe { callback_state(context) }) else {
        return;
    };
    if error != ffi::NO_ERROR {
        callbacks.push(BrowserAction::BrowseError(error));
        return;
    }
    let (Some(name), Some(registration_type), Some(domain)) = (
        unsafe { callback_string(name) },
        unsafe { callback_string(registration_type) },
        unsafe { callback_string(domain) },
    ) else {
        return;
    };
    callbacks.push(BrowserAction::Browse {
        added: flags & ffi::FLAG_ADD != 0,
        interface_index,
        name,
        registration_type,
        domain,
    });
}

unsafe extern "C" fn resolve_callback(
    service_ref: ffi::ServiceRef,
    _flags: ffi::Flags,
    _interface_index: u32,
    error: ffi::Error,
    fullname: *const c_char,
    hostname: *const c_char,
    port: u16,
    txt_len: u16,
    txt_record: *const u8,
    context: *mut c_void,
) {
    let Some(callbacks) = (unsafe { callback_state(context) }) else {
        return;
    };
    let result = if error != ffi::NO_ERROR {
        Err(error)
    } else {
        let fullname = unsafe { callback_string(fullname) };
        let hostname = unsafe { callback_string(hostname) };
        let txt_len = usize::from(txt_len);
        if let (Some(fullname), Some(hostname), true) =
            (fullname, hostname, txt_len <= MAX_TXT_BYTES)
        {
            let txt = if txt_len == 0 {
                Vec::new()
            } else if txt_record.is_null() {
                callbacks.push(BrowserAction::Resolve {
                    operation: service_ref,
                    result: Err(-1),
                });
                return;
            } else {
                unsafe { std::slice::from_raw_parts(txt_record, txt_len) }.to_vec()
            };
            Ok(ResolvedMetadata {
                fullname,
                hostname,
                port: u16::from_be(port),
                txt,
            })
        } else {
            Err(-1)
        }
    };
    callbacks.push(BrowserAction::Resolve {
        operation: service_ref,
        result,
    });
}

unsafe extern "C" fn address_callback(
    service_ref: ffi::ServiceRef,
    flags: ffi::Flags,
    _interface_index: u32,
    error: ffi::Error,
    _hostname: *const c_char,
    address: *const libc::sockaddr,
    ttl_seconds: u32,
    context: *mut c_void,
) {
    let Some(callbacks) = (unsafe { callback_state(context) }) else {
        return;
    };
    let result = if error == ffi::NO_ERROR {
        Ok(AddressChange {
            added: flags & ffi::FLAG_ADD != 0,
            address: (!address.is_null())
                .then(|| unsafe { socket_address_ip(address) })
                .flatten(),
            ttl_seconds,
        })
    } else {
        Err(error)
    };
    callbacks.push(BrowserAction::Address {
        operation: service_ref,
        result,
    });
}

unsafe fn callback_state<'a>(context: *mut c_void) -> Option<&'a mut BrowserCallbacks> {
    unsafe { context.cast::<BrowserCallbacks>().as_mut() }
}

unsafe fn callback_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(value) }.to_bytes();
    if bytes.len() > MAX_CALLBACK_STRING_BYTES {
        return None;
    }
    std::str::from_utf8(bytes).ok().map(ToOwned::to_owned)
}

#[cfg(test)]
#[path = "discovery_bonjour_browser_tests.rs"]
mod tests;
