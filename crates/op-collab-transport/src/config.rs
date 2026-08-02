use std::time::Duration;

use op_collab::{
    WireLimits, MAX_DOCUMENT_NODES, MAX_ENVELOPE_BYTES, MAX_IDENTIFIER_BYTES, MAX_OPS_PER_TXN,
    MAX_PROCESSED_SUBTREE_NODES_PER_OP, MAX_TREE_DEPTH, MAX_VALIDATION_NODE_VISITS_PER_TXN,
};

use crate::ConfigError;

pub const MAX_NOISE_PLAINTEXT_BYTES: usize = 60 * 1024;
pub const NOISE_AEAD_TAG_BYTES: usize = 16;
pub const MAX_NOISE_CIPHERTEXT_BYTES: usize = MAX_NOISE_PLAINTEXT_BYTES + NOISE_AEAD_TAG_BYTES;

pub const MAX_CONTROL_TRANSFER_BYTES: usize = 64 * 1024;
pub const MAX_TICKET_TRANSFER_BYTES: usize = 64 * 1024;
pub const MAX_TICKET_BYTES: usize = 48 * 1024;
pub const MAX_TXN_TRANSFER_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SNAPSHOT_TRANSFER_BYTES: usize = 64 * 1024 * 1024;

/// Leaves room for escaped session/participant/peer identifiers and the
/// `PresenceChanged` envelope inside a 64 KiB Control transfer.
pub const M1_MAX_PRESENCE_BYTES: u32 = 44 * 1024;
/// Leaves room for the Submit/Commit envelope around the encoded transaction.
pub const M1_MAX_TXN_BODY_BYTES: u32 = (MAX_TXN_TRANSFER_BYTES - 32 * 1024) as u32;

/// Global ceiling on concurrent pending handshakes.
///
/// A pending seat is taken before any cryptographic work and is only converted
/// into an active connection after ticket admission, so the ceiling has to keep
/// honest joins working while unauthenticated peers are connecting. The cost of
/// one pending seat is bounded by the accepted socket plus the largest buffers a
/// pre-admission connection can own: one Noise record
/// (`MAX_NOISE_CIPHERTEXT_BYTES`, 60 KiB + 16 B) and one reassembling Ticket
/// transfer (`MAX_TICKET_TRANSFER_BYTES`, 64 KiB), i.e. under 128 KiB. At this
/// ceiling the worst case is 128 seats x 128 KiB = 16 MiB held for at most
/// `timeouts.handshake` (5 s), and — because a seat is also capped per source
/// address — filling it needs 128 / `DEFAULT_MAX_PENDING_HANDSHAKES_PER_IP` = 32
/// distinct addresses instead of the 4 the previous 16-seat ceiling needed.
///
/// The ceiling alone is not the defence: guarded owner accepts enforce
/// `timeouts.handshake_first_message` as the actual socket read deadline. A
/// silent connection must exit and drop its continuously charged guard before
/// its global seat is released. See [`crate::ConnectionLimiter`].
pub const DEFAULT_MAX_PENDING_HANDSHAKES: usize = 128;
pub const DEFAULT_MAX_PENDING_HANDSHAKES_PER_IP: usize = 4;
pub const DEFAULT_MAX_ACTIVE_CONNECTIONS: usize = 64;
pub const DEFAULT_OUTBOUND_QUEUE_ITEMS: usize = 8;
pub const DEFAULT_GLOBAL_QUEUED_BYTES: usize = 256 * 1024 * 1024;
/// Aggregate ceiling for inbound reassembly buffers across every connection.
///
/// Inbound transfers allocate the peer-declared total on chunk 0, so the
/// per-class caps alone bound only one connection at a time: 64 authenticated
/// peers x `MAX_TXN_TRANSFER_BYTES` (4 MiB) is 256 MiB of buffer no
/// per-connection limit can see. This aggregate admits one full owner snapshot
/// (`MAX_SNAPSHOT_TRANSFER_BYTES`, 64 MiB) plus 16 concurrent maximum-size
/// transactions (16 x 4 MiB) and refuses the rest with a typed error, holding
/// transport-owned inbound heap to half the outbound aggregate
/// (`DEFAULT_GLOBAL_QUEUED_BYTES`).
pub const DEFAULT_GLOBAL_INBOUND_REASSEMBLY_BYTES: usize = 128 * 1024 * 1024;
// The aggregate has to admit one full owner snapshot or a legitimate first
// sync would be refused by its own budget, and it must stay under the outbound
// aggregate so inbound buffering cannot become the larger of the two. Both are
// relations between constants, so they belong at compile time — a runtime test
// would only assert what the compiler already knows.
const _: () = assert!(DEFAULT_GLOBAL_INBOUND_REASSEMBLY_BYTES >= MAX_SNAPSHOT_TRANSFER_BYTES);
const _: () = assert!(DEFAULT_GLOBAL_INBOUND_REASSEMBLY_BYTES <= DEFAULT_GLOBAL_QUEUED_BYTES);
pub const MAX_CONFIGURED_CONNECTIONS: usize = 256;
pub const MAX_CONFIGURED_QUEUE_ITEMS: usize = 1_024;
pub const MAX_CONFIGURED_QUEUE_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_CONFIGURED_BYTES_PER_SECOND: u64 = 128 * 1024 * 1024;
pub const MAX_CONFIGURED_BYTE_BURST: u64 = 128 * 1024 * 1024;
pub const MAX_CONFIGURED_RECORDS_PER_SECOND: u64 = 8_192;
pub const MAX_CONFIGURED_RECORD_BURST: u64 = 8_192;
pub const MAX_CONFIGURED_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);

pub fn m1_wire_limits() -> WireLimits {
    WireLimits {
        max_presence_bytes: M1_MAX_PRESENCE_BYTES,
        max_txn_bytes: M1_MAX_TXN_BODY_BYTES,
        ..WireLimits::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutConfig {
    pub connect: Duration,
    pub handshake: Duration,
    /// Actual socket-read deadline in which a freshly accepted connection must
    /// produce its first valid handshake message. The connection is closed and
    /// its continuously held pending seat is released on expiry; after valid
    /// progress, the remaining handshake uses the full `handshake` deadline.
    pub handshake_first_message: Duration,
    pub admission: Duration,
    pub ordinary_transfer: Duration,
    pub snapshot_transfer: Duration,
    pub read_write: Duration,
    pub heartbeat: Duration,
    pub idle: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(5),
            handshake: Duration::from_secs(5),
            // One round trip after TCP establishment is enough for an honest
            // directly accepted initiator, while keeping a silent peer's hold
            // on a global pending seat to a fifth of the handshake window.
            // Authenticated relay bridges may widen this per accepted stream.
            handshake_first_message: Duration::from_secs(1),
            admission: Duration::from_secs(10),
            ordinary_transfer: Duration::from_secs(10),
            snapshot_transfer: Duration::from_secs(60),
            read_write: Duration::from_secs(15),
            heartbeat: Duration::from_secs(30),
            idle: Duration::from_secs(120),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionLimits {
    pub max_pending_handshakes: usize,
    pub max_pending_handshakes_per_ip: usize,
    pub max_active_connections: usize,
    pub outbound_queue_items: usize,
    pub global_queued_bytes: usize,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_pending_handshakes: DEFAULT_MAX_PENDING_HANDSHAKES,
            max_pending_handshakes_per_ip: DEFAULT_MAX_PENDING_HANDSHAKES_PER_IP,
            max_active_connections: DEFAULT_MAX_ACTIVE_CONNECTIONS,
            outbound_queue_items: DEFAULT_OUTBOUND_QUEUE_ITEMS,
            global_queued_bytes: DEFAULT_GLOBAL_QUEUED_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    pub bytes_per_second: u64,
    pub byte_burst: u64,
    pub records_per_second: u64,
    pub record_burst: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            bytes_per_second: 8 * 1024 * 1024,
            // A maximum snapshot also carries one authenticated 24-byte
            // header per record, so leave bounded headroom above its body cap.
            byte_burst: (MAX_SNAPSHOT_TRANSFER_BYTES + MAX_CONTROL_TRANSFER_BYTES) as u64,
            records_per_second: 512,
            record_burst: 2_048,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportConfig {
    pub wire_limits: WireLimits,
    pub timeouts: TimeoutConfig,
    pub connections: ConnectionLimits,
    pub rate: RateLimitConfig,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            wire_limits: m1_wire_limits(),
            timeouts: TimeoutConfig::default(),
            connections: ConnectionLimits::default(),
            rate: RateLimitConfig::default(),
        }
    }
}

impl TransportConfig {
    pub fn validate(self) -> Result<Self, ConfigError> {
        let wire = self.wire_limits;
        for (field, actual, maximum) in [
            (
                "wire.max_envelope_bytes",
                wire.max_envelope_bytes,
                MAX_ENVELOPE_BYTES,
            ),
            (
                "wire.max_txn_bytes",
                wire.max_txn_bytes,
                M1_MAX_TXN_BODY_BYTES,
            ),
            (
                "wire.max_ops_per_txn",
                wire.max_ops_per_txn,
                MAX_OPS_PER_TXN,
            ),
            (
                "wire.max_processed_subtree_nodes_per_op",
                wire.max_processed_subtree_nodes_per_op,
                MAX_PROCESSED_SUBTREE_NODES_PER_OP,
            ),
            (
                "wire.max_document_nodes",
                wire.max_document_nodes,
                MAX_DOCUMENT_NODES,
            ),
            ("wire.max_tree_depth", wire.max_tree_depth, MAX_TREE_DEPTH),
            (
                "wire.max_identifier_bytes",
                wire.max_identifier_bytes,
                MAX_IDENTIFIER_BYTES,
            ),
            (
                "wire.max_presence_bytes",
                wire.max_presence_bytes,
                M1_MAX_PRESENCE_BYTES,
            ),
            (
                "wire.max_validation_node_visits_per_txn",
                wire.max_validation_node_visits_per_txn,
                MAX_VALIDATION_NODE_VISITS_PER_TXN,
            ),
        ] {
            if actual == 0 || actual > maximum {
                return Err(ConfigError::InvalidValue { field });
            }
        }
        let connections = self.connections;
        if connections.max_pending_handshakes == 0
            || connections.max_pending_handshakes > MAX_CONFIGURED_CONNECTIONS
        {
            return Err(ConfigError::InvalidValue {
                field: "max_pending_handshakes",
            });
        }
        if connections.max_pending_handshakes_per_ip == 0
            || connections.max_pending_handshakes_per_ip > connections.max_pending_handshakes
        {
            return Err(ConfigError::InvalidValue {
                field: "max_pending_handshakes_per_ip",
            });
        }
        if connections.max_active_connections == 0
            || connections.max_active_connections > MAX_CONFIGURED_CONNECTIONS
        {
            return Err(ConfigError::InvalidValue {
                field: "max_active_connections",
            });
        }
        if connections.outbound_queue_items == 0
            || connections.outbound_queue_items > MAX_CONFIGURED_QUEUE_ITEMS
        {
            return Err(ConfigError::InvalidValue {
                field: "outbound_queue_items",
            });
        }
        if connections.global_queued_bytes < MAX_SNAPSHOT_TRANSFER_BYTES
            || connections.global_queued_bytes > MAX_CONFIGURED_QUEUE_BYTES
        {
            return Err(ConfigError::InvalidValue {
                field: "global_queued_bytes",
            });
        }
        if self.rate.bytes_per_second == 0
            || self.rate.bytes_per_second > MAX_CONFIGURED_BYTES_PER_SECOND
            || self.rate.byte_burst < MAX_NOISE_PLAINTEXT_BYTES as u64
            || self.rate.byte_burst > MAX_CONFIGURED_BYTE_BURST
            || self.rate.records_per_second == 0
            || self.rate.records_per_second > MAX_CONFIGURED_RECORDS_PER_SECOND
            || self.rate.record_burst == 0
            || self.rate.record_burst > MAX_CONFIGURED_RECORD_BURST
        {
            return Err(ConfigError::InvalidValue { field: "rate" });
        }
        let timeouts = self.timeouts;
        if [
            timeouts.connect,
            timeouts.handshake,
            timeouts.handshake_first_message,
            timeouts.admission,
            timeouts.ordinary_transfer,
            timeouts.snapshot_transfer,
            timeouts.read_write,
            timeouts.heartbeat,
            timeouts.idle,
        ]
        .into_iter()
        .any(|timeout| timeout.is_zero() || timeout > MAX_CONFIGURED_TIMEOUT)
            || timeouts.handshake_first_message > timeouts.handshake
            || timeouts.snapshot_transfer < timeouts.ordinary_transfer
            || timeouts.read_write > timeouts.idle
            || timeouts.admission > timeouts.idle
            || timeouts.idle <= timeouts.heartbeat
        {
            return Err(ConfigError::InvalidValue { field: "timeouts" });
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_fit_protocol_hard_limits() {
        let config = TransportConfig::default().validate().unwrap();
        assert!(config.wire_limits.max_presence_bytes < 64 * 1024);
        assert!(config.wire_limits.max_txn_bytes < 4 * 1024 * 1024);
        assert!(MAX_NOISE_CIPHERTEXT_BYTES <= u16::MAX as usize);
        let maximum_snapshot_records =
            MAX_SNAPSHOT_TRANSFER_BYTES.div_ceil(MAX_NOISE_PLAINTEXT_BYTES - 24);
        assert!(config.rate.record_burst >= maximum_snapshot_records as u64);
        let maximum_snapshot_wire_bytes =
            MAX_SNAPSHOT_TRANSFER_BYTES + maximum_snapshot_records * 24;
        assert!(config.rate.byte_burst >= maximum_snapshot_wire_bytes as u64);
    }

    #[test]
    fn pending_ceiling_leaves_headroom_for_honest_joins_under_a_silent_flood() {
        let config = TransportConfig::default().validate().unwrap();
        let connections = config.connections;
        // The documented flood: every address opens its per-address maximum.
        // The ceiling must still leave seats for honest guests, and a silent
        // peer only holds its seat for the first-message window.
        let flood_addresses = 4;
        let flood_seats = flood_addresses * connections.max_pending_handshakes_per_ip;
        assert!(flood_seats < connections.max_pending_handshakes);
        assert!(
            connections.max_pending_handshakes / connections.max_pending_handshakes_per_ip >= 32,
            "filling the pending pool must require many distinct source addresses"
        );
        assert!(config.timeouts.handshake_first_message < config.timeouts.handshake);
        assert!(connections.max_pending_handshakes <= MAX_CONFIGURED_CONNECTIONS);
    }

    #[test]
    fn invalid_resource_limits_fail_closed() {
        let mut config = TransportConfig::default();
        config.connections.outbound_queue_items = 0;
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue {
                field: "outbound_queue_items"
            })
        );

        let mut config = TransportConfig::default();
        config.timeouts.snapshot_transfer = Duration::from_secs(1);
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue { field: "timeouts" })
        );

        let mut config = TransportConfig::default();
        config.wire_limits.max_presence_bytes = 64 * 1024;
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue {
                field: "wire.max_presence_bytes"
            })
        );

        let mut config = TransportConfig::default();
        config.wire_limits.max_txn_bytes = 4 * 1024 * 1024;
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue {
                field: "wire.max_txn_bytes"
            })
        );

        let mut config = TransportConfig::default();
        config.rate.record_burst = MAX_CONFIGURED_RECORD_BURST + 1;
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue { field: "rate" })
        );

        let mut config = TransportConfig::default();
        config.timeouts.read_write = config.timeouts.idle + Duration::from_millis(1);
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue { field: "timeouts" })
        );

        let mut config = TransportConfig::default();
        config.timeouts.handshake_first_message =
            config.timeouts.handshake + Duration::from_secs(1);
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue { field: "timeouts" })
        );

        let mut config = TransportConfig::default();
        config.timeouts.handshake_first_message = Duration::ZERO;
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue { field: "timeouts" })
        );
    }
}
