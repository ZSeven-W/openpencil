use std::time::Duration;

use crate::TransferClass;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    #[error("transport configuration field `{field}` is outside its safe range")]
    InvalidValue { field: &'static str },
}

#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("record IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("record length {actual} is outside 1..={maximum}")]
    InvalidLength { actual: usize, maximum: usize },
    #[error("Noise record encryption failed: {0}")]
    Encrypt(#[source] snow::Error),
    #[error("Noise record authentication failed: {0}")]
    Decrypt(#[source] snow::Error),
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ChunkError {
    #[error("authenticated chunk header has invalid length {0}")]
    HeaderLength(usize),
    #[error("authenticated chunk header version {0} is unsupported")]
    UnsupportedVersion(u8),
    #[error("authenticated chunk header contains non-zero reserved bits")]
    ReservedBits,
    #[error("authenticated chunk class {0} is unknown")]
    UnknownClass(u8),
    #[error("transfer id must be non-zero")]
    ZeroTransferId,
    #[error("transfer length {actual} exceeds {class:?} limit {maximum}")]
    TransferTooLarge {
        class: TransferClass,
        actual: usize,
        maximum: usize,
    },
    #[error("transfer length must be non-zero")]
    EmptyTransfer,
    #[error("chunk count {actual} does not match expected {expected}")]
    InvalidChunkCount { actual: u32, expected: u32 },
    #[error("chunk index {actual} does not match expected {expected}")]
    UnexpectedChunkIndex { actual: u32, expected: u32 },
    #[error("chunk belongs to a different in-flight transfer")]
    InFlightMismatch,
    #[error("transfer id {actual} is not newer than completed id {previous}")]
    ReplayedTransferId { actual: u64, previous: u64 },
    #[error("chunk payload length {actual} does not match expected {expected}")]
    InvalidPayloadLength { actual: usize, expected: usize },
    #[error("transfer timed out after {0:?}")]
    TimedOut(Duration),
    #[error("transfer length arithmetic overflow")]
    LengthOverflow,
}

#[derive(Debug, thiserror::Error)]
pub enum PreludeError {
    #[error("server prelude IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("server prelude is truncated or malformed")]
    Malformed,
    #[error("server prelude magic is invalid")]
    Magic,
    #[error("transport protocol version {actual} is unsupported; expected {expected}")]
    UnsupportedTransportVersion { actual: u16, expected: u16 },
    #[error("collaboration protocol version {actual} is unsupported; expected {expected}")]
    UnsupportedCollabVersion { actual: u16, expected: u16 },
    #[error("server prelude field `{field}` is empty or too long")]
    InvalidIdentifier { field: &'static str },
    #[error("server prelude is {actual} bytes; maximum is {maximum}")]
    TooLarge { actual: usize, maximum: usize },
    #[error("server prelude is not valid UTF-8")]
    Utf8,
}

#[derive(Debug, thiserror::Error)]
pub enum NoiseTransportError {
    #[error("Noise configuration failed: {0}")]
    Configuration(#[source] snow::Error),
    #[error("Noise handshake IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Noise handshake failed: {0}")]
    Handshake(#[source] snow::Error),
    #[error("Noise handshake frame has invalid length {0}")]
    HandshakeFrameLength(usize),
    #[error("Noise handshake did not expose a 32-byte remote static key")]
    MissingRemoteStatic,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameTransportError {
    #[error("collaboration frame codec rejected the transfer: {0}")]
    Protocol(#[from] op_collab::ProtocolError),
    #[error("encoded {class:?} frame is {actual} bytes; maximum is {maximum}")]
    TransferTooLarge {
        class: TransferClass,
        actual: usize,
        maximum: usize,
    },
    #[error("declared {declared:?} transfer carries a {actual:?} frame")]
    ClassMismatch {
        declared: TransferClass,
        actual: TransferClass,
    },
    #[error("pre-encoded frame was validated under different wire limits")]
    WireLimitsMismatch,
    #[error("sensitive Ticket transfer cannot use shared or coalescing storage")]
    SensitiveTransferNotShareable,
    #[error("Ticket transfer does not carry a RenewTicket frame")]
    TicketFrameExpected,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    #[error("ticket verification failed")]
    Verification,
    #[error("ticket issuer does not match the authenticated session owner")]
    WrongIssuer,
    #[error("ticket subject does not match the authenticated session owner")]
    WrongSubject,
    #[error("ticket X25519 binding does not match the Noise remote static")]
    StaticKeyMismatch,
    #[error("ticket has expired")]
    Expired,
    #[error("ticket renewal changed the authenticated identity")]
    RenewalIdentityChanged,
    #[error("ticket renewal did not extend the expiry")]
    RenewalDidNotExtend,
    #[error("admission state transition is invalid")]
    InvalidState,
    #[error("the admission deadline expired")]
    AdmissionTimedOut,
    #[error("the authenticated ticket reached its expiry")]
    TicketExpired,
    #[error("admission payload is malformed")]
    Malformed,
    #[error("admission payload is {actual} bytes; maximum is {maximum}")]
    TooLarge { actual: usize, maximum: usize },
    #[error("admission field `{field}` is empty or too long")]
    InvalidIdentifier { field: &'static str },
}

#[derive(Debug, thiserror::Error)]
pub enum KeyStoreError {
    #[error("secure file key storage is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("the operating-system credential store is unavailable")]
    PlatformStoreUnavailable,
    #[error("the operating-system credential store is locked or access was denied")]
    PlatformStoreAccessDenied,
    #[error("the operating-system credential store operation failed")]
    PlatformStoreFailure,
    #[error("the operating-system credential store entry is malformed or ambiguous")]
    UnsafePlatformEntry,
    #[error("device static key IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("device static key directory is a symlink or not a directory")]
    UnsafeDirectory,
    #[error("device static key path is a symlink or not a regular file")]
    UnsafeFile,
    #[error("device static key has invalid length {0}")]
    InvalidLength(usize),
    #[error("device static key is all zero")]
    AllZero,
    #[error("secure random generation failed")]
    Random,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum QueueError {
    #[error("bounded transfer queue is full")]
    Full,
    #[error("queued byte budget would be exceeded")]
    ByteBudget,
    #[error("queue item is larger than the configured byte budget")]
    ItemTooLarge,
    #[error("shared queue budget is unavailable")]
    Unavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    InvalidConfig(#[from] ConfigError),
    #[error("TCP connection failed: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Noise(#[from] NoiseTransportError),
    #[error(transparent)]
    Prelude(#[from] PreludeError),
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error(transparent)]
    Chunk(#[from] ChunkError),
    #[error(transparent)]
    Frame(#[from] FrameTransportError),
    #[error(transparent)]
    Admission(#[from] AdmissionError),
    #[error(transparent)]
    Queue(#[from] QueueError),
    #[error("expected {expected:?} transfer, received {actual:?}")]
    UnexpectedClass {
        expected: TransferClass,
        actual: TransferClass,
    },
    #[error("authenticated inbound {0:?} transfer is forbidden in this connection direction")]
    ForbiddenInboundClass(TransferClass),
    #[error("outbound transfer id space is exhausted")]
    TransferIdExhausted,
    #[error("connection rate limit exceeded")]
    RateLimited,
    #[error("the server prelude discovery id does not match the selected endpoint")]
    DiscoveryIdMismatch,
    #[error("the connection was idle past its configured deadline")]
    IdleTimeout,
    #[error("a partial ciphertext record made no read progress before its deadline")]
    ReadTimeout,
    #[error("an encrypted record made no write progress before its deadline")]
    WriteTimeout,
    #[error("the secure connection is no longer usable after a fatal error")]
    ConnectionClosed,
}
