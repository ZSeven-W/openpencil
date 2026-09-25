//! Right-hand rail of the pre-built desktop app-shell.
//!
//! The sidebar app-shell scaffold used to have exactly two columns —
//! `[Sidebar | Main Content]` — and routed every non-sidebar subtask into the
//! vertical content column. A plan that names a RIGHT-side region (a task
//! detail drawer, an inspector, a right panel) therefore stacked that region
//! UNDER the main content: arena-w01's "右侧任务详情抽屉" (right-side task
//! detail drawer) landed below the kanban board and the 1440×900 page grew by
//! the drawer's full height. The planner said "right"; the scaffold is the
//! layout contract that honours it, so a desktop shell whose plan carries such
//! a subtask gets a third column, `[Sidebar | Main Content | Right Panel]`, and
//! the rail subtask generates into it.

use crate::dashboard_columns::is_sidebar_subtask;
use crate::design_type::contains_word;
use crate::plan::{OrchestratorPlan, Subtask};

/// Name stamped on the pre-built rail column; the run loop re-resolves the
/// (remapped-on-insert) id by it, like the other two columns.
pub(crate) const RIGHT_RAIL_COLUMN_NAME: &str = "Right Panel";

/// A third column only fits beside a 260px sidebar and a usable main column
/// on a genuinely wide artboard.
const MIN_ROOT_WIDTH: f64 = 1200.0;
const DEFAULT_RAIL_WIDTH: f64 = 400.0;
const MIN_RAIL_WIDTH: f64 = 320.0;
const MAX_RAIL_WIDTH: f64 = 480.0;

/// Multi-word ASCII cues, matched as phrases.
const ASCII_PHRASES: &[&str] = &[
    "right panel",
    "right rail",
    "right sidebar",
    "right drawer",
    "right column",
    "detail panel",
    "details panel",
];
/// Single ASCII words that name a docked side surface on their own.
const ASCII_WORDS: &[&str] = &["drawer", "inspector"];
const CJK_CUES: &[&str] = &["右侧", "右栏", "抽屉", "详情面板", "检查器"];

/// True when the subtask's id/label names a right-hand docked surface.
pub(crate) fn is_right_rail_subtask(st: &Subtask) -> bool {
    let identity = format!("{} {}", st.id, st.label)
        .to_lowercase()
        .replace(['-', '_'], " ");
    ASCII_PHRASES.iter().any(|p| identity.contains(p))
        || ASCII_WORDS.iter().any(|w| contains_word(&identity, w))
        || CJK_CUES.iter().any(|c| identity.contains(c))
}

/// Width of the rail column for a sidebar-dashboard plan, or `None` when the
/// plan keeps the two-column shell: the artboard is too narrow, no subtask is
/// a right rail, or routing the rail out would leave Main Content empty.
pub(crate) fn plan_right_rail_width(plan: &OrchestratorPlan) -> Option<f64> {
    if plan.root_frame.width < MIN_ROOT_WIDTH {
        return None;
    }
    let mut rail = None;
    let mut main_sections = 0usize;
    for (index, st) in plan.subtasks.iter().enumerate() {
        if index > 0 && is_right_rail_subtask(st) {
            rail.get_or_insert(st);
        } else if !is_sidebar_subtask(st) {
            main_sections += 1;
        }
    }
    let rail = rail?;
    if main_sections == 0 {
        return None;
    }
    let width = rail.region.width;
    Some(if (MIN_RAIL_WIDTH..=MAX_RAIL_WIDTH).contains(&width) {
        width
    } else {
        DEFAULT_RAIL_WIDTH
    })
}

/// The empty rail column appended to the shell's row: fixed width, stretched
/// to the row height, clipped like the sidebar, with the content column's
/// vertical gutter so a panel authored without its own inset does not touch
/// the artboard edge.
pub(crate) fn right_rail_column_json(
    root_id: &str,
    width: f64,
    fill_hex: &str,
    gap: f64,
) -> serde_json::Value {
    serde_json::json!({
        "type": "frame",
        "id": format!("{root_id}-right-rail"),
        "name": RIGHT_RAIL_COLUMN_NAME,
        "width": width,
        "height": "fill_container",
        "layout": "vertical",
        "gap": gap,
        "padding": [32, 24],
        "fill": [{ "type": "solid", "color": fill_hex }],
        "clipContent": true,
        "children": [],
    })
}

#[cfg(test)]
#[path = "scaffold_right_rail_tests.rs"]
mod tests;
