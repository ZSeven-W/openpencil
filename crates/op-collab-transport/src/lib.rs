//! Native Noise/TCP transport for OpenPencil collaboration.
//!
//! The protocol, framing, admission rules, resource limits, key-store fallback,
//! and discovery metadata are public. Production credentials and ticket signing
//! material are injected by the host and never enter this crate.

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[cfg(all(target_arch = "wasm32", feature = "native"))]
compile_error!("op-collab-transport's `native` feature is not available on wasm32");

mod admission;
#[cfg(all(feature = "native", target_os = "ios"))]
mod apple_key_store;
mod chunk;
mod config;
mod connection_limit;
mod credential_store;
#[cfg(feature = "mdns")]
mod discovery;
mod driver;
mod driver_io;
mod error;
mod frame;
mod key_store;
mod noise;
mod os_key_store;
mod prelude;
mod queue;
mod reassembly_budget;
mod record;
mod runtime;
mod tcp;
#[cfg(all(feature = "native", target_os = "windows"))]
mod windows_key_store;

pub use admission::{
    verify_initial_ticket, verify_renewal_ticket, AdmissionHello, AdmissionIdentity,
    AdmissionPhase, AdmissionState, JoinIntent, PeerIdentityPolicy, ResumeHint, TicketVerifier,
    VerifiedTicketClaims,
};
pub use chunk::{
    ChunkHeader, CompletedTransfer, Reassembler, TransferChunk, TransferChunkIter, TransferClass,
    CHUNK_HEADER_BYTES, MAX_CHUNK_PAYLOAD,
};
pub use config::{
    m1_wire_limits, ConnectionLimits, RateLimitConfig, TimeoutConfig, TransportConfig,
    DEFAULT_GLOBAL_INBOUND_REASSEMBLY_BYTES, DEFAULT_MAX_PENDING_HANDSHAKES,
    DEFAULT_MAX_PENDING_HANDSHAKES_PER_IP, MAX_CONTROL_TRANSFER_BYTES, MAX_NOISE_CIPHERTEXT_BYTES,
    MAX_NOISE_PLAINTEXT_BYTES, MAX_SNAPSHOT_TRANSFER_BYTES, MAX_TICKET_BYTES,
    MAX_TICKET_TRANSFER_BYTES, MAX_TXN_TRANSFER_BYTES,
};
pub use connection_limit::{
    ActiveConnectionGuard, ConnectionCounts, ConnectionLimitError, ConnectionLimiter,
    PendingHandshakeGuard,
};
pub use credential_store::{CredentialStore, CredentialStoreKeyStore};
#[cfg(feature = "mdns")]
pub use discovery::{
    DiscoveredSession, DiscoveryBrowser, DiscoveryError, DiscoveryPublisher,
    DISCOVERY_SERVICE_TYPE, MAX_DISCOVERY_ADDRESSES,
};
pub use driver::{ConnectionDriver, DriverEvent, DriverPoll, InboundTransferPolicy};
pub use driver_io::IncrementalRecordDecoder;
pub use error::{
    AdmissionError, ChunkError, ConfigError, FrameTransportError, KeyStoreError,
    NoiseTransportError, PreludeError, QueueError, RecordError, RuntimeError,
    RuntimeErrorDiagnostic, RuntimeErrorPhase,
};
pub use frame::{
    decode_frame_transfer, encode_frame_transfer, frame_transfer_class, EncodedFrameTransfer,
};
pub use key_store::{DeviceStaticKey, FileKeyStore, StaticKeyStore};
pub use noise::{run_xx_initiator, run_xx_responder, NoiseSession, NOISE_PROTOCOL_NAME};
pub use os_key_store::OsKeyStore;
pub use prelude::{
    read_server_prelude, write_server_prelude, EncodedServerPrelude, ServerPrelude,
    TRANSPORT_PROTOCOL_VERSION,
};
pub use queue::{SharedQueueBudget, SharedQueueReservation, TokenBucket};
pub use reassembly_budget::{SharedReassemblyBudget, SharedReassemblyReservation};
pub use record::{
    decrypt_record, encrypt_record, read_ciphertext_record, write_ciphertext_record,
    TRANSPORT_HEARTBEAT_PLAINTEXT,
};
pub use runtime::{SecureConnection, TransportTransfer};
pub use tcp::{
    accept_secure_tcp, accept_secure_tcp_guarded, connect_manual, connect_secure_tcp,
    connect_secure_tcp_until, connect_secure_tcp_until_cancellable, prepare_tcp_stream,
};
