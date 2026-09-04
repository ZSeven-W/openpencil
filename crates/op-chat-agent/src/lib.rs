//! Builtin-provider agent tool loop + canvas/design tool surface.
//!
//! Everything here moved verbatim from `op-host-services` (which re-exports
//! it all under its original paths) so the mobile FFI hosts run the SAME
//! design pipeline as desktop — the tool-executing loop with its corrective
//! budgets, the 15-tool design surface with its per-batch quality feedback,
//! and the loop-end structural finalize plumbing. The move is pure code
//! motion: module names are unchanged, and the [`chat_builtin_http`] /
//! [`chat_runtime`] / [`mcp_serve`] alias modules keep the moved files'
//! `crate::…` paths resolving without edits.
//!
//! What deliberately did NOT move: the provider transports themselves
//! (`ConfiguredBuiltinProvider`, CLI subprocess bridges, ACP, the daemon)
//! and the browser-credential policy (`web_dial_policy_for`) — those stay in
//! `op-host-services`, which remains the daemon-side owner.

pub mod chat_agent_context;
pub mod chat_agent_loop;
pub mod chat_canvas_tools;
pub mod chat_modify_sanitize;
pub mod chat_tool_result;
pub mod design_agent_diagnostics;
pub mod design_agent_tools;
pub mod design_context;
pub mod ip_screen;
pub mod loop_blocker_ledger;
pub mod provider_dial;
pub mod quality_credential;
pub mod runtime;
pub mod screen_sets;
pub mod tool_schemas;

pub use design_agent_tools::{normalize_mobile_screens, MobileNormalizeReport};

/// Path-compat alias for `crate::chat_intent::listed_whole_screen_names`
/// used by `design_agent_tools/root_seed.rs` — only the pure listed-screen
/// parser moved; the LLM intent pipeline stays in op-host-services.
pub mod chat_intent {
    pub use crate::screen_sets::*;
}

#[path = "backoff.rs"]
pub mod backoff;

// The moved error file, mounted at the crate root (an inline-module `#[path]`
// would resolve under a `chat_builtin_http/` subdirectory).
#[path = "chat_builtin_http_error.rs"]
mod builtin_http_error;

/// Path-compat alias: the moved modules referenced
/// `crate::chat_builtin_http::{send_with_backoff, BuiltinHttpError, …}`
/// inside op-host-services; this module keeps those paths valid here. The
/// error enum's file also moved (`chat_builtin_http_error.rs`).
pub mod chat_builtin_http {
    pub use crate::builtin_http_error::BuiltinHttpError;

    pub use crate::backoff::{
        apply_reasoning_wire_control, apply_reasoning_wire_control_anthropic, builtin_http_client,
        builtin_http_client_builder, default_backoff_knobs, send_with_backoff,
        DESIGN_LOOP_MAX_OUTPUT_TOKENS, DESIGN_LOOP_MAX_TURNS,
    };
    pub use op_ai::chat_sse::{map_anthropic_stop_reason, map_openai_stop_reason};
}

/// Path-compat alias for `crate::chat_runtime::{shared_runtime,
/// block_on_anywhere}` used by the moved loop tests and dial guard.
pub mod chat_runtime {
    pub use crate::runtime::{block_on_anywhere, shared_runtime};
}

/// Path-compat alias for `crate::mcp_serve::schemas` used by
/// `design_agent_tools` (tool defs are derived from the MCP catalog so the
/// two surfaces stay byte-equal).
pub mod mcp_serve {
    pub mod schemas {
        pub use crate::tool_schemas::*;
    }
}
