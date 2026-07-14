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
use jian_ops_schema::node::{ContainerProps, PenNode, TextContent};
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
        let contrast_issues = scan_contrast_issues(
            state.active_children(),
            state.doc.variables.as_ref(),
            &effective_theme,
        );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSeedTarget {
    Mobile,
    Desktop,
}

impl RootSeedTarget {
    fn from_mobile(mobile: bool) -> Self {
        if mobile {
            Self::Mobile
        } else {
            Self::Desktop
        }
    }

    fn dimensions(self) -> (f64, f64) {
        match self {
            Self::Mobile => (390.0, 844.0),
            Self::Desktop => (1440.0, 900.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RootSeedGuard {
    target: RootSeedTarget,
    consumed: bool,
}

impl RootSeedGuard {
    pub fn from_prompt(prompt: &str) -> Self {
        Self {
            target: root_seed_target_for_prompt(prompt),
            consumed: false,
        }
    }

    pub fn disabled() -> Self {
        Self {
            target: RootSeedTarget::Desktop,
            consumed: true,
        }
    }

    fn pending_target(&self) -> Option<RootSeedTarget> {
        (!self.consumed).then_some(self.target)
    }

    fn mark_consumed(&mut self) {
        self.consumed = true;
    }
}

pub fn root_seed_target_for_prompt(prompt: &str) -> RootSeedTarget {
    RootSeedTarget::from_mobile(root_seed_prompt_is_mobile(prompt))
}

pub fn root_seed_prompt_is_mobile(prompt: &str) -> bool {
    if prompt.contains("手机") {
        return true;
    }
    let lower = prompt.to_ascii_lowercase();
    // "web app" / "webapp" name desktop products — a dashboard "web app"
    // must not be seeded 390x844. Strip the desktop-ish phrases before the
    // word scan so the bare "app" signal only fires for actual mobile asks.
    if lower.contains("web app") || lower.contains("webapp") || lower.contains("desktop") {
        let desktopish = lower.replace("web app", " ").replace("webapp", " ");
        return desktopish
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| matches!(word, "mobile" | "phone" | "ios" | "android"));
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| matches!(word, "mobile" | "app" | "phone" | "ios" | "android"))
}

fn should_track_root_seed_candidate(
    _state: &EditorState,
    name: &str,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&RootSeedGuard>,
) -> bool {
    if name != "batch_design" {
        return false;
    }
    root_seed_guard
        .and_then(RootSeedGuard::pending_target)
        .is_some()
        || indicator_epoch
            .and_then(op_editor_core::agent_indicators::root_seed_hint_if_pending)
            .is_some()
}

fn collect_active_top_level_node_ids(state: &EditorState) -> HashSet<String> {
    state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect()
}

fn maybe_apply_root_seed_guard(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&mut RootSeedGuard>,
) -> Option<String> {
    let explicit_target = root_seed_guard
        .as_ref()
        .and_then(|guard| guard.pending_target());
    let epoch_target = indicator_epoch
        .and_then(op_editor_core::agent_indicators::root_seed_hint_if_pending)
        .map(RootSeedTarget::from_mobile);
    let target = explicit_target.or(epoch_target)?;
    let seed_hint = seed_root_frame_if_needed(state, ids_before, target);
    // Mobile chrome parity with the orchestrator scaffold: the loop's first
    // batch gets the SAME pre-inserted status bar, so `mobile-app.md`'s
    // "status bar is already pre-inserted" contract holds on this path too.
    // Runs even when the model authored explicit root dimensions (the seed
    // above early-returns then, but the chrome must still land).
    let chrome_hint = (target == RootSeedTarget::Mobile)
        .then(|| inject_mobile_status_bar_if_missing(state, ids_before))
        .flatten();

    if let Some(guard) = root_seed_guard {
        guard.mark_consumed();
    } else if let Some(epoch) = indicator_epoch {
        op_editor_core::agent_indicators::mark_root_seed_guard_consumed(epoch);
    }

    match (seed_hint, chrome_hint) {
        (Some(seed), Some(chrome)) => Some(format!("{seed} {chrome}")),
        (Some(seed), None) => Some(seed),
        (None, Some(chrome)) => Some(chrome),
        (None, None) => None,
    }
}

/// Insert the standard iOS status-bar chrome as the mobile root's FIRST
/// child unless the model already built one. Reuses the orchestrator
/// scaffold's exact node tree (`scaffold::mobile_status_bar_node`) so both
/// generation paths ship byte-identical chrome. Returns the model-facing
/// hint when the bar was injected.
fn inject_mobile_status_bar_if_missing(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
) -> Option<String> {
    let root = root_seed_candidate_mut(state, ids_before)?;
    // OS chrome has exactly one canonical form. A model-built status bar
    // (name matches, structure doesn't — no role, ad-hoc children) is
    // REPLACED in place rather than kept: every hand-rolled variant we
    // measured deviated visibly from the iOS reference (GLM-5.2 2026-07-11).
    let noncanonical_index = root
        .children()
        .into_iter()
        .flatten()
        .position(|child| is_status_bar_node(child) && !is_canonical_status_bar(child));
    if let Some(index) = noncanonical_index {
        let root_id = root.id_str().to_string();
        let fill_hex = op_editor_core::first_solid_fill_hex(root)
            .unwrap_or("#ffffff")
            .to_string();
        let width = root.width_px().unwrap_or(390.0);
        if let Ok(bar) =
            op_orchestrator::scaffold::mobile_status_bar_node(&root_id, &fill_hex, width)
        {
            if let Some(children) = root.children_mut() {
                children[index] = bar;
                return Some(
                    "The status bar you built was replaced with the standard iOS status bar                      (62px, role=status-bar) - do NOT rebuild or restyle it."
                        .to_string(),
                );
            }
        }
        return None;
    }
    if root
        .children()
        .into_iter()
        .flatten()
        .any(is_status_bar_node)
    {
        return None;
    }
    let root_id = root.id_str().to_string();
    let fill_hex = op_editor_core::first_solid_fill_hex(root)
        .unwrap_or("#ffffff")
        .to_string();
    let width = root.width_px().unwrap_or(390.0);
    let bar = op_orchestrator::scaffold::mobile_status_bar_node(&root_id, &fill_hex, width).ok()?;
    root.children_mut()?.insert(0, bar);
    Some(
        "A standard iOS status bar (62px, role=status-bar) was pre-inserted as the root's \
         first child - do NOT create another status bar; start your content below it."
            .to_string(),
    )
}

/// The injected/scaffold chrome shape: role tag + the Time/Levels pair.
/// Anything else that merely NAMES itself a status bar is a hand-rolled
/// variant slated for replacement.
fn is_canonical_status_bar(node: &PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        && node.children().is_some_and(|children| {
            children
                .iter()
                .any(|c| c.base().name.as_deref() == Some("Levels"))
        })
}

fn is_status_bar_node(node: &PenNode) -> bool {
    if node.base().role.as_deref() == Some("status-bar") {
        return true;
    }
    node.base()
        .name
        .as_deref()
        .is_some_and(|name| name.to_ascii_lowercase().contains("status bar"))
}

/// Contract sweep, every batch: once a root carries the canonical status
/// bar as a direct child, any OTHER status-bar-looking node in that root is
/// removed on the spot. The first-batch injection hook can't see later
/// batches — measured (test0711-22 23:07 run): the model hand-built a
/// second "21:30" bar inside the Header several batches in, which then sat
/// on screen until finalize. OS chrome is a contract, so the duplicate is
/// removed deterministically rather than echoed.
fn remove_nested_duplicate_status_bars(state: &mut EditorState) -> usize {
    let mut removed = 0;
    for root in state.active_children_mut() {
        let has_canonical = root
            .children()
            .into_iter()
            .flatten()
            .any(is_canonical_status_bar);
        if !has_canonical {
            continue;
        }
        fn prune(node: &mut PenNode, keep_canonical: bool, removed: &mut usize) {
            let Some(children) = node.children_mut() else {
                return;
            };
            children.retain(|child| {
                let duplicate = is_status_bar_node(child)
                    && !(keep_canonical && is_canonical_status_bar(child));
                if duplicate {
                    *removed += 1;
                }
                !duplicate
            });
            for child in children {
                prune(child, false, removed);
            }
        }
        prune(root, true, &mut removed);
    }
    removed
}

fn seed_root_frame_if_needed(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
    target: RootSeedTarget,
) -> Option<String> {
    let root = root_seed_candidate_mut(state, ids_before)?;
    let width_before = root.width_px();
    let height_before = root.height_px();
    if width_before.is_some() && height_before.is_some() {
        return None;
    }

    let (target_width, target_height) = target.dimensions();
    if width_before.is_none() {
        root.set_width_px(target_width);
    }
    if height_before.is_none() {
        root.set_height_px(target_height);
    }
    default_root_layout_to_vertical(root);

    let width = root.width_px().unwrap_or(target_width);
    let height = root.height_px().unwrap_or(target_height);
    Some(format!(
        "root seeded to {}x{} - grow height if content exceeds.",
        format_seed_dimension(width),
        format_seed_dimension(height)
    ))
}

fn root_seed_candidate_mut<'a>(
    state: &'a mut EditorState,
    ids_before: &HashSet<String>,
) -> Option<&'a mut PenNode> {
    let roots = state.active_children_mut();
    let new_root_index = roots
        .iter()
        .position(|node| !ids_before.contains(node.id_str()) && matches!(node, PenNode::Frame(_)));
    if let Some(index) = new_root_index {
        return roots.get_mut(index);
    }
    if roots.len() == 1 && matches!(roots[0], PenNode::Frame(_)) {
        roots.get_mut(0)
    } else {
        None
    }
}

fn default_root_layout_to_vertical(node: &mut PenNode) {
    if let PenNode::Frame(frame) = node {
        if frame.container.layout.is_none() {
            frame.container.layout = Some(LayoutMode::Vertical);
        }
    }
}

fn format_seed_dimension(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

/// Build a registry carrying only the requested design tool — snapshot
/// registered against the live state so read tools see prior writes.
fn design_tool_registry(state: &EditorState, requested: &str) -> ToolRegistry {
    use crate::mcp_serve::export_tool::export_nodes_snapshot;
    use crate::mcp_serve::screenshot_tool::get_screenshot_snapshot;

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
        "get_screenshot" => r.register(Box::new(get_screenshot_snapshot(state))),
        "export_nodes" => r.register(Box::new(export_nodes_snapshot(state))),
        "spawn_agents" => r.register(Box::new(op_mcp::spawn_agents_snapshot())),
        "ToolSearch" => r.register(Box::new(op_mcp::tool_search_snapshot(
            schemas::TOOL_SCHEMAS,
        ))),
        _ => {}
    }
    r
}

fn should_register_batch_reveals(name: &str, indicator_epoch: Option<u64>) -> bool {
    indicator_epoch.is_some() && name == "batch_design"
}

fn collect_active_node_ids(state: &EditorState) -> HashSet<String> {
    let mut out = HashSet::new();
    for node in state.active_children() {
        collect_node_ids(node, &mut out);
    }
    out
}

fn collect_node_ids(node: &PenNode, out: &mut HashSet<String>) {
    out.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_node_ids(child, out);
        }
    }
}

fn register_new_node_reveals(
    ids_before: &HashSet<String>,
    state: &EditorState,
    indicator_epoch: Option<u64>,
    reveal_started_ms: u64,
) {
    let Some(epoch) = indicator_epoch else {
        return;
    };
    let mut stream = RevealStream {
        index: 0,
        next_start_ms: reveal_started_ms,
    };
    for node in state.active_children() {
        register_node_reveals(
            node,
            ids_before,
            epoch,
            reveal_started_ms,
            0,
            None,
            &mut stream,
        );
    }
}

struct RevealStream {
    index: u64,
    next_start_ms: u64,
}

fn register_node_reveals(
    node: &PenNode,
    ids_before: &HashSet<String>,
    epoch: u64,
    reveal_started_ms: u64,
    depth: u64,
    parent_reveal_start_ms: Option<u64>,
    stream: &mut RevealStream,
) {
    let id = node.id_str();
    let mut own_reveal_start_ms = parent_reveal_start_ms;
    if !ids_before.contains(id) && should_reveal_node(node, depth) {
        let own_stream_index = stream.index;
        stream.index += 1;
        let base_start = reveal_started_ms
            + op_editor_core::agent_indicators::reveal_offset_ms(depth, own_stream_index);
        let child_runway_start = parent_reveal_start_ms
            .map(|started_at| {
                started_at.saturating_add(op_editor_core::agent_indicators::REVEAL_CHILD_RUNWAY_MS)
            })
            .unwrap_or(reveal_started_ms);
        let started_at = base_start.max(child_runway_start).max(stream.next_start_ms);
        op_editor_core::agent_indicators::add_reveal(epoch, id, started_at);
        stream.next_start_ms =
            started_at.saturating_add(op_editor_core::agent_indicators::REVEAL_STAGGER_MS);
        own_reveal_start_ms = Some(started_at);
    }
    if let Some(children) = node.children() {
        for child in children {
            register_node_reveals(
                child,
                ids_before,
                epoch,
                reveal_started_ms,
                depth + 1,
                own_reveal_start_ms,
                stream,
            );
        }
    }
}

fn should_reveal_node(node: &PenNode, depth: u64) -> bool {
    depth == 0 || node_has_own_visual(node) || node_is_named_structure(node)
}

fn node_has_own_visual(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => {
            container_has_own_visual(&n.container) || n.image_search_query.is_some()
        }
        PenNode::Group(n) => container_has_own_visual(&n.container),
        PenNode::Rectangle(n) => container_has_own_visual(&n.container),
        PenNode::Ref(_) => false,
        PenNode::Text(n) => match &n.content {
            TextContent::Plain(s) => !s.is_empty(),
            TextContent::Styled(segments) => !segments.is_empty(),
        },
        _ => true,
    }
}

fn container_has_own_visual(container: &ContainerProps) -> bool {
    container
        .fill
        .as_ref()
        .is_some_and(|fills| !fills.is_empty())
        || container.stroke.is_some()
        || container
            .effects
            .as_ref()
            .is_some_and(|effects| !effects.is_empty())
}

fn node_is_named_structure(node: &PenNode) -> bool {
    if !node.is_container() {
        return false;
    }
    let base = node.base();
    base.role.as_deref().is_some_and(|role| !role.is_empty())
        || base.name.as_deref().is_some_and(|name| !name.is_empty())
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
struct ContrastIssue {
    #[serde(rename = "nodeId")]
    node_id: String,
    #[serde(rename = "nodeName")]
    node_name: Option<String>,
    fg: String,
    bg: String,
    ratio: f64,
    target: f64,
}

const CONTRAST_AA_TARGET: f64 = 4.5;
const CONTRAST_ICON_TARGET: f64 = 3.0;

/// Structure echo for abandoned rebuilds: TWO top-level frames with the
/// same name means the model started a fresh copy instead of filling the
/// existing root (measured: MiniMax-M3 left the original `Explore` with an
/// empty AppContent and built everything in a second `Explore` — the user
/// sees a blank artboard mid-run). Finalize's duplicate-root pass repairs
/// the END state, but the in-loop model should merge NOW.
fn scan_duplicate_root_issues(nodes: &[PenNode]) -> Vec<String> {
    use std::collections::HashMap;
    let mut by_name: HashMap<&str, Vec<&PenNode>> = HashMap::new();
    for node in nodes {
        if let PenNode::Frame(_) = node {
            if let Some(name) = node.base().name.as_deref() {
                if !name.trim().is_empty() {
                    by_name.entry(name).or_default().push(node);
                }
            }
        }
    }
    let mut out = Vec::new();
    for (name, dupes) in by_name {
        if dupes.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = dupes.iter().map(|n| n.id_str()).collect();
        out.push(format!(
            "duplicate top-level roots named \"{name}\" ({}) — you rebuilt a copy instead of \
             filling the existing frame. Move your content into ONE root with M() and D() the \
             abandoned empty copy; never leave both.",
            ids.join(", ")
        ));
    }
    out.sort();
    out
}

/// Contract echo for broken icons: an `icon_font` whose `iconFontName` is
/// missing, empty, or a FONT FAMILY name ("lucide" / "feather" /
/// "material symbols …") renders as the fallback dot — the model wrote the
/// family into the glyph field (measured: test0711-1.op shipped every icon
/// as `iconFontName:"lucide"` with no glyph anywhere). The intended glyph
/// cannot be recovered deterministically, so this echoes the offending ids
/// for the in-loop model to repair with `U()`.
/// Hairline "activity ring" echo: a cluster of large concentric ellipses
/// stroked ~1px reads as faint wireframe circles, not progress rings
/// (measured: GLM-5.2 test0711-2.op stacked six 1px ellipses for the
/// Today's Activity ring). Ring thickness is the model's design intent, so
/// this echoes instead of auto-fixing.
/// Inventory of still-empty named shells — the skeleton-first protocol's
/// countdown. Informational (not an "issue"): intermediate batches SHOULD
/// have empty shells; the model uses the list to know what remains and to
/// never end the turn with one unfilled (measured: an aborted run shipped
/// an empty TabBar + MiniPlayer, test0711-22).
/// A header-named row holding ONLY icons while its title text sits outside
/// as a SIBLING — the bell floats alone in a full-width strip above the
/// greeting (measured: "Header Row" = [bell], "Good evening" outside it,
/// test0711-22). Which text belongs in the row is intent, so this echoes.
fn scan_header_icon_row_issues(nodes: &[PenNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if out.len() >= 4 {
                return;
            }
            let Some(children) = node.children() else {
                continue;
            };
            let has_text_sibling_ctx = children.iter().any(|c| matches!(c, PenNode::Text(_)));
            for child in children {
                let name = child.base().name.as_deref().unwrap_or("");
                if !name.to_ascii_lowercase().contains("header") {
                    continue;
                }
                let Some(row_children) = child.children() else {
                    continue;
                };
                let icons_only = !row_children.is_empty()
                    && row_children
                        .iter()
                        .all(|c| matches!(c, PenNode::IconFont(_)));
                if icons_only && has_text_sibling_ctx {
                    out.push(format!(
                        "{} ({}): contains ONLY icons while the title text sits outside as a                          sibling - M() the title INTO this row (layout horizontal,                          justifyContent space_between) so the greeting and the icons share                          one line",
                        name,
                        child.id_str()
                    ));
                }
            }
            walk(children, out);
        }
    }
    walk(nodes, &mut out);
    out
}

fn scan_empty_shells(nodes: &[PenNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if out.len() >= 12 {
                return;
            }
            if let Some(children) = node.children() {
                let named = node.base().name.as_deref().unwrap_or("");
                if children.is_empty()
                    && !named.is_empty()
                    && node.base().role.as_deref() != Some("status-bar")
                {
                    out.push(named.to_string());
                } else {
                    walk(children, out);
                }
            }
        }
    }
    walk(nodes, &mut out);
    out
}

fn scan_ring_issues(nodes: &[PenNode]) -> Vec<String> {
    const MIN_RING_SIZE: f64 = 48.0;
    const HAIRLINE: f32 = 2.5;
    let mut out = op_design_lint::detect_missing_progress_rings(nodes)
        .into_iter()
        .take(4)
        .map(|missing| {
            format!(
                "missing-progress-ring at {} ({}): numeric metric has no visible circle or arc - author ellipse/arc geometry or a painted circular frame",
                missing.node_id, missing.node_name
            )
        })
        .collect::<Vec<_>>();
    fn hairline_ring(node: &PenNode) -> bool {
        let PenNode::Ellipse(ellipse) = node else {
            return false;
        };
        if node.width_px().unwrap_or(0.0) < MIN_RING_SIZE {
            return false;
        }
        matches!(
            ellipse.stroke.as_ref().map(|s| &s.thickness),
            Some(jian_ops_schema::style::StrokeThickness::Uniform(t)) if *t <= HAIRLINE
        )
    }
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if out.len() >= 4 {
                return;
            }
            if let Some(children) = node.children() {
                let hairlines = children.iter().filter(|c| hairline_ring(c)).count();
                if hairlines >= 2 {
                    out.push(format!(
                        "{}: {hairlines} large ellipses stroked <=2px look like faint wireframe                          circles, not progress rings - give each ring a thick stroke                          (thickness 8-12), muted track + accent progress",
                        node.id_str()
                    ));
                }
                walk(children, out);
            }
        }
    }
    walk(nodes, &mut out);
    out
}

fn scan_icon_issues(nodes: &[PenNode]) -> Vec<String> {
    const FAMILY_NAMES: [&str; 3] = ["lucide", "feather", "material symbols"];
    let mut out = Vec::new();
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if out.len() >= 12 {
                return;
            }
            if let PenNode::IconFont(icon) = node {
                let name = icon.icon_font_name.trim();
                let lowered = name.to_ascii_lowercase();
                let family_as_glyph = FAMILY_NAMES
                    .iter()
                    .any(|family| lowered.starts_with(family));
                if name.is_empty() || family_as_glyph {
                    out.push(format!(
                        "icon {}: iconFontName is {} — it must be the GLYPH name \
                         (e.g. \"home\", \"compass\"), not the font family",
                        icon.base.id,
                        if name.is_empty() {
                            "missing".to_string()
                        } else {
                            format!("\"{name}\"")
                        }
                    ));
                }
            }
            if let Some(children) = node.children() {
                walk(children, out);
            }
        }
    }
    walk(nodes, &mut out);
    out
}

fn scan_contrast_issues(
    nodes: &[PenNode],
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
) -> Vec<ContrastIssue> {
    let mut candidates = Vec::new();
    let mut bg_stack = Vec::new();
    for node in nodes {
        collect_contrast_candidates(node, variables, theme, &mut bg_stack, &mut candidates);
    }

    let pairs: Vec<(String, String, f64)> = candidates
        .iter()
        .map(|candidate| (candidate.fg.clone(), candidate.bg.clone(), candidate.target))
        .collect();
    let report = op_ai_skills::color::contrast::scan_pairs(&pairs);
    let mut violations = report.violations.into_iter().peekable();
    let mut issues = Vec::new();
    for candidate in candidates {
        let Some(violation) = violations.peek() else {
            break;
        };
        if violation.fg == candidate.fg
            && violation.bg == candidate.bg
            && (violation.target - candidate.target).abs() < f64::EPSILON
        {
            let violation = violations.next().expect("peeked violation");
            issues.push(ContrastIssue {
                node_id: candidate.node_id,
                node_name: candidate.node_name,
                fg: violation.fg,
                bg: violation.bg,
                ratio: violation.ratio,
                target: violation.target,
            });
        }
    }
    issues
}

#[derive(Debug, Clone, PartialEq)]
struct ContrastCandidate {
    node_id: String,
    node_name: Option<String>,
    fg: String,
    bg: String,
    target: f64,
}

fn collect_contrast_candidates(
    node: &PenNode,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    bg_stack: &mut Vec<String>,
    out: &mut Vec<ContrastCandidate>,
) {
    let opacity = resolved_node_opacity(node, variables, theme);
    let pushed_bg = container_background_hex(
        node,
        variables,
        theme,
        bg_stack.last().map(String::as_str),
        opacity,
    );
    if let Some(bg) = pushed_bg.as_ref() {
        bg_stack.push(bg.clone());
    }

    let foreground = match node {
        PenNode::Text(text) => Some((&text.fill, CONTRAST_AA_TARGET)),
        PenNode::IconFont(icon) => Some((&icon.fill, CONTRAST_ICON_TARGET)),
        _ => None,
    };
    if let (Some((fill, target)), Some(bg)) = (foreground, bg_stack.last()) {
        if let Some(fg) = first_solid_hex(fill, variables, theme, Some(bg), opacity) {
            out.push(ContrastCandidate {
                node_id: node.id_str().to_string(),
                node_name: node.base().name.clone(),
                fg,
                bg: bg.clone(),
                target,
            });
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_contrast_candidates(child, variables, theme, bg_stack, out);
        }
    }

    if pushed_bg.is_some() {
        bg_stack.pop();
    }
}

fn container_background_hex(
    node: &PenNode,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    parent_bg: Option<&str>,
    node_opacity: f64,
) -> Option<String> {
    let fill = match node {
        PenNode::Frame(n) => &n.container.fill,
        PenNode::Group(n) => &n.container.fill,
        PenNode::Rectangle(n) => &n.container.fill,
        PenNode::Tabs(n) => &n.fill,
        _ => return None,
    };
    first_solid_hex(fill, variables, theme, parent_bg, node_opacity)
}

fn first_solid_hex(
    fill: &Option<Vec<PenFill>>,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    background: Option<&str>,
    node_opacity: f64,
) -> Option<String> {
    fill.as_ref()?.iter().find_map(|fill| match fill {
        PenFill::Solid(body) => resolved_hex(
            &body.color,
            variables,
            theme,
            background,
            node_opacity * f64::from(body.opacity.unwrap_or(1.0)),
        ),
        _ => None,
    })
}

fn resolved_node_opacity(
    node: &PenNode,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
) -> f64 {
    match node.base().opacity.as_ref() {
        Some(NumberOrExpression::Number(opacity)) => opacity.clamp(0.0, 1.0),
        Some(NumberOrExpression::Expression(reference)) => {
            op_editor_core::variables_resolve::resolve_numeric_ref(reference, variables, theme)
                .unwrap_or(1.0)
                .clamp(0.0, 1.0)
        }
        None => 1.0,
    }
}

fn resolved_hex(
    color: &str,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    background: Option<&str>,
    opacity: f64,
) -> Option<String> {
    let resolved = op_editor_core::variables_resolve::resolve_color_ref(color, variables, theme)?;
    composite_color(resolved.trim(), background.unwrap_or("#FFFFFF"), opacity)
}

fn composite_color(foreground: &str, background: &str, opacity: f64) -> Option<String> {
    let (fg_r, fg_g, fg_b) = op_editor_core::parse_hex_rgb(foreground)?;
    let (bg_r, bg_g, bg_b) = op_editor_core::parse_hex_rgb(background)?;
    let alpha = f64::from(op_editor_core::parse_hex_alpha(foreground)) * opacity.clamp(0.0, 1.0);
    let blend = |foreground: f32, background: f32| {
        ((f64::from(foreground) * alpha + f64::from(background) * (1.0 - alpha)) * 255.0).round()
            as u8
    };
    Some(format!(
        "#{:02X}{:02X}{:02X}",
        blend(fg_r, bg_r),
        blend(fg_g, bg_g),
        blend(fg_b, bg_b)
    ))
}

fn contrast_hint(issues: &[ContrastIssue]) -> String {
    let text_count = issues
        .iter()
        .filter(|issue| (issue.target - CONTRAST_AA_TARGET).abs() < f64::EPSILON)
        .count();
    let icon_count = issues.len().saturating_sub(text_count);
    format!(
        "{} foreground/background pair(s) below their target (text: {text_count} below {CONTRAST_AA_TARGET}:1; icons: {icon_count} below {CONTRAST_ICON_TARGET}:1). Use a deliberate semantic foreground with sufficient contrast.",
        issues.len()
    )
}

fn reveal_now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

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

#[cfg(test)]
mod tests {
    #[test]
    fn web_app_prompts_are_not_mobile_seeded() {
        // A dashboard "web app" is a desktop product — it must not be
        // seeded 390x844 (regression: the bare "app" word-match did).
        assert!(!super::root_seed_prompt_is_mobile(
            "Technical dashboard web app for a utilities company"
        ));
        assert!(!super::root_seed_prompt_is_mobile(
            "Luxury webapp for managing barbershop clients"
        ));
    }

    #[test]
    fn app_prompts_are_mobile_seeded() {
        assert!(super::root_seed_prompt_is_mobile(
            "Technical dashboard app for a utilities company"
        ));
        // "web app" phrasing is covered (negatively) by
        // web_app_prompts_are_not_mobile_seeded below.
        assert!(super::root_seed_prompt_is_mobile(
            "mobile companion for our web app"
        ));
        assert!(super::root_seed_prompt_is_mobile(
            "phone booking flow for a travel brand"
        ));
        assert!(super::root_seed_prompt_is_mobile(
            "Design a travel booking mobile app explore page"
        ));
        assert!(super::root_seed_prompt_is_mobile("设计一个手机端首页"));
    }

    use super::*;

    #[test]
    fn design_tool_defs_cover_all_14_tools_with_schema_parity() {
        let defs = design_tool_defs();

        // All 15 tools are present, every one MCP-sourced.
        assert_eq!(defs.len(), 15, "expected 15 design tool defs");
        for (name, _) in DESIGN_TOOLS {
            assert!(
                defs.iter().any(|d| d.name == *name),
                "missing design tool def for {name}"
            );
        }

        // PARITY: for each tool, the input_schema_json in the def must equal
        // the inputSchema value from TOOL_SCHEMAS (as parsed JSON), so
        // in-process defs stay byte-equal to the MCP server.
        for def in defs.iter() {
            // Find the matching TOOL_SCHEMAS entry.
            let schema_entry = schemas::TOOL_SCHEMAS
                .iter()
                .find(|entry| {
                    let v: serde_json::Value = serde_json::from_str(entry).unwrap();
                    v.get("name").and_then(|n| n.as_str()) == Some(def.name.as_str())
                })
                .unwrap_or_else(|| panic!("design tool {} not found in TOOL_SCHEMAS", def.name));

            // Extract the canonical inputSchema from TOOL_SCHEMAS.
            let canonical: serde_json::Value = serde_json::from_str(schema_entry).unwrap();
            let canonical_schema = canonical.get("inputSchema").unwrap_or_else(|| {
                panic!("TOOL_SCHEMAS entry for {} missing inputSchema", def.name)
            });

            // Parse the def's input_schema_json and compare as Value.
            let def_schema: serde_json::Value = serde_json::from_str(&def.input_schema_json)
                .unwrap_or_else(|e| {
                    panic!("def.input_schema_json for {} unparseable: {e}", def.name)
                });

            assert_eq!(
                def_schema, *canonical_schema,
                "inputSchema mismatch for {}: in-process def != TOOL_SCHEMAS",
                def.name
            );
        }

        // Every DESIGN_TOOLS entry must exist in TOOL_SCHEMAS (no orphans).
        for (name, _) in DESIGN_TOOLS {
            let found = schemas::TOOL_SCHEMAS.iter().any(|entry| {
                let v: serde_json::Value = serde_json::from_str(entry).unwrap();
                v.get("name").and_then(|n| n.as_str()) == Some(*name)
            });
            assert!(found, "design tool {name} is not in TOOL_SCHEMAS — orphan!");
        }
    }

    #[test]
    fn execute_design_rejects_tools_outside_the_design_set() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(&mut state, "delete_page", "{}");
        assert!(result.is_error);
        assert!(!mutated);
        assert!(result.content.contains("not available in design agent"));
    }

    #[test]
    fn execute_design_read_tool_returns_success_envelope() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(&mut state, "get_editor_state", "{}");
        assert!(!result.is_error, "got {}", result.content);
        assert!(!mutated, "read tools never mutate");
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(v["success"], serde_json::Value::Bool(true));
    }

    #[test]
    fn execute_design_batch_design_inserts_frame_and_mutates() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',width:120,height:80})"}"#,
        );
        assert!(!result.is_error, "batch_design failed: {}", result.content);
        assert!(mutated, "batch_design must mutate the document");

        // The active page must now have at least one child (the inserted frame).
        assert!(
            !state.active_children().is_empty(),
            "doc must have a frame after batch_design"
        );
    }

    #[test]
    fn execute_design_batch_design_registers_reveals_when_epoch_is_set() {
        use op_editor_core::agent_indicators;

        agent_indicators::clear();
        let epoch = agent_indicators::begin();
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool_with_reveals(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Root',width:120,height:80})\nbox=I(root,{type:'rectangle',name:'Box',width:80,height:20})"}"#,
            Some(epoch),
        );
        assert!(!result.is_error, "batch_design failed: {}", result.content);
        assert!(mutated, "batch_design must mutate the document");

        let ids: Vec<String> = collect_active_node_ids(&state).into_iter().collect();
        assert!(ids.len() >= 2, "batch inserted a subtree, got {ids:?}");
        let snapshot = agent_indicators::snapshot();
        for id in ids {
            assert!(
                snapshot.reveals.contains_key(&id),
                "newly inserted node {id} should have a reveal: {:?}",
                snapshot.reveals
            );
        }
        agent_indicators::end_if_epoch(epoch);
        agent_indicators::clear();
    }

    #[test]
    fn execute_design_batch_design_attaches_per_batch_layout_feedback() {
        // A batch that lands an OVERFLOWING table (5×240 fixed columns in a
        // 600px root) must come back with `layoutIssues` — the per-batch
        // geometry feedback the model repairs in-process.
        let mut state = EditorState::new();
        let ops = r#"{"operations":"root=I(null,{\"type\":\"frame\",\"name\":\"Page\",\"width\":600,\"height\":\"fit_content\",\"layout\":\"vertical\",\"children\":[{\"type\":\"frame\",\"name\":\"Client Table\",\"layout\":\"vertical\",\"width\":\"fill_container\",\"children\":[{\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\",\"gap\":16,\"width\":\"fill_container\",\"height\":24,\"children\":[{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20}]},{\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\",\"gap\":16,\"width\":\"fill_container\",\"height\":24,\"children\":[{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20}]}]}]})"}"#;
        let (result, mutated) = execute_design_tool(&mut state, "batch_design", ops);
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        let issues = v["layoutIssues"].as_array().expect("layoutIssues attached");
        assert!(
            issues
                .iter()
                .any(|i| i.as_str().unwrap_or("").contains("column widths")),
            "table overflow reported, got {issues:?}"
        );
        assert!(v["layoutHint"].is_string(), "actionable hint attached");
    }

    #[test]
    fn execute_design_clean_batch_attaches_no_layout_feedback() {
        // A geometrically clean batch must NOT carry layoutIssues noise.
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',width:400,height:300})"}"#,
        );
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(
            v.get("layoutIssues").is_none(),
            "clean layout must not attach issues: {}",
            result.content
        );
    }

    #[test]
    fn flat_script_reports_missing_layout_after_the_forest_is_assembled() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r#"{"script":"const section=I(null,{type:'frame',name:'Popular',width:360,height:240}); I(section,{type:'text',name:'Title',content:'Popular'}); I(section,{type:'frame',name:'Rail',layout:'horizontal',width:'fill_container',height:180});"}"#,
        );
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);

        let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        let questions = value["intentQuestions"]
            .as_array()
            .expect("missing layout is reported after flat I(parent,obj) assembly");
        assert!(questions.iter().any(|question| question
            .as_str()
            .is_some_and(|line| line.contains("Popular") && line.contains("no layout"))));

        let root = state.active_children().first().expect("section inserted");
        let PenNode::Frame(frame) = root else {
            panic!("expected frame")
        };
        assert_eq!(
            frame.container.layout, None,
            "reporting ambiguity must not silently write vertical"
        );
    }

    #[test]
    fn contrast_scanner_flags_bad_pair() {
        let bad_root: PenNode = serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "root",
            "name": "Card",
            "fill": [{ "type": "solid", "color": "#888888" }],
            "children": [{
                "type": "text",
                "id": "title",
                "name": "Title",
                "content": "Low contrast",
                "fill": [{ "type": "solid", "color": "#777777" }]
            }]
        }))
        .unwrap();
        let issues = scan_contrast_issues(&[bad_root], None, &std::collections::BTreeMap::new());
        assert_eq!(issues.len(), 1, "exactly one bad text/background pair");
        assert_eq!(issues[0].node_id, "title");
        assert_eq!(issues[0].node_name.as_deref(), Some("Title"));
        assert_eq!(issues[0].fg, "#777777");
        assert_eq!(issues[0].bg, "#888888");
        assert_eq!(issues[0].target, 4.5);
        assert!(issues[0].ratio < issues[0].target);

        let passing_root: PenNode = serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "root",
            "name": "Card",
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [{
                "type": "text",
                "id": "title",
                "name": "Title",
                "content": "Readable",
                "fill": [{ "type": "solid", "color": "#111111" }]
            }]
        }))
        .unwrap();
        assert!(
            scan_contrast_issues(&[passing_root], None, &std::collections::BTreeMap::new())
                .is_empty()
        );
    }

    #[test]
    fn contrast_scanner_resolves_tokens_and_alpha_for_icons() {
        let root: PenNode = serde_json::from_value(serde_json::json!({
            "type": "frame", "id": "root", "name": "Search", "layout": "horizontal",
            "fill": [{"type":"solid","color":"$--accent"}], "children": [{
                "type": "frame", "id": "filter", "name": "Filter", "layout": "horizontal",
                "fill": [{"type":"solid","color":"#EA580C15"}], "children": [{
                    "type": "icon_font", "id": "icon", "name": "Filter Icon",
                    "iconFontName": "sliders-horizontal", "width": 20, "height": 20,
                    "fill": [{"type":"solid","color":"$--white"}]
                }]
            }]
        }))
        .unwrap();
        let variables: std::collections::BTreeMap<
            String,
            jian_ops_schema::variable::VariableDefinition,
        > = serde_json::from_value(serde_json::json!({
            "--accent": {"type":"color","value":"#F5F5F5"},
            "--white": {"type":"color","value":"#FFFFFF"}
        }))
        .unwrap();

        let issues = scan_contrast_issues(
            &[root],
            Some(&variables),
            &std::collections::BTreeMap::new(),
        );

        let issue = issues
            .iter()
            .find(|issue| issue.node_id == "icon")
            .expect("white icon on a translucent orange tint is reported");
        assert_eq!(issue.fg, "#FFFFFF");
        assert_eq!(issue.target, CONTRAST_ICON_TARGET);
        assert!(issue.ratio < issue.target);
    }

    #[test]
    fn contrast_scanner_accounts_for_fill_and_node_opacity() {
        let root: PenNode = serde_json::from_value(serde_json::json!({
            "type":"frame", "id":"root", "fill":[{"type":"solid","color":"#000000"}],
            "children":[
                {"type":"text", "id":"fill-opacity", "content":"Dimmed",
                 "fill":[{"type":"solid","color":"#FFFFFF","opacity":0.2}]},
                {"type":"text", "id":"node-opacity", "content":"Also dimmed", "opacity":0.2,
                 "fill":[{"type":"solid","color":"#FFFFFF"}]}
            ]
        }))
        .unwrap();

        let issues = scan_contrast_issues(&[root], None, &std::collections::BTreeMap::new());

        assert!(issues.iter().any(|issue| issue.node_id == "fill-opacity"));
        assert!(issues.iter().any(|issue| issue.node_id == "node-opacity"));
    }

    #[test]
    fn batch_design_image_slot_feedback_requires_the_exact_slot_id() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r##"{"operations":"root=I(null,{type:'frame',name:'Playlist',layout:'vertical',width:320,height:200,fill:[{type:'solid',color:'#111111'}],children:[{type:'frame',name:'Cover',layout:'none',width:56,height:56,fill:[{type:'solid',color:'#222222'}]}]})"}"##,
        );
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);

        let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(
            value["imageSlots"].as_array().is_some_and(|slots| slots
                .iter()
                .any(|slot| slot.as_str().is_some_and(|line| line.contains("Cover")))),
            "explicit empty cover slot is surfaced: {}",
            result.content
        );
        let hint = value["layoutHint"].as_str().unwrap_or("");
        assert!(hint.contains("exact EMPTY slot id"), "{hint}");
        assert!(hint.contains("never its row/card container"), "{hint}");
    }

    #[test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Windows CI aborts in native text geometry while attaching batch contrast feedback"
    )]
    fn batch_design_result_carries_contrast_issues() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r##"{"operations":"root=I(null,{type:'frame',name:'Card',width:320,height:120,fill:[{type:'solid',color:'#888888'}],children:[{type:'text',name:'Title',content:'Low contrast',width:180,height:24,fill:[{type:'solid',color:'#777777'}]}]})"}"##,
        );
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);

        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        let issues = v["contrastIssues"]
            .as_array()
            .expect("contrastIssues attached");
        assert!(!issues.is_empty(), "bad contrast pair reported");
        assert_eq!(issues[0]["nodeName"], "Title");
        assert_eq!(issues[0]["fg"], "#777777");
        assert_eq!(issues[0]["bg"], "#888888");
        assert!(issues[0]["ratio"].as_f64().unwrap() < issues[0]["target"].as_f64().unwrap());
        assert!(
            v["contrastHint"]
                .as_str()
                .unwrap_or("")
                .contains("text: 1 below 4.5:1"),
            "actionable contrast hint attached: {}",
            result.content
        );
    }

    #[test]
    fn execute_design_first_batch_seeds_mobile_sizeless_root() {
        let mut state = EditorState::new();
        let mut guard = RootSeedGuard::from_prompt("travel itinerary app");
        let (result, mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Mobile Page'})"}"#,
            None,
            Some(&mut guard),
        );

        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let root = only_root_frame(&state);
        assert_eq!(root.width_px(), Some(390.0));
        assert_eq!(root.height_px(), Some(844.0));
        assert!(root_frame_layout_is_vertical(root));
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(
            v["layoutHint"]
                .as_str()
                .unwrap_or("")
                .contains("root seeded to 390x844"),
            "seed hint must be visible to the next batch: {}",
            result.content
        );
    }

    #[test]
    fn execute_design_first_batch_seeds_desktop_sizeless_root() {
        let mut state = EditorState::new();
        let mut guard = RootSeedGuard::from_prompt("build a SaaS analytics dashboard");
        let (result, mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Dashboard'})"}"#,
            None,
            Some(&mut guard),
        );

        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let root = only_root_frame(&state);
        assert_eq!(root.width_px(), Some(1440.0));
        assert_eq!(root.height_px(), Some(900.0));
        assert!(root_frame_layout_is_vertical(root));
    }

    #[test]
    fn execute_design_root_seed_preserves_authored_numeric_width() {
        let mut state = EditorState::new();
        let mut guard = RootSeedGuard::from_prompt("mobile hotel booking");
        let (result, mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Phone',width:320,height:'fit_content'})"}"#,
            None,
            Some(&mut guard),
        );

        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let root = only_root_frame(&state);
        assert_eq!(
            root.width_px(),
            Some(320.0),
            "authored numeric width must stay untouched"
        );
        assert_eq!(root.height_px(), Some(844.0));
    }

    #[test]
    fn execute_design_mobile_first_batch_injects_status_bar_chrome() {
        // Chrome parity with the orchestrator scaffold: even when the model
        // authored explicit root dimensions (so size seeding is skipped),
        // the mobile root still gets the pre-inserted status bar as its
        // FIRST child, and the hint tells the model not to build another.
        let mut state = EditorState::new();
        let mut guard = RootSeedGuard::from_prompt("mobile fitness tracker home");
        let (result, mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Home',width:390,height:844,layout:'vertical'})"}"#,
            None,
            Some(&mut guard),
        );

        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let root = only_root_frame(&state);
        let first = &root.children().expect("root children")[0];
        assert_eq!(
            first.base().role.as_deref(),
            Some("status-bar"),
            "status bar must be the root's first child"
        );
        assert_eq!(first.base().name.as_deref(), Some("Status Bar"));
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(
            v["layoutHint"]
                .as_str()
                .unwrap_or("")
                .contains("do NOT create another status bar"),
            "chrome hint must be visible to the next batch: {}",
            result.content
        );
    }

    #[test]
    fn execute_design_mobile_batch_canonicalizes_model_authored_status_bar() {
        // The model built its own status bar in the first batch — the guard
        // must NOT stack a second one, and the hand-rolled variant is
        // replaced in place with the canonical chrome (every measured
        // hand-built bar deviated visibly from the iOS reference).
        let mut state = EditorState::new();
        let mut guard = RootSeedGuard::from_prompt("iphone travel app");
        let (result, mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Screen',width:390,height:844,layout:'vertical',children:[{type:'frame',name:'Status Bar',width:'fill_container',height:62}]})"}"#,
            None,
            Some(&mut guard),
        );

        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let root = only_root_frame(&state);
        let bars = root
            .children()
            .expect("root children")
            .iter()
            .filter(|c| {
                c.base()
                    .name
                    .as_deref()
                    .is_some_and(|n| n.to_ascii_lowercase().contains("status bar"))
            })
            .count();
        assert_eq!(bars, 1, "must not stack a second status bar");
        let bar = root
            .children()
            .expect("root children")
            .iter()
            .find(|c| c.base().role.as_deref() == Some("status-bar"))
            .expect("model-built bar replaced with the canonical status bar");
        assert!(
            bar.children()
                .is_some_and(|children| children
                    .iter()
                    .any(|c| c.base().name.as_deref() == Some("Levels"))),
            "canonical bar carries the Time/Levels structure"
        );
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(
            v["layoutHint"]
                .as_str()
                .unwrap_or("")
                .contains("replaced with the standard iOS status bar"),
            "replacement must be echoed so the model stops restyling it: {}",
            result.content
        );
    }

    #[test]
    fn execute_design_desktop_first_batch_gets_no_status_bar() {
        let mut state = EditorState::new();
        let mut guard = RootSeedGuard::from_prompt("SaaS analytics web app dashboard");
        let (result, mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Dashboard'})"}"#,
            None,
            Some(&mut guard),
        );

        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let root = only_root_frame(&state);
        let has_bar = root
            .children()
            .into_iter()
            .flatten()
            .any(|c| c.base().role.as_deref() == Some("status-bar"));
        assert!(!has_bar, "desktop roots must not get mobile chrome");
    }

    #[test]
    fn execute_design_root_seed_guard_consumes_after_first_successful_batch() {
        let mut state = EditorState::new();
        let mut guard = RootSeedGuard::from_prompt("phone onboarding flow");
        let (first, first_mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Root',width:390,height:844})"}"#,
            None,
            Some(&mut guard),
        );
        assert!(!first.is_error, "first batch failed: {}", first.content);
        assert!(first_mutated);

        let (second, second_mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            r#"{"operations":"second=I(null,{type:'frame',name:'Second',width:'fit_content',height:'fit_content'})"}"#,
            None,
            Some(&mut guard),
        );

        assert!(!second.is_error, "second batch failed: {}", second.content);
        assert!(second_mutated);
        let second_root = state
            .active_children()
            .iter()
            .find(|node| node.base().name.as_deref() == Some("Second"))
            .expect("second top-level frame exists");
        assert_eq!(
            second_root.width_px(),
            None,
            "second batch must not be seeded after the first success"
        );
        assert_eq!(
            second_root.height_px(),
            None,
            "second batch must not be seeded after the first success"
        );
        let v: serde_json::Value = serde_json::from_str(&second.content).unwrap();
        assert!(
            v.get("layoutHint")
                .and_then(|h| h.as_str())
                .is_none_or(|hint| !hint.contains("root seeded")),
            "second batch must not get another root seed hint: {}",
            second.content
        );
    }

    #[test]
    fn execute_design_tool_without_loop_root_seed_guard_does_not_seed() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Plain',width:'fit_content',height:'fit_content'})"}"#,
        );

        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let root = only_root_frame(&state);
        assert_eq!(root.width_px(), None);
        assert_eq!(root.height_px(), None);
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(
            v.get("layoutHint")
                .and_then(|h| h.as_str())
                .is_none_or(|hint| !hint.contains("root seeded")),
            "non-loop path must not inject root seed feedback: {}",
            result.content
        );
    }

    fn only_root_frame(state: &EditorState) -> &PenNode {
        let children = state.active_children();
        assert_eq!(children.len(), 1, "expected a single root frame");
        let root = &children[0];
        assert!(matches!(root, PenNode::Frame(_)), "expected frame root");
        root
    }

    fn root_frame_layout_is_vertical(node: &PenNode) -> bool {
        let PenNode::Frame(frame) = node else {
            return false;
        };
        matches!(
            frame.container.layout,
            Some(jian_ops_schema::node::container::LayoutMode::Vertical)
        )
    }

    // --- execute_agent_tool tests ---

    #[test]
    fn execute_agent_tool_routes_design_tool_to_design_surface() {
        // batch_design is a design-only tool — it must execute and mutate
        // via the design surface, not the CRUD surface.
        let mut state = EditorState::new();
        let (result, mutated) = execute_agent_tool(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',width:80,height:60})"}"#,
        );
        assert!(
            !result.is_error,
            "batch_design via agent router failed: {}",
            result.content
        );
        assert!(mutated, "batch_design must mutate via the design surface");
        assert!(
            !state.active_children().is_empty(),
            "a frame must exist after batch_design via execute_agent_tool"
        );
    }

    #[test]
    fn execute_agent_tool_routes_crud_tool_to_chat_surface() {
        // delete_node is a CRUD-only tool — it must route to execute_chat_tool.
        // With an unknown nodeId the chat surface returns an error (node not found),
        // which proves the CRUD path was taken rather than the design path that
        // would have returned "not available in design agent".
        let mut state = EditorState::new();
        let (result, mutated) =
            execute_agent_tool(&mut state, "delete_node", r#"{"nodeId":"nope"}"#);
        // The CRUD surface returns an error for an unknown node — NOT "not available in design agent".
        assert!(result.is_error, "unknown node delete must error");
        assert!(!mutated);
        assert!(
            !result.content.contains("not available in design agent"),
            "must have taken the CRUD path, not the design path"
        );
    }

    #[test]
    fn execute_agent_tool_unknown_name_returns_not_available_error() {
        // A name outside both sets falls through to execute_chat_tool
        // which returns "not available in chat".
        let mut state = EditorState::new();
        let (result, mutated) = execute_agent_tool(&mut state, "delete_page", "{}");
        assert!(result.is_error);
        assert!(!mutated);
        assert!(
            result.content.contains("not available in chat"),
            "unknown tools should report the CRUD surface's 'not available in chat' error, got: {}",
            result.content
        );
    }
}

#[cfg(test)]
mod icon_issue_tests {
    use super::*;

    #[test]
    fn family_name_as_glyph_and_missing_glyph_are_echoed() {
        // test0711-1.op regression: every icon shipped as
        // iconFontName:"lucide" (family in the glyph field) → fallback dots.
        let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
            { "type": "frame", "id": "root", "name": "R", "width": 100, "height": 100,
              "children": [
                { "type": "icon_font", "id": "bad1", "iconFontName": "lucide",
                  "width": 20, "height": 20 },
                { "type": "icon_font", "id": "bad2", "iconFontName": "",
                  "width": 20, "height": 20 },
                { "type": "icon_font", "id": "ok", "iconFontName": "compass",
                  "width": 20, "height": 20 }
              ] }
        ]))
        .expect("nodes");
        let issues = scan_icon_issues(&nodes);
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues[0].contains("bad1") && issues[0].contains("lucide"));
        assert!(issues[1].contains("bad2") && issues[1].contains("missing"));
        assert!(!issues.iter().any(|i| i.contains("\"ok\"")), "{issues:?}");
    }
}

#[cfg(test)]
mod duplicate_status_bar_tests {
    use super::remove_nested_duplicate_status_bars;
    use op_editor_core::PenNodeExt;

    #[test]
    fn nested_hand_built_status_bar_is_removed_once_canonical_exists() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(
            r##"{ "version": "1.0", "children": [{
                "type": "frame", "id": "root", "name": "Music Home",
                "width": 402, "height": 874, "layout": "vertical",
                "children": [
                    { "type": "frame", "id": "sb", "name": "Status Bar", "role": "status-bar",
                      "width": "fill_container", "height": 62,
                      "children": [
                        { "type": "text", "id": "time", "name": "Time", "content": "9:41",
                          "width": 54, "height": 22 },
                        { "type": "frame", "id": "lv", "name": "Levels", "width": 70, "height": 22 }
                      ] },
                    { "type": "frame", "id": "hdr", "name": "Header",
                      "width": "fill_container", "height": "fit_content",
                      "children": [
                        { "type": "frame", "id": "fake", "name": "Status Bar 2",
                          "width": "fill_container", "height": 44 },
                        { "type": "text", "id": "greet", "name": "Greeting",
                          "content": "Good evening", "width": 200, "height": 30 }
                      ] }
                ]
            }] }"##,
        )
        .expect("doc");
        let mut state = op_editor_core::EditorState::from_document(doc);
        let removed = remove_nested_duplicate_status_bars(&mut state);
        assert_eq!(removed, 1, "the nested hand-built bar is swept");
        let root = &state.active_children()[0];
        fn find<'a>(
            node: &'a jian_ops_schema::node::PenNode,
            id: &str,
        ) -> Option<&'a jian_ops_schema::node::PenNode> {
            if node.id_str() == id {
                return Some(node);
            }
            node.children()?.iter().find_map(|c| find(c, id))
        }
        assert!(find(root, "sb").is_some(), "canonical bar survives");
        assert!(find(root, "fake").is_none(), "nested duplicate removed");
        assert!(find(root, "greet").is_some(), "siblings untouched");
    }
}

#[cfg(test)]
mod ring_issue_tests {
    use super::scan_ring_issues;

    #[test]
    fn hairline_ring_cluster_is_echoed_and_thick_rings_are_not() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(
            r##"{ "version": "1.0", "children": [{
                "type": "frame", "id": "root", "name": "Screen",
                "width": 390, "height": 844,
                "children": [
                    { "type": "frame", "id": "ring", "name": "ActivityRing",
                      "width": 140, "height": 140, "layout": "none",
                      "children": [
                        { "type": "ellipse", "id": "e1", "width": 120, "height": 120,
                          "stroke": { "thickness": 1 } },
                        { "type": "ellipse", "id": "e2", "width": 120, "height": 120,
                          "stroke": { "thickness": 1 } }
                      ] },
                    { "type": "frame", "id": "ok", "name": "HealthyRing",
                      "width": 140, "height": 140, "layout": "none",
                      "children": [
                        { "type": "ellipse", "id": "e3", "width": 120, "height": 120,
                          "stroke": { "thickness": 10 } },
                        { "type": "ellipse", "id": "e4", "width": 120, "height": 120,
                          "stroke": { "thickness": 10 } }
                      ] }
                ]
            }] }"##,
        )
        .expect("doc");
        let issues = scan_ring_issues(&doc.children);
        assert_eq!(issues.len(), 1, "one cluster echoed: {issues:?}");
        assert!(
            issues[0].contains("ring") && issues[0].contains("thickness 8-12"),
            "echo names the cluster and the fix: {issues:?}"
        );
    }

    #[test]
    fn missing_ring_wrapper_is_echoed_by_builtin_scan() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(
            r##"{ "version": "1.0", "children": [{
                "type": "frame", "id": "steps-ring", "name": "Steps Ring",
                "width": 124, "height": 124, "layout": "vertical",
                "alignItems": "center", "justifyContent": "center",
                "children": [
                    {"type": "text", "id": "value", "content": "8,432"},
                    {"type": "text", "id": "label", "content": "steps"}
                ]
            }] }"##,
        )
        .expect("doc");

        let issues = scan_ring_issues(&doc.children);

        assert_eq!(issues.len(), 1, "one missing ring echoed: {issues:?}");
        assert!(issues[0].contains("missing-progress-ring"), "{issues:?}");
    }
}

#[cfg(test)]
mod duplicate_root_tests {
    use super::*;

    #[test]
    fn same_named_top_level_frames_are_echoed_once_per_name() {
        // test0711-1-m3.op shape: model abandoned the original `Explore`
        // (empty AppContent) and rebuilt everything in a second `Explore`.
        let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
            { "type": "frame", "id": "r1", "name": "Explore", "width": 390, "height": 844,
              "children": [ { "type": "frame", "id": "empty", "name": "AppContent",
                               "width": "fill_container", "height": "fit_content" } ] },
            { "type": "frame", "id": "r2", "name": "Explore", "width": 390,
              "height": "fit_content",
              "children": [ { "type": "frame", "id": "rich", "name": "AppContent",
                               "width": "fill_container", "height": "fit_content" } ] },
            { "type": "frame", "id": "solo", "name": "Profile", "width": 390, "height": 844,
              "children": [] }
        ]))
        .expect("nodes");
        let issues = scan_duplicate_root_issues(&nodes);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(
            issues[0].contains("Explore") && issues[0].contains("r1") && issues[0].contains("r2")
        );
        assert!(issues[0].contains("M()") && issues[0].contains("D()"));
        assert!(!issues[0].contains("Profile"));
    }
}
