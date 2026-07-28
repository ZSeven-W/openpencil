use std::ffi::{c_char, c_int, c_uchar, c_void};

pub(super) type ServiceRef = *mut c_void;
pub(super) type RecordRef = *mut c_void;
pub(super) type Flags = u32;
pub(super) type Error = i32;
pub(super) type Protocol = u32;

pub(super) const NO_ERROR: Error = 0;
pub(super) const FLAG_ADD: Flags = 0x2;
pub(super) const FLAG_NO_AUTO_RENAME: Flags = 0x8;
pub(super) const FLAG_UNIQUE: Flags = 0x20;
pub(super) const FLAG_FORCE_MULTICAST: Flags = 0x400;
pub(super) const FLAG_SHARE_CONNECTION: Flags = 0x4000;
pub(super) const PROTOCOL_IPV4: Protocol = 0x01;
pub(super) const PROTOCOL_IPV6: Protocol = 0x02;
pub(super) const CLASS_IN: u16 = 1;
pub(super) const TYPE_A: u16 = 1;
pub(super) const TYPE_AAAA: u16 = 28;

pub(super) type RegisterReply = unsafe extern "C" fn(
    ServiceRef,
    Flags,
    Error,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
);

pub(super) type RegisterRecordReply =
    unsafe extern "C" fn(ServiceRef, RecordRef, Flags, Error, *mut c_void);

pub(super) type BrowseReply = unsafe extern "C" fn(
    ServiceRef,
    Flags,
    u32,
    Error,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
);

pub(super) type ResolveReply = unsafe extern "C" fn(
    ServiceRef,
    Flags,
    u32,
    Error,
    *const c_char,
    *const c_char,
    u16,
    u16,
    *const c_uchar,
    *mut c_void,
);

pub(super) type AddrInfoReply = unsafe extern "C" fn(
    ServiceRef,
    Flags,
    u32,
    Error,
    *const c_char,
    *const libc::sockaddr,
    u32,
    *mut c_void,
);

#[link(name = "System", kind = "dylib")]
unsafe extern "C" {
    #[link_name = "DNSServiceCreateConnection"]
    pub(super) fn create_connection(service_ref: *mut ServiceRef) -> Error;

    #[link_name = "DNSServiceRegister"]
    pub(super) fn register(
        service_ref: *mut ServiceRef,
        flags: Flags,
        interface_index: u32,
        name: *const c_char,
        registration_type: *const c_char,
        domain: *const c_char,
        host: *const c_char,
        port: u16,
        txt_len: u16,
        txt_record: *const c_void,
        callback: Option<RegisterReply>,
        context: *mut c_void,
    ) -> Error;

    #[link_name = "DNSServiceRegisterRecord"]
    pub(super) fn register_record(
        service_ref: ServiceRef,
        record_ref: *mut RecordRef,
        flags: Flags,
        interface_index: u32,
        fullname: *const c_char,
        record_type: u16,
        record_class: u16,
        data_len: u16,
        data: *const c_void,
        ttl: u32,
        callback: Option<RegisterRecordReply>,
        context: *mut c_void,
    ) -> Error;

    #[link_name = "DNSServiceBrowse"]
    pub(super) fn browse(
        service_ref: *mut ServiceRef,
        flags: Flags,
        interface_index: u32,
        registration_type: *const c_char,
        domain: *const c_char,
        callback: Option<BrowseReply>,
        context: *mut c_void,
    ) -> Error;

    #[link_name = "DNSServiceResolve"]
    pub(super) fn resolve(
        service_ref: *mut ServiceRef,
        flags: Flags,
        interface_index: u32,
        name: *const c_char,
        registration_type: *const c_char,
        domain: *const c_char,
        callback: Option<ResolveReply>,
        context: *mut c_void,
    ) -> Error;

    #[link_name = "DNSServiceGetAddrInfo"]
    pub(super) fn get_addr_info(
        service_ref: *mut ServiceRef,
        flags: Flags,
        interface_index: u32,
        protocol: Protocol,
        hostname: *const c_char,
        callback: Option<AddrInfoReply>,
        context: *mut c_void,
    ) -> Error;

    #[link_name = "DNSServiceRefSockFD"]
    pub(super) fn socket_fd(service_ref: ServiceRef) -> c_int;

    #[link_name = "DNSServiceProcessResult"]
    pub(super) fn process_result(service_ref: ServiceRef) -> Error;

    #[link_name = "DNSServiceRefDeallocate"]
    pub(super) fn deallocate(service_ref: ServiceRef);
}
