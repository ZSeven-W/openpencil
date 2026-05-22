//! §4.6 Slot assignment — `assign_dashboard_main_parents`.
//!
//! Split from `dashboard_columns.rs` to keep it under the 800-line ceiling.
//! Faithful port of TS `assignDashboardMainParents` (`orchestrator.ts:401-484`).

use super::group_dashboard_main_rows;
use crate::plan::OrchestratorPlan;

/// Return item from [`assign_dashboard_main_parents`]:
/// synthesized row/slot frame plus the id of its direct parent.
pub(crate) type RowFrameEntry = (jian_ops_schema::node::PenNode, String);

/// Synthesizes row and slot frames for the dashboard main column, and assigns
/// each non-sidebar subtask's `parent_frame_id` to its slot.
///
/// Port of TS `assignDashboardMainParents` (`orchestrator.ts:401-484`).
///
/// # Arguments
/// * `plan` — mutable plan (subtask `parent_frame_id` fields are written).
/// * `main_parent_id` — the id of the main column frame (parent for row frames).
///
/// # Returns
/// `Ok(Vec<(PenNode, parent_id)>)` — synthesized row and slot frames in
/// insertion order (row frame first, then its slot children left-to-right).
/// Returns `Err` only on internal JSON template bugs.
///
/// ## Layout rules (faithful TS port)
/// * **1-subtask row** → one vertical `fill_container`-width Slot frame,
///   `id = {main_parent_id}-row-{n}-{subtask_id}-slot`, parented to `main_parent_id`.
/// * **N-subtask row** → one horizontal Dashboard Row frame
///   (`id = {main_parent_id}-row-{n}`, `gap = row_gap`, `fill_container` width),
///   parented to `main_parent_id`, followed by N vertical Slot frames each
///   parented to the row id.
///   - Slot widths are proportional: `round(available_width * st.width / Σ widths)`.
///   - `available_width = max(320, full_width - slot_gap)` where
///     `slot_gap = row_gap * (row.len - 1)`.
///   - Each slot width is floored at **220**.
///   - The **last slot** always has `fill_container` width (absorbs rounding).
///   - The capping formula guards against over-allocation:
///     `min(available - assigned - 220*(remaining_after), proportional)`.
pub(crate) fn assign_dashboard_main_parents(
    plan: &mut OrchestratorPlan,
    main_parent_id: &str,
) -> Result<Vec<RowFrameEntry>, String> {
    use jian_ops_schema::node::PenNode;

    let row_groups = group_dashboard_main_rows(plan);
    let rows = &row_groups.rows;
    let full_width = row_groups.full_width;
    let row_gap = row_groups.row_gap;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    // Clone rows before mutating subtasks' parent_frame_id.
    let rows_snapshot: Vec<Vec<usize>> = rows.clone();

    let mut result: Vec<RowFrameEntry> = Vec::new();

    for (row_idx, row_indices) in rows_snapshot.iter().enumerate() {
        let row_n = row_idx + 1; // 1-based, matches TS `rowIndex`

        if row_indices.len() == 1 {
            let idx = row_indices[0];
            let (subtask_id, subtask_label) = {
                let st = &plan.subtasks[idx];
                (st.id.clone(), st.label.clone())
            };
            let slot_id = format!("{main_parent_id}-row-{row_n}-{subtask_id}-slot");
            let slot_name = format!("{subtask_label} Slot");

            let slot_node = make_slot_frame(&slot_id, &slot_name, SlotWidth::FillContainer)?;
            result.push((PenNode::Frame(slot_node), main_parent_id.to_string()));

            plan.subtasks[idx].parent_frame_id = Some(slot_id);
        } else {
            let row_id = format!("{main_parent_id}-row-{row_n}");
            let slot_gap = row_gap * (row_indices.len() as f64 - 1.0);
            let available_width = f64::max(320.0, full_width - slot_gap);

            // Sum of subtask widths (with floor 1.0 per TS `Math.max(1, ...)`)
            let width_sum: f64 = row_indices
                .iter()
                .map(|&i| f64::max(1.0, plan.subtasks[i].region.width))
                .sum();

            // Push the Dashboard Row frame first.
            let row_frame = make_row_frame(&row_id, row_gap)?;
            result.push((PenNode::Frame(row_frame), main_parent_id.to_string()));

            let mut assigned_width: f64 = 0.0;
            let row_len = row_indices.len();
            for (local_idx, &st_idx) in row_indices.iter().enumerate() {
                let is_last = local_idx == row_len - 1;
                let (subtask_id, subtask_label, st_width) = {
                    let st = &plan.subtasks[st_idx];
                    let w = f64::max(1.0, st.region.width);
                    (st.id.clone(), st.label.clone(), w)
                };

                let proportional_width =
                    f64::max(220.0, (available_width * st_width / width_sum).round());
                // Remaining after this slot (0-based from right)
                let remaining_after = row_len - local_idx - 1;
                let slot_width: SlotWidth = if is_last {
                    SlotWidth::FillContainer
                } else {
                    let capped = f64::min(
                        available_width - assigned_width - 220.0 * remaining_after as f64,
                        proportional_width,
                    );
                    SlotWidth::Px(capped)
                };

                if let SlotWidth::Px(w) = slot_width {
                    assigned_width += w;
                }

                let slot_id = format!("{row_id}-{subtask_id}-slot");
                let slot_name = format!("{subtask_label} Slot");
                let slot_node = make_slot_frame(&slot_id, &slot_name, slot_width)?;
                result.push((PenNode::Frame(slot_node), row_id.clone()));

                plan.subtasks[st_idx].parent_frame_id = Some(slot_id);
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Width variant for a slot frame: a fixed pixel value or `fill_container`.
enum SlotWidth {
    Px(f64),
    FillContainer,
}

/// Build a vertical slot frame node (deserialized via `PenNode` enum tag).
fn make_slot_frame(
    id: &str,
    name: &str,
    width: SlotWidth,
) -> Result<jian_ops_schema::node::FrameNode, String> {
    let width_json = match width {
        SlotWidth::Px(w) => serde_json::json!(w),
        SlotWidth::FillContainer => serde_json::json!("fill_container"),
    };
    let frame = serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "width": width_json,
        "height": "fit_content",
        "layout": "vertical",
        "children": [],
    });
    match serde_json::from_value::<jian_ops_schema::node::PenNode>(frame)
        .map_err(|e| format!("slot frame `{id}`: {e}"))?
    {
        jian_ops_schema::node::PenNode::Frame(f) => Ok(f),
        other => Err(format!("slot frame `{id}`: unexpected variant {other:?}")),
    }
}

/// Build a horizontal Dashboard Row frame node (deserialized via `PenNode` enum tag).
fn make_row_frame(id: &str, gap: f64) -> Result<jian_ops_schema::node::FrameNode, String> {
    let frame = serde_json::json!({
        "type": "frame",
        "id": id,
        "name": format!("Dashboard Row {}", id.split('-').next_back().unwrap_or("?")),
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "gap": gap,
        "children": [],
    });
    match serde_json::from_value::<jian_ops_schema::node::PenNode>(frame)
        .map_err(|e| format!("row frame `{id}`: {e}"))?
    {
        jian_ops_schema::node::PenNode::Frame(f) => Ok(f),
        other => Err(format!("row frame `{id}`: unexpected variant {other:?}")),
    }
}
