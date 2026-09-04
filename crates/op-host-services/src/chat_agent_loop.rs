//! Re-export shim: the tool-executing agent loop moved to `op-chat-agent`
//! (pure code motion) so the mobile FFI hosts run the same loop; every
//! existing `op_host_services::chat_agent_loop::…` path stays valid.

pub use op_chat_agent::chat_agent_loop::*;
