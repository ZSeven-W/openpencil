//! Minimal scaffold seed for the headless agentic loop
//! (`OPENPENCIL_SMOKE_LOOP=1` + `OPENPENCIL_SMOKE_LOOP_SEED=1`).
//!
//! This is the SIMPLIFIED generation path that copies Pencil's single
//! agentic loop while keeping a tiny weak-model safety net. Instead of the
//! COMPLEX `Orchestrator` (planning LLM with mode-rotation, 3 scaffold
//! strategies, per-subtask sub-agent fan-out, 3-attempt retry ladder), the
//! seed path is exactly:
//!
//! ```text
//! MINIMAL SCAFFOLD SEED  →  ONE agentic tool-loop fills it  →  finalize backstop
//! ```
//!
//! ## What is reused vs. new
//!
//! - **The lightweight planner is reused** — [`build_seed_subtree`] calls
//!   `op_orchestrator::plan::build_fallback_plan`, the orchestrator's
//!   heuristic, NO-LLM fallback planner. It already derives the root-frame
//!   spec (width / height / layout / gap / fill, mobile-aware including
//!   explicit `390x844`-style sizes) AND a small set of named sections
//!   (1–3 for landing, top-summary + main for mobile) straight from the
//!   prompt. We call it with `concurrency: 1` so none of the concurrent /
//!   dashboard branches can engage — this is the planner's simplest mode.
//! - **The scaffold *construction idiom* is reused** — like the
//!   orchestrator's `scaffold::build_scaffold` we build the root frame as a
//!   `serde_json::Value` and `serde_json::from_value` it into a canonical
//!   `PenNode`, then apply ONE `EditorCommand::InsertSubtree`. The single
//!   genuinely new piece is injecting the EMPTY named SECTION child frames:
//!   the orchestrator deliberately leaves the root childless because its
//!   per-subtask fan-out fills the sections later — the seed path drops the
//!   fan-out, so it seeds the section stubs itself. They are the structure
//!   weak models collapse without.
//! - **No new post-processing** — `finalize_on_exit` stays true on the
//!   loop, so `op_orchestrator::apply_loop_finalize` (the Class-A
//!   structural backstop) runs at the end exactly as in pure-loop mode.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use op_orchestrator::plan::{build_fallback_plan, OrchestratorPlan};
use op_orchestrator::types::DesignRequest;

/// Build the minimal seed as ONE `EditorCommand::InsertSubtree`: a
/// page-root frame carrying a small number of EMPTY named section-stub
/// child frames, derived from the orchestrator's heuristic fallback plan.
///
/// Returns `Err` only when the root-frame JSON template fails to
/// deserialize (an implementation bug, never a prompt problem) — the
/// caller treats that as "skip the seed, run the pure loop".
pub fn build_seed_command(prompt: &str) -> Result<EditorCommand, String> {
    let req = DesignRequest {
        prompt: prompt.to_string(),
        model: None,
        provider: None,
        design_md: None,
        // concurrency 1 ⇒ the planner's simplest single-screen shape; the
        // seed path never engages the concurrent / dashboard branches.
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
    };
    // Reuse the orchestrator's NO-LLM heuristic planner for the root-frame
    // spec + the section names/count.
    let plan = build_fallback_plan(&req);
    let node = build_seed_subtree(&plan)?;
    Ok(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    })
}

/// Section stubs hug their content vertically as the loop fills them — no
/// explicit height — and span the page-root width.
fn section_stub_json(plan: &OrchestratorPlan, index: usize) -> serde_json::Value {
    let st = &plan.subtasks[index];
    let id = if st.id.is_empty() {
        format!("section-{}", index + 1)
    } else {
        st.id.clone()
    };
    let name = if st.label.is_empty() {
        format!("Section {}", index + 1)
    } else {
        st.label.clone()
    };
    serde_json::json!({
        "type": "frame",
        "id": format!("seed-{id}"),
        "name": name,
        "width": "fill_container",
        // No "height" key ⇒ fit_content: the section grows as the loop
        // populates it instead of being frozen to a guessed pixel box.
        "layout": "vertical",
        "gap": 16,
        "fill": [],
        "children": [],
    })
}

/// Build the page-root frame `PenNode` with one empty named section child
/// per plan subtask. Mirrors `scaffold::build_root_frame_node`'s
/// build-JSON-then-deserialize idiom so the canonical parse path (not a
/// hand-written struct literal) validates the shape.
fn build_seed_subtree(plan: &OrchestratorPlan) -> Result<PenNode, String> {
    let rf = &plan.root_frame;
    let layout = rf.layout.as_deref().unwrap_or("vertical");
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());
    let gap = rf.gap.filter(|g| *g > 0.0).unwrap_or(20.0);

    let sections: Vec<serde_json::Value> = (0..plan.subtasks.len())
        .map(|i| section_stub_json(plan, i))
        .collect();

    let root = serde_json::json!({
        "type": "frame",
        "id": format!("seed-{}", rf.id),
        "name": rf.name,
        "x": 80,
        "y": 40,
        "width": rf.width,
        "height": rf.height,
        "layout": layout,
        "gap": gap,
        "fill": [{ "type": "solid", "color": fill_hex }],
        "children": sections,
    });

    serde_json::from_value(root).map_err(|e| format!("seed root frame: {e}"))
}

/// Augment the design-agent system prompt so the model knows a scaffold
/// already exists and its job is to FILL the seeded sections (not to start
/// from an empty canvas). Appended to the base prompt for seed mode only.
pub fn seed_system_prompt_suffix(prompt: &str) -> String {
    let req = DesignRequest {
        prompt: prompt.to_string(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
    };
    let plan = build_fallback_plan(&req);
    let section_list = plan
        .subtasks
        .iter()
        .enumerate()
        .map(|(i, st)| {
            let label = if st.label.is_empty() {
                format!("Section {}", i + 1)
            } else {
                st.label.clone()
            };
            format!("  - \"{label}\"")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "\n\n## Scaffold already created\n\
         A page-root frame with these EMPTY named sections has ALREADY been \
         inserted on the canvas:\n{section_list}\n\
         Do NOT recreate the page-root or duplicate the design. Read the canvas \
         (get_editor_state / get_screenshot), then FILL each named section in \
         place with its content using batch_design — adjust or add sections as \
         the prompt needs, but build on the existing scaffold rather than \
         starting over.\n"
    )
}

#[cfg(test)]
#[path = "loop_seed_tests.rs"]
mod tests;
