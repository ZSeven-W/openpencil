use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_void, CString};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ptr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mdns_sd::{IfKind, ServiceInfo};

use super::bonjour_ffi as ffi;
use super::{
    is_usable_address, property_value, renamed_instance, DiscoveryError, PublisherBackend,
    MAX_DISCOVERY_ADDRESSES,
};

const REGISTRATION_TYPE: &[u8] = b"_openpencil-collab._tcp\0";
const LOCAL_DOMAIN: &[u8] = b"local.\0";
const HOST_RECORD_TTL: u32 = 120;
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const REGISTRATION_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct BonjourPublisherBackend {
    disabled_family: Option<IfKind>,
    worker: Option<Worker>,
}

impl BonjourPublisherBackend {
    pub(super) const fn new(disabled_family: Option<IfKind>) -> Self {
        Self {
            disabled_family,
            worker: None,
        }
    }
}

impl PublisherBackend for BonjourPublisherBackend {
    fn register(&mut self, service: ServiceInfo) -> Result<(), DiscoveryError> {
        if self.worker.is_some() {
            return Err(DiscoveryError::InvalidMetadata);
        }
        let specification =
            PublisherSpecification::from_service(&service, self.disabled_family.as_ref())?;
        let (stop_sender, stop_receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let join = thread::Builder::new()
            .name("op-bonjour-publisher".to_owned())
            .spawn(move || publisher_worker(specification, stop_receiver, ready_sender))
            .map_err(DiscoveryError::BonjourIo)?;
        let worker = Worker {
            stop_sender,
            join: Some(join),
        };
        match ready_receiver.recv_timeout(REGISTRATION_TIMEOUT) {
            Ok(Ok(())) => {
                self.worker = Some(worker);
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = worker.stop();
                Err(DiscoveryError::Bonjour(error))
            }
            Err(_) => {
                let _ = worker.stop();
                Err(DiscoveryError::BonjourWorkerStopped)
            }
        }
    }

    fn unregister(&mut self, _service: &ServiceInfo) -> Result<(), DiscoveryError> {
        let worker = self.worker.take().ok_or(DiscoveryError::UnregisterFailed)?;
        worker.stop().map_err(|_| DiscoveryError::UnregisterFailed)
    }

    fn shutdown(&mut self) -> Result<(), DiscoveryError> {
        match self.worker.take() {
            Some(worker) => worker.stop(),
            None => Ok(()),
        }
    }
}

struct Worker {
    stop_sender: Sender<()>,
    join: Option<JoinHandle<Result<(), DiscoveryError>>>,
}

impl Worker {
    fn stop(mut self) -> Result<(), DiscoveryError> {
        let _ = self.stop_sender.send(());
        let join = self
            .join
            .take()
            .ok_or(DiscoveryError::BonjourWorkerStopped)?;
        join.join()
            .map_err(|_| DiscoveryError::BonjourWorkerStopped)?
    }
}

pub(super) struct DnsConnection {
    pub(super) raw: ffi::ServiceRef,
}

impl DnsConnection {
    pub(super) fn create() -> Result<Self, DiscoveryError> {
        let mut raw = ptr::null_mut();
        check(unsafe { ffi::create_connection(&mut raw) })?;
        if raw.is_null() {
            return Err(DiscoveryError::BonjourWorkerStopped);
        }
        Ok(Self { raw })
    }

    pub(super) fn socket_fd(&self) -> Result<i32, DiscoveryError> {
        let descriptor = unsafe { ffi::socket_fd(self.raw) };
        if descriptor < 0 {
            Err(DiscoveryError::BonjourWorkerStopped)
        } else {
            Ok(descriptor)
        }
    }
}

impl Drop for DnsConnection {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { ffi::deallocate(self.raw) };
            self.raw = ptr::null_mut();
        }
    }
}

struct PublisherSpecification {
    name: CString,
    hostname: CString,
    port: u16,
    txt: Vec<u8>,
    interfaces: Vec<InterfaceAdvertisement>,
}

struct InterfaceAdvertisement {
    index: u32,
    addresses: Vec<IpAddr>,
}

impl PublisherSpecification {
    fn from_service(
        service: &ServiceInfo,
        disabled_family: Option<&IfKind>,
    ) -> Result<Self, DiscoveryError> {
        let name = CString::new(renamed_instance(service.get_fullname())?)
            .map_err(|_| DiscoveryError::InvalidMetadata)?;
        let hostname =
            CString::new(service.get_hostname()).map_err(|_| DiscoveryError::InvalidMetadata)?;
        let txt = encode_txt(service)?;
        let interfaces = interface_advertisements(service, disabled_family)?;
        Ok(Self {
            name,
            hostname,
            port: service.get_port(),
            txt,
            interfaces,
        })
    }
}

struct PublisherCallbackState {
    pending: usize,
    error: Option<i32>,
}

struct PublisherCallback {
    state: *mut PublisherCallbackState,
    completed: bool,
}

fn publisher_worker(
    specification: PublisherSpecification,
    stop: Receiver<()>,
    ready: mpsc::SyncSender<Result<(), i32>>,
) -> Result<(), DiscoveryError> {
    let mut callback_state = Box::new(PublisherCallbackState {
        pending: 0,
        error: None,
    });
    let state_pointer = (&mut *callback_state) as *mut PublisherCallbackState;
    let mut callbacks = Vec::<Box<PublisherCallback>>::new();
    let connection = DnsConnection::create()?;

    for interface in &specification.interfaces {
        callbacks.push(Box::new(PublisherCallback {
            state: state_pointer,
            completed: false,
        }));
        callback_state.pending += 1;
        let context = callback_pointer(&mut callbacks);
        let mut service_ref = connection.raw;
        check(unsafe {
            ffi::register(
                &mut service_ref,
                ffi::FLAG_SHARE_CONNECTION | ffi::FLAG_NO_AUTO_RENAME,
                interface.index,
                specification.name.as_ptr(),
                REGISTRATION_TYPE.as_ptr().cast(),
                LOCAL_DOMAIN.as_ptr().cast(),
                specification.hostname.as_ptr(),
                specification.port.to_be(),
                u16::try_from(specification.txt.len())
                    .map_err(|_| DiscoveryError::InvalidMetadata)?,
                specification.txt.as_ptr().cast(),
                Some(register_callback),
                context,
            )
        })?;

        for address in &interface.addresses {
            callbacks.push(Box::new(PublisherCallback {
                state: state_pointer,
                completed: false,
            }));
            callback_state.pending += 1;
            let context = callback_pointer(&mut callbacks);
            register_address_record(
                connection.raw,
                interface.index,
                &specification.hostname,
                *address,
                context,
            )?;
        }
    }

    let descriptor = connection.socket_fd()?;
    let mut ready = Some(ready);
    loop {
        if stop.try_recv().is_ok() {
            return Ok(());
        }
        poll_connection(&connection, descriptor)?;
        if let Some(error) = callback_state.error {
            if let Some(sender) = ready.take() {
                let _ = sender.send(Err(error));
            }
            return Err(DiscoveryError::Bonjour(error));
        }
        if callback_state.pending == 0 {
            if let Some(sender) = ready.take() {
                let _ = sender.send(Ok(()));
            }
        }
    }
}

fn callback_pointer(callbacks: &mut [Box<PublisherCallback>]) -> *mut c_void {
    callbacks
        .last_mut()
        .map(|callback| (&mut **callback as *mut PublisherCallback).cast())
        .unwrap_or(ptr::null_mut())
}

fn register_address_record(
    connection: ffi::ServiceRef,
    interface_index: u32,
    hostname: &CString,
    address: IpAddr,
    context: *mut c_void,
) -> Result<(), DiscoveryError> {
    let mut record_ref = ptr::null_mut();
    let (record_type, bytes): (u16, Vec<u8>) = match address {
        IpAddr::V4(address) => (ffi::TYPE_A, address.octets().to_vec()),
        IpAddr::V6(address) => (ffi::TYPE_AAAA, address.octets().to_vec()),
    };
    check(unsafe {
        ffi::register_record(
            connection,
            &mut record_ref,
            ffi::FLAG_UNIQUE | ffi::FLAG_FORCE_MULTICAST,
            interface_index,
            hostname.as_ptr(),
            record_type,
            ffi::CLASS_IN,
            u16::try_from(bytes.len()).unwrap_or(0),
            bytes.as_ptr().cast(),
            HOST_RECORD_TTL,
            Some(register_record_callback),
            context,
        )
    })
}

unsafe extern "C" fn register_callback(
    _service_ref: ffi::ServiceRef,
    flags: ffi::Flags,
    error: ffi::Error,
    _name: *const std::ffi::c_char,
    _registration_type: *const std::ffi::c_char,
    _domain: *const std::ffi::c_char,
    context: *mut c_void,
) {
    unsafe { complete_publisher_callback(context, error, flags & ffi::FLAG_ADD != 0) };
}

unsafe extern "C" fn register_record_callback(
    _service_ref: ffi::ServiceRef,
    _record_ref: ffi::RecordRef,
    _flags: ffi::Flags,
    error: ffi::Error,
    context: *mut c_void,
) {
    unsafe { complete_publisher_callback(context, error, true) };
}

unsafe fn complete_publisher_callback(context: *mut c_void, error: i32, added: bool) {
    let Some(callback) = (unsafe { context.cast::<PublisherCallback>().as_mut() }) else {
        return;
    };
    let Some(state) = (unsafe { callback.state.as_mut() }) else {
        return;
    };
    if error != ffi::NO_ERROR || !added {
        state.error = Some(if error == ffi::NO_ERROR { -1 } else { error });
        return;
    }
    if !callback.completed {
        callback.completed = true;
        state.pending = state.pending.saturating_sub(1);
    }
}

fn encode_txt(service: &ServiceInfo) -> Result<Vec<u8>, DiscoveryError> {
    let mut encoded = Vec::with_capacity(64);
    for key in ["id", "v", "p"] {
        let entry = format!("{key}={}", property_value(service, key)?);
        let length = u8::try_from(entry.len()).map_err(|_| DiscoveryError::InvalidMetadata)?;
        encoded.push(length);
        encoded.extend_from_slice(entry.as_bytes());
    }
    Ok(encoded)
}

fn interface_advertisements(
    service: &ServiceInfo,
    disabled_family: Option<&IfKind>,
) -> Result<Vec<InterfaceAdvertisement>, DiscoveryError> {
    let local = local_interface_addresses()?;
    let explicit: BTreeSet<_> = service.get_addresses().iter().copied().collect();
    let mut selected = BTreeSet::new();

    if explicit.is_empty() {
        for candidate in &local {
            if candidate.multicast
                && !candidate.loopback
                && !family_is_disabled(candidate.address, disabled_family)
            {
                selected.insert((candidate.index, candidate.address));
            }
        }
    } else {
        for address in &explicit {
            if family_is_disabled(*address, disabled_family) {
                return Err(DiscoveryError::InvalidAddress);
            }
            let matches: Vec<_> = local
                .iter()
                .filter(|candidate| candidate.address == *address)
                .map(|candidate| (candidate.index, candidate.address))
                .collect();
            if matches.is_empty() {
                return Err(DiscoveryError::InvalidAddress);
            }
            selected.extend(matches);
        }
    }

    let unique_addresses: BTreeSet<_> = selected.iter().map(|(_, address)| *address).collect();
    if unique_addresses.is_empty() || unique_addresses.len() > MAX_DISCOVERY_ADDRESSES {
        return Err(DiscoveryError::InvalidAddress);
    }
    let mut by_interface = BTreeMap::<u32, Vec<IpAddr>>::new();
    for (index, address) in selected {
        by_interface.entry(index).or_default().push(address);
    }
    Ok(by_interface
        .into_iter()
        .map(|(index, mut addresses)| {
            addresses.sort_unstable();
            addresses.dedup();
            InterfaceAdvertisement { index, addresses }
        })
        .collect())
}

fn family_is_disabled(address: IpAddr, disabled_family: Option<&IfKind>) -> bool {
    matches!(
        (address, disabled_family),
        (IpAddr::V4(_), Some(IfKind::IPv4)) | (IpAddr::V6(_), Some(IfKind::IPv6))
    )
}

struct LocalInterfaceAddress {
    index: u32,
    address: IpAddr,
    multicast: bool,
    loopback: bool,
}

struct IfAddrs(*mut libc::ifaddrs);

impl Drop for IfAddrs {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { libc::freeifaddrs(self.0) };
        }
    }
}

fn local_interface_addresses() -> Result<Vec<LocalInterfaceAddress>, DiscoveryError> {
    let mut head = ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Err(DiscoveryError::BonjourIo(io::Error::last_os_error()));
    }
    let guard = IfAddrs(head);
    let mut current = guard.0;
    let mut addresses = Vec::new();
    while !current.is_null() {
        let interface = unsafe { &*current };
        current = interface.ifa_next;
        if interface.ifa_addr.is_null()
            || interface.ifa_name.is_null()
            || interface.ifa_flags & u32::try_from(libc::IFF_UP).unwrap_or(0) == 0
        {
            continue;
        }
        let index = unsafe { libc::if_nametoindex(interface.ifa_name) };
        if index == 0 {
            continue;
        }
        let Some(address) = (unsafe { socket_address_ip(interface.ifa_addr) }) else {
            continue;
        };
        if !is_usable_address(&address) {
            continue;
        }
        addresses.push(LocalInterfaceAddress {
            index,
            address,
            multicast: interface.ifa_flags & u32::try_from(libc::IFF_MULTICAST).unwrap_or(0) != 0,
            loopback: interface.ifa_flags & u32::try_from(libc::IFF_LOOPBACK).unwrap_or(0) != 0,
        });
    }
    addresses.sort_by_key(|candidate| (candidate.index, candidate.address));
    addresses.dedup_by_key(|candidate| (candidate.index, candidate.address));
    Ok(addresses)
}

pub(super) unsafe fn socket_address_ip(address: *const libc::sockaddr) -> Option<IpAddr> {
    if address.is_null() {
        return None;
    }
    let family = unsafe { (*address).sa_family as i32 };
    match family {
        libc::AF_INET => {
            let address = unsafe { &*address.cast::<libc::sockaddr_in>() };
            Some(IpAddr::V4(Ipv4Addr::from(
                address.sin_addr.s_addr.to_ne_bytes(),
            )))
        }
        libc::AF_INET6 => {
            let address = unsafe { &*address.cast::<libc::sockaddr_in6>() };
            Some(IpAddr::V6(Ipv6Addr::from(address.sin6_addr.s6_addr)))
        }
        _ => None,
    }
}

pub(super) fn poll_connection(
    connection: &DnsConnection,
    descriptor: i32,
) -> Result<(), DiscoveryError> {
    let mut descriptor = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: 0,
    };
    let timeout = i32::try_from(POLL_INTERVAL.as_millis()).unwrap_or(50);
    let result = unsafe { libc::poll(&mut descriptor, 1, timeout) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return Ok(());
        }
        return Err(DiscoveryError::BonjourIo(error));
    }
    if result > 0 && descriptor.revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP) != 0 {
        check(unsafe { ffi::process_result(connection.raw) })?;
    }
    Ok(())
}

pub(super) fn check(error: i32) -> Result<(), DiscoveryError> {
    if error == ffi::NO_ERROR {
        Ok(())
    } else {
        Err(DiscoveryError::Bonjour(error))
    }
}
