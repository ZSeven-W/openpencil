//! Shim over `op_chat_agent::design_agent_tools` (the design tool surface
//! moved there, pure code motion, so mobile hosts share it).
//!
//! Two jobs beyond re-exporting:
//!
//! 1. Install this host's screenshot/export tool registrar before any
//!    executor entry point runs — those two tool arms live with the
//!    render/export stack (`mcp_serve::{screenshot_tool, export_tool}`),
//!    which `op-chat-agent` cannot link. Installing lazily from every
//!    wrapper keeps desktop/daemon behavior identical with zero startup
//!    wiring.
//! 2. Keep every existing `op_host_services::design_agent_tools::…` path
//!    valid.

use op_ai::chat_provider::ChatToolResult;
use op_editor_core::EditorState;
use op_mcp::ToolRegistry;

pub use op_chat_agent::design_agent_tools::{
    design_tool_defs, design_tool_level, install_host_tool_registrar, reveal_now_millis,
    root_seed_prompt_is_continuation, root_seed_prompt_is_mobile, root_seed_target_for_prompt,
    scan_duplicate_root_issues, scan_empty_shells, scan_header_icon_row_issues, scan_ring_issues,
    HostToolRegistrar, RootSeedGuard, RootSeedTarget, DESIGN_TOOLS,
};
pub(crate) use op_editor_core::agent_reveals::{
    collect_active_node_ids, register_new_node_reveals,
};

/// This host's screenshot/export registry arms — the render/export half the
/// shared crate parameterizes out. Returns true when it registered.
fn host_tool_registrar(state: &EditorState, requested: &str, registry: &mut ToolRegistry) -> bool {
    use crate::mcp_serve::export_tool::export_nodes_snapshot;
    use crate::mcp_serve::screenshot_tool::get_screenshot_snapshot;
    match requested {
        "get_screenshot" => {
            registry.register(Box::new(get_screenshot_snapshot(state)));
            true
        }
        "export_nodes" => {
            registry.register(Box::new(export_nodes_snapshot(state)));
            true
        }
        _ => false,
    }
}

/// Idempotent; called from every executor wrapper below.
fn ensure_host_registrar() {
    install_host_tool_registrar(host_tool_registrar);
}

pub fn execute_design_tool(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
) -> (ChatToolResult, bool) {
    ensure_host_registrar();
    op_chat_agent::design_agent_tools::execute_design_tool(state, name, args_json)
}

pub fn execute_design_tool_with_reveals(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
) -> (ChatToolResult, bool) {
    ensure_host_registrar();
    op_chat_agent::design_agent_tools::execute_design_tool_with_reveals(
        state,
        name,
        args_json,
        indicator_epoch,
    )
}

pub fn execute_design_tool_with_root_seed_guard(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&mut RootSeedGuard>,
) -> (ChatToolResult, bool) {
    ensure_host_registrar();
    op_chat_agent::design_agent_tools::execute_design_tool_with_root_seed_guard(
        state,
        name,
        args_json,
        indicator_epoch,
        root_seed_guard,
    )
}

pub fn execute_agent_tool(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
) -> (ChatToolResult, bool) {
    ensure_host_registrar();
    op_chat_agent::design_agent_tools::execute_agent_tool(state, name, args_json)
}

pub fn execute_agent_tool_with_reveals(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
) -> (ChatToolResult, bool) {
    ensure_host_registrar();
    op_chat_agent::design_agent_tools::execute_agent_tool_with_reveals(
        state,
        name,
        args_json,
        indicator_epoch,
    )
}

pub fn execute_agent_tool_with_root_seed_guard(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&mut RootSeedGuard>,
) -> (ChatToolResult, bool) {
    ensure_host_registrar();
    op_chat_agent::design_agent_tools::execute_agent_tool_with_root_seed_guard(
        state,
        name,
        args_json,
        indicator_epoch,
        root_seed_guard,
    )
}
