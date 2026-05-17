//! OpenPencil ACP client — a Rust port of the TS `pen-acp` package.
//!
//! ACP (Agent Client Protocol) lets third-party agents plug into
//! OpenPencil over JSON-RPC 2.0 framed as newline-delimited JSON. An
//! agent runs either as a local child process (stdio) or behind a
//! remote WebSocket endpoint.
//!
//! Layers:
//!  - [`transport`] — ndJSON frame read / write;
//!  - [`jsonrpc`] — request-id correlation, notification routing,
//!    `session/request_permission` auto-approval;
//!  - [`client`] — [`AcpConnection`]: `initialize` / `session/new` /
//!    `session/prompt`;
//!  - [`event_adapter`] — maps `session/update` notifications onto the
//!    chat panel's `ChatDelta` vocabulary.
//!
//! Desktop-only — it spawns processes + drives async IO via tokio, so
//! no wasm crate depends on it.

pub mod client;
pub mod event_adapter;
pub mod jsonrpc;
pub mod protocol;
pub mod transport;
pub mod types;

pub use client::{connect_acp_agent, AcpConnection};
pub use event_adapter::session_update_to_delta;
pub use protocol::{SessionNotification, SessionUpdate};
pub use types::{AcpAgentConfig, AcpAgentInfo, AcpConnectResult, AcpError, ConnectionType};
