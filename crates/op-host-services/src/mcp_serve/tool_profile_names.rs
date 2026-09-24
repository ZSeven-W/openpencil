//! Catalog-name helpers for `tool_profile`, split out of that module at the
//! 800-line cap (pure code motion; `tool_profile` re-exports every item so
//! import paths are unchanged).

#[cfg(feature = "mcp-debug-tools")]
use super::super::schemas::DEBUG_TOOL_SCHEMAS;
use super::super::schemas::TOOL_SCHEMAS;

/// Names that only exist in a build with the debug-tool feature.
///
/// They stay classified in every build so the deny decision cannot be lost
/// by flipping a feature flag; the parity test knows they are absent from the
/// catalog when the feature is off.
pub const DEBUG_ONLY_TOOLS: &[&str] = &[
    "debug_logs_tail",
    "debug_screenshot",
    "debug_validation_report",
];

pub fn is_debug_only_tool(name: &str) -> bool {
    DEBUG_ONLY_TOOLS.contains(&name)
}

/// Every catalog name this build advertises.
///
/// Only the parity tests consume this; it stays compiled in every build so
/// the debug-tool and production catalogs cannot drift.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn catalog_tool_names() -> Vec<String> {
    #[cfg_attr(not(feature = "mcp-debug-tools"), allow(unused_mut))]
    let mut names: Vec<String> = TOOL_SCHEMAS
        .iter()
        .filter_map(|schema| schema_name(schema))
        .collect();
    #[cfg(feature = "mcp-debug-tools")]
    names.extend(
        DEBUG_TOOL_SCHEMAS
            .iter()
            .filter_map(|schema| schema_name(schema)),
    );
    names
}

/// Pull `"name":"…"` out of a schema entry.
///
/// The schemas are pre-serialized JSON string constants, and the name is
/// always the first member, so this reads it without a JSON parse.
pub(crate) fn schema_name(schema: &str) -> Option<String> {
    let rest = schema.split_once(r#""name":"#)?.1.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
