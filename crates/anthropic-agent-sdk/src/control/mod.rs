//! Control protocol for bidirectional communication with Claude Code
//!
//! This module implements the control protocol that enables bidirectional
//! communication between the SDK and Claude Code CLI, including:
//! - Request/response routing
//! - Initialization handshake
//! - Control messages (interrupt, pause, resume)
//! - Hook invocations
//! - Permission requests
//!
//! # Architecture
//!
//! The control protocol operates over the same transport as regular messages,
//! using a special message format to distinguish control messages from data messages.
//!
//! ## Message Flow
//!
//! ```text
//! SDK                          CLI
//!  |                            |
//!  |--- Init Request ---------->|
//!  |<-- Init Response -----------|
//!  |                            |
//!  |--- User Message ---------->|
//!  |<-- Assistant Message ------|
//!  |<-- Hook Event -------------|
//!  |--- Hook Response --------->|
//!  |<-- Permission Request -----|
//!  |--- Permission Response --->|
//!  |--- Interrupt ------------->|
//!  |<-- Result Message ---------|
//!  ```
//!
//! # Example: Using the Protocol Handler
//!
//! ```rust
//! use anthropic_agent_sdk::control::ProtocolHandler;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a protocol handler
//! let mut handler = ProtocolHandler::new();
//!
//! // Initialize the protocol
//! let init_request = handler.create_init_request();
//! // ... send init_request to CLI and receive response ...
//!
//! // Mark as initialized after handshake
//! handler.set_initialized(true);
//!
//! // Create and send control requests
//! let interrupt = handler.create_interrupt_request();
//! let send_msg = handler.create_send_message_request("Hello".to_string());
//! # Ok(())
//! # }
//! ```
//!
//! # Protocol Version
//!
//! Current protocol version: `1.0`
//!
//! The protocol version is negotiated during initialization. If the CLI
//! supports a different version, the connection will fail with an error.

pub mod protocol;

pub use protocol::{
    ControlMessage, ControlRequest, ControlResponse, InitRequest, InitResponse, ProtocolHandler,
};
