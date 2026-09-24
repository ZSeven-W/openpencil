//! The `tools/list` catalog response.
//!
//! Split out of the `mcp_serve` spine at the 800-line cap. It lives next to
//! `tool_profile` because the two are one decision: the profile says which
//! tools a deployment offers, and this is where that answer becomes the
//! catalog a client sees.

use op_editor_core::EditorState;

use super::lean_profile::lean_tool_schemas;
#[cfg(feature = "mcp-debug-tools")]
use super::schemas::DEBUG_TOOL_SCHEMAS;
use super::schemas::TOOL_SCHEMAS;
use super::tool_catalog::McpToolCatalog;
use super::tool_profile;

pub(super) fn tools_list_response(
    id_raw: &str,
    state: &EditorState,
    debug_enabled: bool,
    profile: tool_profile::McpAccessProfile,
) -> String {
    // Discovery follows enforcement: a tool this profile would refuse is not
    // advertised, so a client never plans around one it cannot call.
    let listed =
        |schema: &str| tool_profile::schema_name(schema).is_none_or(|name| profile.lists(&name));
    // The lean catalog advertises its own derived schemas — never the kit
    // insert tools or debug tools, which it does not serve.
    if profile.catalog == McpToolCatalog::Lean {
        let entries: Vec<&str> = lean_tool_schemas()
            .iter()
            .map(String::as_str)
            .filter(|schema| listed(schema))
            .collect();
        return format!(
            r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"tools":[{}]}}}}"#,
            entries.join(",")
        );
    }
    let mut entries: Vec<String> = TOOL_SCHEMAS
        .iter()
        .filter(|schema| listed(schema))
        .map(|s| (*s).to_string())
        .collect();
    entries.extend(
        op_mcp::element_tools::element_tool_schemas(state)
            .into_iter()
            .filter(|schema| listed(schema)),
    );
    #[cfg(not(feature = "mcp-debug-tools"))]
    let _ = debug_enabled;
    #[cfg(feature = "mcp-debug-tools")]
    if debug_enabled {
        entries.extend(
            DEBUG_TOOL_SCHEMAS
                .iter()
                .filter(|schema| listed(schema))
                .map(|s| (*s).to_string()),
        );
    }
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"tools":[{}]}}}}"#,
        entries.join(",")
    )
}
