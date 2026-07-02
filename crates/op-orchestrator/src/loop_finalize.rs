//! Track-1 Step 4 — whole-doc deterministic structural-quality backstop for
//! the agentic design loop.
//!
//! The orchestrator runs an ordered **Class-A** structural sequence PER SUBTASK
//! (`subagent.rs`) on each parsed forest before insertion, then a per-root
//! cleanup (`cleanup::finalize_design`). The agentic tool-loop
//! (`op-host-services::chat_agent_loop`) runs the model + canvas tools but
//! applies NONE of those deterministic passes — it relies on the model
//! self-correcting. These passes are the invisible-in-a-screenshot
//! structural-correctness work (semantic roles, surface-color discipline,
//! design-token binding) a vision model cannot reconstruct.
//!
//! [`apply_loop_finalize`] gives the loop the SAME backstop, run ONCE at loop
//! end over the WHOLE assembled document instead of per subtask. It reuses the
//! exact pass functions the orchestrator calls (it does NOT reimplement them),
//! in the SAME order, then routes the per-root cleanup through
//! [`crate::cleanup::finalize_design`].
//!
//! ## Included vs excluded passes
//!
//! The Class-A sequence in `subagent.rs` is:
//!
//! 1. `coalesce_subtask_section`            — SUBTASK-BOUNDARY dependent → EXCLUDED
//! 2. `role_infer::resolve_forest_roles`    — boundary-independent → INCLUDED
//! 3. `role_post_pass::post_pass_forest`    — boundary-independent → INCLUDED
//! 4. `promote::promote_forest`             — boundary-independent → INCLUDED
//! 5. `tree_heuristics::apply_tree_heuristics` — boundary-independent → INCLUDED
//! 6. `variable_binding::bind_generated_color_variables` — boundary-independent → INCLUDED
//! 7. `role_post_pass::enforce_surface_color_discipline` — boundary-independent → INCLUDED
//! 8. `normalize_section_roots_for_parent_layout` — SUBTASK-BOUNDARY dependent → EXCLUDED
//! 9. `orchestration_self_check`            — REJECTION gate, not a fix → EXCLUDED
//! 10. per-root `cleanup::finalize_design` — INCLUDED (whole-doc roots).
//!
//! ### Why `coalesce_subtask_section` + `normalize_section_roots_for_parent_layout`
//! are excluded
//!
//! Both require *one-section-per-forest* context. `coalesce_subtask_section`
//! reparents weak-model sibling fragments back into the single wrapper a
//! subtask is *known* to be; `normalize_section_roots_for_parent_layout`
//! normalizes a subtask's roots for the parent frame's layout axis. At loop
//! end the document is a finished multi-section tree — each top-level child is
//! an independent, already-placed section, not the split fragments of one
//! subtask — so re-coalescing or re-normalizing would corrupt the assembled
//! layout instead of repairing it. The whole-doc cleanup passes
//! (`cleanup::finalize_design`) already cover the cross-section structural
//! repairs that are safe at this boundary. See the matching SCOPE NOTE in
//! `cleanup.rs::finalize_design`.
//!
//! `orchestration_self_check` is a *rejection* gate (it fails a subtask whose
//! parse is structurally fatal so the orchestrator can retry); it does not fix
//! anything, and there is no per-subtask retry to drive at loop end, so it is
//! not part of the finalize.

use crate::role_defaults::{detect_theme_from_fill, Theme};
use jian_ops_schema::node::PenNode;
use op_editor_core::{first_solid_fill_hex, EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};

/// Default canvas width when the document has no measurable top-level frame.
/// Mirrors the desktop/web default design width.
const DEFAULT_CANVAS_WIDTH: f64 = 1200.0;

/// Minimal [`DocSink`](crate::types::DocSink) over a borrowed `&mut EditorState`
/// so the per-root cleanup passes (which speak `DocSink` + `EditorCommand`) can
/// run against the loop's live document. Modeled on the test `VecDocSink` and
/// the production `RemoteDocSink`, but synchronous + non-recording: every
/// `apply` goes straight through `EditorState::apply`.
struct StateDocSink<'a> {
    state: &'a mut EditorState,
}

impl crate::types::DocSink for StateDocSink<'_> {
    fn state(&self) -> &EditorState {
        self.state
    }
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.state.apply(cmd)
    }
    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        self.state
            .insert_subtree_returning_root_ids(nodes, parent_id)
    }
    fn begin_undo_batch(&mut self) {}
    fn end_undo_batch(&mut self) {}
}

/// Whether `node` is a single page/screen-root wrapper: a `frame` (or group)
/// container whose children are the design's sections. This mirrors the
/// orchestrator's scaffold page-root — the frame the subtask sections are
/// inserted UNDER. When the loop's whole-doc forest is exactly one such
/// wrapper, the Class-A section passes must run on its CHILDREN (the sections),
/// not on the wrapper itself, so the page background (the wrapper's own fill)
/// is not mistaken for a redundant section surface.
fn is_page_root_wrapper(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_) | PenNode::Group(_))
        && node.children().is_some_and(|c| !c.is_empty())
}

/// Locate the "section forest" + its page context within the active page.
///
/// - **Single page-root wrapper** (one top-level container child): the section
///   forest is that wrapper's children; `page_bg`/`width`/`theme` come from the
///   wrapper. This is the orchestrator's scaffold-page-root contract — sections
///   sit UNDER the page frame, on the page background.
/// - **Flat section forest** (0 or 2+ top-level nodes): the top-level nodes ARE
///   the sections; there is no page-bg frame, so `page_bg = None`,
///   `width = first node width`, `theme` from the first node's fill.
fn locate_section_context(forest: &[PenNode]) -> (bool, Option<String>, f64, Theme) {
    if forest.len() == 1 && is_page_root_wrapper(&forest[0]) {
        let root = &forest[0];
        let width = root
            .width_px()
            .filter(|w| *w > 0.0)
            .unwrap_or(DEFAULT_CANVAS_WIDTH);
        let page_bg = first_solid_fill_hex(root).map(str::to_string);
        let theme = detect_theme_from_fill(page_bg.as_deref());
        (true, page_bg, width, theme)
    } else {
        let first = forest.first();
        let width = first
            .and_then(PenNodeExt::width_px)
            .filter(|w| *w > 0.0)
            .unwrap_or(DEFAULT_CANVAS_WIDTH);
        // A flat-section forest has no page-bg frame: each top-level node is a
        // section, so there is no "redundant page background" to match against.
        // Theme falls back to Light (matching `detect_theme_from_fill(None)`).
        (false, None, width, Theme::Light)
    }
}

/// Perceived luminance (0.299/0.587/0.114) of an `#RRGGBB` hex. `None` on a
/// non-hex string. Local reimplementation to avoid a cross-module `pub`.
fn hex_luminance(hex: &str) -> Option<f64> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if !h.is_ascii() || h.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f64 / 255.0;
    Some(0.299 * r + 0.587 * g + 0.114 * b)
}

/// First solid-fill color STRING from a node's `fill` — the raw authored value
/// (a `$ref` or `#hex`), whether the fill is a bare string or the canonical
/// `[{type:"solid",color}]` array. Resolution to hex happens at the call site.
fn first_solid_color_str(fill: &Value) -> Option<String> {
    match fill {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Array(a) => a.first().and_then(|f| {
            f.get("color")
                .and_then(Value::as_str)
                .or_else(|| f.as_str())
                .map(str::to_string)
        }),
        _ => None,
    }
}

/// A text node has a visible fill when `fill` is a non-empty array or a
/// non-blank string. glm-5.2 in the loop omits `fill` on text entirely.
fn text_has_fill(v: &Value) -> bool {
    match v.get("fill") {
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::String(s)) => !s.trim().is_empty(),
        _ => false,
    }
}

/// Give every fill-less text node a fill that contrasts with its nearest
/// resolved background. A model (glm-5.2 in the agentic loop) defines a light
/// `primary_text` variable but omits `fill` on the text nodes themselves, so
/// they render with the default (dark) color — invisible on the dark surfaces
/// it also built (measured: a full 176-node dashboard rendered near-black on
/// black). `role_post_pass` only fills BUTTON text (`fix_button_foreground_
/// contrast`); general text (headings, KPI labels, table cells) had no
/// fill-injection pass. This is Pencil's hard rule "text without a fill is
/// invisible — always set one". `$ref` backgrounds resolve through the live
/// variable table (`state.resolve_color_variable_hex`).
fn ensure_text_fill_forest(nodes: &mut [PenNode], state: &EditorState) {
    fn resolve_hex(color: &str, state: &EditorState) -> Option<String> {
        match color.strip_prefix('$') {
            Some(name) => state.resolve_color_variable_hex(name),
            None if color.starts_with('#') => Some(color.to_string()),
            None => None,
        }
    }
    fn walk(v: &mut Value, bg: Option<String>, state: &EditorState) {
        // This node's own solid fill (resolved) becomes the background its
        // descendants sit on; unresolvable / absent fills inherit the ancestor.
        let own = v
            .get("fill")
            .and_then(first_solid_color_str)
            .and_then(|c| resolve_hex(&c, state));
        let child_bg = own.or(bg.clone());
        if v.get("type").and_then(Value::as_str) == Some("text") && !text_has_fill(v) {
            // Default the bg to light (lum 1.0) when unknown, so an unresolved
            // background yields dark text rather than white-on-white.
            let lum = child_bg.as_deref().and_then(hex_luminance).unwrap_or(1.0);
            let fg = if lum < 0.5 { "#F5F5F5" } else { "#111827" };
            if let Some(obj) = v.as_object_mut() {
                obj.insert("fill".into(), json!([{ "type": "solid", "color": fg }]));
            }
        }
        if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
            for c in kids.iter_mut() {
                walk(c, child_bg.clone(), state);
            }
        }
    }
    for node in nodes.iter_mut() {
        let Ok(mut v) = serde_json::to_value(&*node) else {
            continue;
        };
        walk(&mut v, None, state);
        if let Ok(nn) = serde_json::from_value::<PenNode>(v) {
            *node = nn;
        }
    }
}

/// Run the WHOLE-DOC subset of the orchestrator's Class-A structural sequence
/// over the assembled document, in the SAME order the per-subtask path uses,
/// then route the per-root cleanup through [`crate::cleanup::finalize_design`].
///
/// This is the agentic-loop counterpart to the per-subtask passes in
/// `subagent.rs`. It is invoked ONCE at loop end (see
/// `op-host-services::chat_agent_loop`). The orchestrator path does NOT call
/// this — it already runs the same passes per subtask.
///
/// The Class-A section passes run on the **section forest** — the page-root
/// wrapper's children when the doc is a single page frame, else the top-level
/// nodes themselves (see [`locate_section_context`]). This matches the
/// orchestrator, where the scaffold page-root is created separately and the
/// section passes only ever see the subtask sections, never the page frame.
/// Feeding the page frame itself through `apply_tree_heuristics` would let its
/// (legitimate) background fill be stripped as a "redundant section surface".
///
/// See the module doc for the included-vs-excluded pass list and the rationale
/// for excluding the two subtask-boundary-dependent passes.
pub fn apply_loop_finalize(state: &mut EditorState) {
    if state.active_children().is_empty() {
        return;
    }

    // -- Resolve the section forest + its page context. --
    let (has_wrapper, page_bg, canvas_width, theme) =
        locate_section_context(state.active_children());
    let light_theme = theme == Theme::Light;

    {
        // `bind_generated_color_variables` reads the live state while the forest
        // is mutably borrowed, so snapshot the state for it first.
        let snapshot = state.clone();
        // The section forest: the page-root wrapper's children, or the top-level
        // nodes themselves. The page-root frame is intentionally NOT fed through
        // these section-level passes (its background is not a section surface).
        let forest: &mut [PenNode] = if has_wrapper {
            state.active_children_mut()[0]
                .children_mut()
                .map(Vec::as_mut_slice)
                .unwrap_or_default()
        } else {
            state.active_children_mut()
        };
        // Dominant brand accent over the section forest (drives the
        // invisible-band fill in `apply_tree_heuristics`).
        let prior_accent = crate::tree_heuristics::dominant_design_accent(forest);

        crate::role_infer::resolve_forest_roles(forest, canvas_width, theme);
        crate::role_post_pass::post_pass_forest(forest, canvas_width);
        jian_ops_schema::promote::promote_forest(forest);
        crate::tree_heuristics::apply_tree_heuristics(
            forest,
            page_bg.as_deref(),
            light_theme,
            prior_accent.as_deref(),
        );
        crate::variable_binding::bind_generated_color_variables(forest, &snapshot);
        crate::role_post_pass::enforce_surface_color_discipline(forest);
    }

    // The app-shell restructure (flat-vertical sidebar dashboard → horizontal
    // [sidebar | content]) runs inside `cleanup::finalize_design` below, the
    // whole-doc finalize point SHARED with the orchestrator path — so both the
    // agentic loop and the orchestrator get it exactly once, after roles are
    // resolved. See `cleanup::run_cleanup_passes`.

    // -- Per-root cleanup (`cleanup::finalize_design`) over every top-level
    //    root, via a borrowed-state DocSink. The cleanup passes are whole-root
    //    (they take the page-root id and recurse), so they always run over the
    //    real top-level nodes — NOT the unwrapped section forest. A synthesized
    //    minimal plan carries the root frame's name (the only plan field the
    //    cleanup passes read — dashboard keyword matching) + its width/fill. --
    let root_ids: Vec<String> = state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    let plan = synthesize_plan(state.active_children(), canvas_width);
    {
        let mut sink = StateDocSink { state: &mut *state };
        let root_id_refs: Vec<&str> = root_ids.iter().map(String::as_str).collect();
        crate::cleanup::finalize_design(&mut sink, &plan, &root_id_refs);
    }

    // Fill-less text → a background-contrasting fill, over the FULLY
    // restructured tree — AFTER the app-shell reshape has moved the sidebar into
    // place — so every text node, including a restructured sidebar's nav labels,
    // is reached (running it earlier over the section forest missed the sidebar
    // text: measured glm-5.2 left all 10 sidebar labels fill-less → invisible).
    // `role_post_pass` only fills button text; a loop model leaves general text
    // fill-less. A fresh snapshot backs the immutable `$variable` resolution
    // while the tree is mutated.
    let snapshot = state.clone();
    ensure_text_fill_forest(state.active_children_mut(), &snapshot);
}

/// Build the minimal [`OrchestratorPlan`](crate::plan::OrchestratorPlan) the
/// cleanup passes need. Only `root_frame.name` (dashboard keyword matching) +
/// `width`/`fill` are read by `finalize_design`; `subtasks` is left empty.
fn synthesize_plan(forest: &[PenNode], canvas_width: f64) -> crate::plan::OrchestratorPlan {
    let root = forest.first();
    let name = root
        .and_then(|n| n.base().name.clone())
        .unwrap_or_else(|| "Page".to_string());
    let fill = root.and_then(first_solid_fill_hex).map(|hex| {
        vec![crate::plan::PlanFill {
            kind: "solid".to_string(),
            color: hex.to_string(),
        }]
    });
    crate::plan::OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: root.map(|n| n.id_str().to_string()).unwrap_or_default(),
            name,
            width: canvas_width,
            height: 0.0,
            layout: None,
            gap: None,
            padding: None,
            fill,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
    }
}

#[cfg(test)]
#[path = "loop_finalize_tests.rs"]
mod tests;
