//! Re-export shim: the chat CRUD canvas tool surface moved to
//! `op-chat-agent` (pure code motion); every existing
//! `op_host_services::chat_canvas_tools::…` path stays valid.

pub use op_chat_agent::chat_canvas_tools::*;
