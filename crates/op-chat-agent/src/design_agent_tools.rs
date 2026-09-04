//! In-process design tool surface for the AI design agent loop.
//!
//! Mirrors `chat_canvas_tools.rs` for the 15-tool design toolset (vs the
//! 7-tool CRUD set). Schema definitions for every tool are derived from
//! `mcp_serve::schemas::TOOL_SCHEMAS` — the same source the MCP server
//! advertises — so the in-process and MCP surfaces stay byte-equal as JSON.
//!
//! The loop's design surface is `batch_design`, which accepts a sandboxed-JS
//! `script` input (see `op_mcp::script_runner`) in addition to the `operations`
//! DSL — giving the loop loops/data-driven emission without a separate
//! element-builder tool.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_ai::chat_provider::{ChatToolDef, ChatToolResult};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use op_mcp::ToolRegistry;

use crate::chat_canvas_tools::{execute_chat_tool, execute_with_registry};
use crate::mcp_serve::schemas;

/// The 15-tool design toolset with auth levels.
/// Reads = "read"; batch_design / set_variables / spawn_agents /
/// export_nodes = "create".
pub const DESIGN_TOOLS: &[(&str, &str)] = &[
    ("get_editor_state", "read"),
    ("get_guidelines", "read"),
    ("get_style_guide_tags", "read"),
    ("get_style_guide", "read"),
    ("get_variables", "read"),
    ("set_variables", "create"),
    ("apply_design_system", "create"),
    ("batch_get", "read"),
    ("snapshot_layout", "read"),
    ("find_empty_space", "read"),
    ("batch_design", "create"),
    ("get_screenshot", "read"),
    ("export_nodes", "create"),
    ("spawn_agents", "create"),
    ("ToolSearch", "read"),
];

/// Non-destructive quality report shared by the in-process design loop and
/// the public `get_design_quality` MCP tool. Every field is evidence only:
/// collecting the report never applies cleanup, rewrites layout, or changes
/// editor state.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignQualityDiagnostics {
    pub geometry_issues: Vec<String>,
    pub layout_issues: Vec<String>,
    pub contrast_issues: Vec<ContrastIssue>,
    pub icon_issues: Vec<String>,
    pub structure_issues: Vec<String>,
    pub empty_shells: Vec<String>,
    pub intent_questions: Vec<String>,
    pub variable_issues: Vec<String>,
    pub image_slots: Vec<String>,
    pub nav_issues: Vec<String>,
}

/// One quality response carries the full actionable contrast inventory for a
/// normal design while remaining bounded for adversarial documents.
const MAX_CONTRAST_ISSUES: usize = 64;

/// Collect the exact detect-only diagnostics used after a design-agent batch.
///
/// Deliberately excludes the two repair calls in the post-batch path
/// (`remove_nested_duplicate_status_bars` and mobile-nav reflow), making this
/// safe for read-only MCP credentials and repeated polling.
pub fn collect_design_quality(state: &EditorState) -> DesignQualityDiagnostics {
    let geometry_issues = op_orchestrator::geometry_validation::geometry_diagnostics(state);
    let effective_theme = op_editor_core::variables_resolve::effective_theme(
        &state.doc,
        &state.ui.variables.active_theme,
    );
    let mut contrast_issues = scan_contrast_issues(
        state.active_children(),
        state.doc.variables.as_ref(),
        &effective_theme,
    );
    contrast_issues.truncate(MAX_CONTRAST_ISSUES);
    let icon_issues = scan_icon_issues(state.active_children());
    let mut structure_issues = scan_duplicate_root_issues(state.active_children());
    structure_issues.extend(scan_ring_issues(state.active_children()));
    structure_issues.extend(scan_header_icon_row_issues(state.active_children()));
    structure_issues.truncate(12);
    let empty_shells = scan_empty_shells(state.active_children());
    let diagnostics = crate::design_agent_diagnostics::collect_batch_design_diagnostics(state);
    let nav_issues = op_orchestrator::nav_issues::scan_nav_issues(state);
    DesignQualityDiagnostics {
        geometry_issues,
        layout_issues: diagnostics.layout_issues,
        contrast_issues,
        icon_issues,
        structure_issues,
        empty_shells,
        intent_questions: diagnostics.intent_questions,
        variable_issues: diagnostics.variable_issues,
        image_slots: diagnostics.image_slot_candidates,
        nav_issues,
    }
}

/// Auth level for a design tool name (`None` = not in the design set).
pub fn design_tool_level(name: &str) -> Option<&'static str> {
    DESIGN_TOOLS
        .iter()
        .find(|(tool, _)| *tool == name)
        .map(|(_, level)| *level)
}

/// Build tool definitions for the design agent by deriving them from
/// `schemas::TOOL_SCHEMAS` — so the in-process schema is byte-equal to
/// what the MCP server advertises (parity guarantee).
pub fn design_tool_defs() -> Vec<ChatToolDef> {
    DESIGN_TOOLS
        .iter()
        .map(|(name, _)| {
            // Every design tool is sourced from TOOL_SCHEMAS for byte-equal
            // MCP parity.
            let (description, input_schema_json) = extract_from_schemas(name)
                .unwrap_or_else(|| panic!("design tool {name} not found in TOOL_SCHEMAS"));
            ChatToolDef {
                name: name.to_string(),
                description,
                level: design_tool_level(name).unwrap_or("read").to_string(),
                input_schema_json,
            }
        })
        .collect()
}

/// Execute one design tool call against the live editor state. Returns
/// the TS-shaped tool result plus whether the call mutated the document.
///
/// Reuses `execute_with_registry` from `chat_canvas_tools` so the
/// dispatch+apply discipline (parse_tool_call → registry.dispatch →
/// state.apply → envelope) is not duplicated.
pub fn execute_design_tool(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
) -> (ChatToolResult, bool) {
    execute_design_tool_with_reveals(
        state,
        name,
        args_json,
        op_editor_core::agent_indicators::active_epoch(),
    )
}

/// Execute one design tool call and register entrance reveals for nodes inserted
/// by write batches when the host has an active indicator epoch.
pub fn execute_design_tool_with_reveals(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
) -> (ChatToolResult, bool) {
    execute_design_tool_with_root_seed_guard(state, name, args_json, indicator_epoch, None)
}

/// Execute one design tool call with an optional root seed guard for headless
/// loop callers that do not use an indicator epoch.
pub fn execute_design_tool_with_root_seed_guard(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
    mut root_seed_guard: Option<&mut RootSeedGuard>,
) -> (ChatToolResult, bool) {
    let Some(_level) = design_tool_level(name) else {
        let envelope = serde_json::json!({ "success": false, "error": format!("tool not available in design agent: {name}") });
        return (
            ChatToolResult {
                content: envelope.to_string(),
                is_error: true,
            },
            false,
        );
    };
    // An explicit batch page is a context for the whole same-batch quality
    // pass, not just for the write command. Keep every before/after read and
    // deterministic repair on that page, then put the user's canvas context
    // back exactly as it was.
    let original_page_index = state.ui.active_page_index;
    let original_selection = state.selection.clone();
    if name == "batch_design" {
        if let Some(target_page_index) = batch_design_target_page_index(state, args_json) {
            state.ui.active_page_index = target_page_index;
        }
    }
    let reveal_started_ms = reveal_now_millis();
    let ids_before = should_register_batch_reveals(name, indicator_epoch)
        .then(|| collect_active_node_ids(state));
    let root_ids_before =
        should_track_root_seed_candidate(state, name, indicator_epoch, root_seed_guard.as_deref())
            .then(|| collect_active_top_level_node_ids(state));
    let registry = design_tool_registry(state, name);
    let (mut result, mutated) = execute_with_registry(state, name, args_json, registry);
    if mutated && !result.is_error {
        if let Some(ids_before) = ids_before.as_ref() {
            register_new_node_reveals(ids_before, state, indicator_epoch, reveal_started_ms);
        }
    }
    if mutated && name == "batch_design" && !result.is_error {
        let root_seed_hint = root_ids_before.as_ref().and_then(|ids_before| {
            maybe_apply_root_seed_guard(state, ids_before, indicator_epoch, root_seed_guard.take())
        });

        // Per-batch layout feedback: after every WRITE batch, attach what the
        // real layout proves wrong (collapses / table overflow / text overflow)
        // so the model sees each batch's geometric consequences immediately and
        // repairs them in-process, instead of piling defects up for the loop-end
        // finalize. Deterministic analogue of Pencil's per-batch
        // snapshot_layout feedback.
        let dup_bars_removed = remove_nested_duplicate_status_bars(state);
        // A mobile skeleton is deliberately numeric while it is empty. Once a
        // trailing bottom nav lands, recover any stale numeric content-shell
        // remainder before diagnostics: otherwise the old shell consumes the
        // whole root and places the nav exactly outside the clipped artboard.
        // The orchestrator owns the narrow structural proof and only grows the
        // numeric root when the shell's real content actually needs it.
        let mobile_nav_reflowed = op_orchestrator::repair_mobile_trailing_nav_reflow(state);
        let mut layout_issues = op_orchestrator::geometry_validation::geometry_diagnostics(state);
        let effective_theme = op_editor_core::variables_resolve::effective_theme(
            &state.doc,
            &state.ui.variables.active_theme,
        );
        let mut contrast_issues = scan_contrast_issues(
            state.active_children(),
            state.doc.variables.as_ref(),
            &effective_theme,
        );
        contrast_issues.truncate(MAX_CONTRAST_ISSUES);
        let icon_issues = scan_icon_issues(state.active_children());
        let mut dup_root_issues = scan_duplicate_root_issues(state.active_children());
        dup_root_issues.extend(scan_ring_issues(state.active_children()));
        dup_root_issues.extend(scan_header_icon_row_issues(state.active_children()));
        if dup_bars_removed > 0 {
            dup_root_issues.push(format!(
                "removed {dup_bars_removed} extra status bar(s) you built - the standard                  status bar already exists; NEVER create another one"
            ));
        }
        if mobile_nav_reflowed {
            dup_root_issues.push(
                "reflowed the mobile content shell and trailing bottom nav inside the root - keep ordinary content wrappers height=fit_content, preserve the status/content/nav region gap, and grow the numeric root only when real content requires it"
                    .to_string(),
            );
        }
        let empty_shells = scan_empty_shells(state.active_children());
        // Track B of the interactive-preview plan: an intent-shaped echo (not
        // an auto-fix — see `op_orchestrator::nav_issues` module doc) naming
        // any nav-tab item that name-matches an already screen-marked frame
        // but has no `events.onTap` bound yet. `wire_screen_navigation`
        // (Track A) is the deterministic backstop if the model never gets to
        // it before the design ends.
        let nav_issues = op_orchestrator::nav_issues::scan_nav_issues(state);
        let design_diagnostics =
            crate::design_agent_diagnostics::collect_batch_design_diagnostics(state);
        layout_issues.extend(design_diagnostics.layout_issues);
        let intent_questions = design_diagnostics.intent_questions;
        let variable_issues = design_diagnostics.variable_issues;
        let image_slot_candidates = design_diagnostics.image_slot_candidates;
        if !layout_issues.is_empty()
            || !dup_root_issues.is_empty()
            || !contrast_issues.is_empty()
            || !icon_issues.is_empty()
            || !empty_shells.is_empty()
            || !intent_questions.is_empty()
            || !variable_issues.is_empty()
            || !image_slot_candidates.is_empty()
            || !nav_issues.is_empty()
            || root_seed_hint.is_some()
        {
            if let Ok(mut envelope) = serde_json::from_str::<serde_json::Value>(&result.content) {
                if let Some(obj) = envelope.as_object_mut() {
                    if !layout_issues.is_empty() {
                        obj.insert("layoutIssues".into(), serde_json::json!(layout_issues));
                    }
                    if !icon_issues.is_empty() {
                        obj.insert("iconIssues".into(), serde_json::json!(icon_issues));
                    }
                    if !dup_root_issues.is_empty() {
                        obj.insert("structureIssues".into(), serde_json::json!(dup_root_issues));
                    }
                    if !intent_questions.is_empty() {
                        obj.insert(
                            "intentQuestions".into(),
                            serde_json::json!(intent_questions),
                        );
                    }
                    if !variable_issues.is_empty() {
                        obj.insert("variableIssues".into(), serde_json::json!(variable_issues));
                    }
                    if !image_slot_candidates.is_empty() {
                        obj.insert(
                            "imageSlots".into(),
                            serde_json::json!(image_slot_candidates),
                        );
                    }
                    if !empty_shells.is_empty() {
                        obj.insert(
                            "shellsRemaining".into(),
                            serde_json::json!(format!(
                                "{} - fill each in its own batch; NEVER end the design while any shell is empty (D() a shell you decided against)",
                                empty_shells.join(", ")
                            )),
                        );
                    }
                    if !contrast_issues.is_empty() {
                        obj.insert(
                            "contrastHint".into(),
                            serde_json::json!(contrast_hint(&contrast_issues)),
                        );
                        obj.insert("contrastIssues".into(), serde_json::json!(contrast_issues));
                    }
                    if !nav_issues.is_empty() {
                        obj.insert("navIssues".into(), serde_json::json!(nav_issues));
                    }
                    let mut hints = Vec::new();
                    if !layout_issues.is_empty() {
                        hints.push(
                            "The resolved layout has the issues above. Fix them with a follow-up batch_design before building the next section."
                                .to_string(),
                        );
                    }
                    if !icon_issues.is_empty() {
                        hints.push(
                            "iconIssues: every icon listed renders as a fallback dot. Fix each with U(id, {\"iconFontName\":\"<glyph>\"}) using a real lucide glyph name (home/search/heart/compass/...)."
                                .to_string(),
                        );
                    }
                    if !intent_questions.is_empty() {
                        hints.push(
                            "intentQuestions are ambiguous: inspect the named nodes and choose explicitly. Do not assume the finalizer will move, resize, delete, or recolor them."
                                .to_string(),
                        );
                    }
                    if !variable_issues.is_empty() {
                        hints.push(
                            "variableIssues name broken references. Replace them deliberately with a token returned by get_variables or a concrete value; nearby colours are not sufficient evidence."
                                .to_string(),
                        );
                    }
                    if !image_slot_candidates.is_empty() {
                        hints.push(
                            "imageSlots lists unresolved media slots. Resolve them before continuing: default G requires the exact EMPTY slot id, never its row/card container; if an image is already a direct sibling, use M(imageId, slotId) only when explicit parent/slot hierarchy assigns it there. Do not decide from image subject, aesthetics, or perceived quality."
                                .to_string(),
                        );
                    }
                    if !nav_issues.is_empty() {
                        hints.push(
                            "navIssues: this is a multi-screen app - the listed nav tabs are not wired to switch screens yet. Bind each one's events.onTap exactly as shown; do not guess a different destination."
                                .to_string(),
                        );
                    }
                    if let Some(hint) = root_seed_hint {
                        hints.push(hint);
                    }
                    if !hints.is_empty() {
                        obj.insert("layoutHint".into(), serde_json::json!(hints.join(" ")));
                    }
                    result.content = envelope.to_string();
                }
            }
        }
    }
    if name == "batch_design" {
        state.ui.active_page_index = original_page_index;
        state.selection = original_selection;
    }
    (result, mutated)
}

/// Resolve the optional outer batch page once, using the same id-first then
/// legacy-index contract as the MCP command applier. An invalid explicit page
/// returns `None`; the write will be rejected by the MCP path and no post-pass
/// will run.
fn batch_design_target_page_index(state: &EditorState, args_json: &str) -> Option<usize> {
    let page_selector = serde_json::from_str::<serde_json::Value>(args_json)
        .ok()
        .and_then(|args| {
            args.get("pageId")
                .or_else(|| args.get("page_id"))
                .or_else(|| args.get("page"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|page| !page.is_empty())
                .map(str::to_string)
        });
    match page_selector.as_deref() {
        Some(page) => match state.doc.pages.as_ref() {
            Some(pages) if !pages.is_empty() => pages
                .iter()
                .position(|candidate| candidate.id == page)
                .or_else(|| {
                    page.parse::<usize>()
                        .ok()
                        .filter(|index| *index < pages.len())
                }),
            _ => page.parse::<usize>().ok().filter(|index| *index == 0),
        },
        None => Some(
            state
                .ui
                .active_page_index
                .min(state.page_count().saturating_sub(1)),
        ),
    }
}

/// Unified executor for the design agent pump: design-surface tools
/// route to [`execute_design_tool`]; everything else falls through to
/// [`execute_chat_tool`] (the CRUD surface). Only tools the provider
/// ADVERTISES are ever called by the model, so CRUD tools never see
/// design-only names and vice versa — this router is purely defensive.
///
/// This is the single call-site in `chat_session.rs::drain_tool_requests`
/// once the design-loop flag is ON. When the flag is OFF the pump still
/// calls `execute_chat_tool` directly, so the CRUD path is unaffected.
pub fn execute_agent_tool(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
) -> (ChatToolResult, bool) {
    execute_agent_tool_with_reveals(
        state,
        name,
        args_json,
        op_editor_core::agent_indicators::active_epoch(),
    )
}

/// Host-facing tool router with an explicit indicator epoch for tests and
/// desktop paths that already know the active design-loop epoch.
pub fn execute_agent_tool_with_reveals(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
) -> (ChatToolResult, bool) {
    execute_agent_tool_with_root_seed_guard(state, name, args_json, indicator_epoch, None)
}

/// Host-facing router with an optional local root seed guard for tool loops
/// without an indicator epoch.
pub fn execute_agent_tool_with_root_seed_guard(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&mut RootSeedGuard>,
) -> (ChatToolResult, bool) {
    if design_tool_level(name).is_some() {
        execute_design_tool_with_root_seed_guard(
            state,
            name,
            args_json,
            indicator_epoch,
            root_seed_guard,
        )
    } else {
        execute_chat_tool(state, name, args_json)
    }
}

/// Registrar for the host-supplied tool arms (`get_screenshot` /
/// `export_nodes`), whose implementations live with the render/export stack
/// in `op-host-services::mcp_serve` — a closure this crate cannot link.
/// Returns true when it registered the requested tool.
///
/// `op-host-services`' `design_agent_tools` shim installs its registrar
/// lazily from every executor entry point, so desktop/daemon behavior is
/// unchanged. A host that never installs one (the mobile FFI) simply has no
/// screenshot/export arms — those tools are not advertised there either.
pub type HostToolRegistrar = fn(&EditorState, &str, &mut ToolRegistry) -> bool;

static HOST_TOOL_REGISTRAR: std::sync::OnceLock<HostToolRegistrar> = std::sync::OnceLock::new();

/// Install the host's screenshot/export tool registrar. Idempotent; the
/// first installer wins.
pub fn install_host_tool_registrar(registrar: HostToolRegistrar) {
    let _ = HOST_TOOL_REGISTRAR.set(registrar);
}

/// The descriptor catalog `ToolSearch` answers from.
pub type HostToolCatalog = fn() -> &'static [&'static str];

static HOST_TOOL_CATALOG: std::sync::OnceLock<HostToolCatalog> = std::sync::OnceLock::new();

/// Narrow `ToolSearch` to the tools THIS host can actually execute.
///
/// A `select:` query returns every matched descriptor regardless of
/// `max_results`, and the protocol's Step 2 select list names
/// `get_screenshot` / `spawn_agents` — so a host that does not advertise
/// those (the mobile FFI) otherwise hands the model descriptors for tools
/// its executor rejects, teaching it to spend turns on calls that can only
/// fail. Idempotent; the first installer wins. Uninstalled hosts keep the
/// full MCP catalog (desktop/daemon behavior unchanged).
pub fn install_host_tool_catalog(catalog: HostToolCatalog) {
    let _ = HOST_TOOL_CATALOG.set(catalog);
}

fn tool_search_catalog() -> &'static [&'static str] {
    HOST_TOOL_CATALOG
        .get()
        .map(|catalog| catalog())
        .unwrap_or(schemas::TOOL_SCHEMAS)
}

/// Build a registry carrying only the requested design tool — snapshot
/// registered against the live state so read tools see prior writes.
fn design_tool_registry(state: &EditorState, requested: &str) -> ToolRegistry {
    let mut r = ToolRegistry::default();
    match requested {
        "get_editor_state" => r.register(Box::new(op_mcp::get_editor_state_snapshot(state))),
        "get_guidelines" => r.register(Box::new(op_mcp::get_guidelines_snapshot())),
        "get_style_guide_tags" => r.register(Box::new(op_mcp::get_style_guide_tags_snapshot())),
        "get_style_guide" => r.register(Box::new(op_mcp::get_style_guide_snapshot())),
        "get_variables" => r.register(Box::new(op_mcp::get_variables_snapshot(state))),
        "set_variables" => r.register(Box::new(op_mcp::set_variables_snapshot())),
        "apply_design_system" => r.register(Box::new(op_mcp::apply_design_system_snapshot())),
        "batch_get" => r.register(Box::new(op_mcp::batch_get_snapshot(state))),
        "snapshot_layout" => r.register(Box::new(op_mcp::snapshot_layout_snapshot(state))),
        "find_empty_space" => r.register(Box::new(op_mcp::find_empty_space_snapshot(state))),
        "batch_design" => r.register(Box::new(op_mcp::batch_design_snapshot(state))),
        "get_screenshot" | "export_nodes" => {
            if let Some(registrar) = HOST_TOOL_REGISTRAR.get() {
                let _ = registrar(state, requested, &mut r);
            }
        }
        "spawn_agents" => r.register(Box::new(op_mcp::spawn_agents_snapshot())),
        "ToolSearch" => r.register(Box::new(
            op_mcp::tool_search_snapshot(tool_search_catalog()),
        )),
        _ => {}
    }
    r
}

fn should_register_batch_reveals(name: &str, indicator_epoch: Option<u64>) -> bool {
    indicator_epoch.is_some() && name == "batch_design"
}

// The reveal walk (before/after id diff + staggered reveal registration)
// is single-sourced in op-editor-core; re-exported `pub(crate)` so the
// live-MCP indicator hook (`mcp_live.rs`) keeps its existing paths.
pub use op_editor_core::agent_reveals::{collect_active_node_ids, register_new_node_reveals};

/// Extract `description` and `inputSchema` JSON string from `TOOL_SCHEMAS`
/// for the given tool name. Returns `None` when no entry matches.
///
/// Parses the raw JSON descriptor using `serde_json` so the extracted
/// `inputSchema` is round-tripped through `serde_json::Value` — ensuring
/// the parity test can compare it by value rather than string equality.
fn extract_from_schemas(name: &str) -> Option<(String, String)> {
    for entry in schemas::TOOL_SCHEMAS {
        let v: serde_json::Value = serde_json::from_str(entry).ok()?;
        if v.get("name").and_then(|n| n.as_str()) == Some(name) {
            return extract_from_schema_entry(entry);
        }
    }
    None
}

/// Parse one tool descriptor JSON string into `(description, inputSchema)`.
/// Used for `TOOL_SCHEMAS` entries.
fn extract_from_schema_entry(entry: &str) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(entry).ok()?;
    let description = v.get("description")?.as_str()?.to_string();
    let input_schema = v.get("inputSchema")?.clone();
    Some((description, input_schema.to_string()))
}

mod quality_scans;
mod root_seed;

pub use quality_scans::*;
pub use root_seed::*;

#[cfg(test)]
#[path = "design_agent_tools_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "design_agent_tools_seed_tests.rs"]
mod seed_tests;

#[cfg(test)]
#[path = "design_agent_tools_scan_tests.rs"]
mod scan_tests;

#[cfg(test)]
#[path = "design_agent_tools_continuation_tests.rs"]
mod continuation_tests;

#[cfg(test)]
#[path = "mobile_normalize_tests.rs"]
mod mobile_normalize_tests;
