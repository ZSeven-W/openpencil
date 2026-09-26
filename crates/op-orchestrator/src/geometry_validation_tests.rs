//! Tests for the geometry-driven table-overflow fix. The detection + scale math
//! is exercised against a KNOWN resolved row width (no real layout pass needed);
//! end-to-end behaviour is verified by rendering a generated design.

use super::*;
use serde_json::json;

fn cell(id: &str, w: serde_json::Value) -> serde_json::Value {
    json!({ "type": "frame", "id": id, "name": id, "width": w, "children": [] })
}

fn row(id: &str, widths: &[serde_json::Value]) -> serde_json::Value {
    let cells: Vec<serde_json::Value> = widths
        .iter()
        .enumerate()
        .map(|(i, w)| cell(&format!("{id}-c{i}"), w.clone()))
        .collect();
    json!({ "type": "frame", "id": id, "name": "Row", "layout": "horizontal", "gap": 16, "children": cells })
}

/// Table with 5 fixed columns of 240 (sum 1200) + 4 gaps × 16 = 64 → 1264 needed
/// in a resolved row width of 800: must scale down.
fn overflowing_table() -> serde_json::Value {
    let w = || json!(240);
    let widths = [w(), w(), w(), w(), w()];
    json!({
        "type": "frame", "id": "tbl", "name": "Client Table", "layout": "vertical", "children": [
            row("hdr", &widths), row("r1", &widths), row("r2", &widths)
        ]
    })
}

// ── collapse detector ──

fn kw_op(cmds: &[EditorCommand], prop: &str, val: &str) -> bool {
    cmds.iter().any(|c| {
        matches!(
            c,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == prop && k == val
        )
    })
}

fn resolved_heights_by_name(
    state: &op_editor_core::EditorState,
    rects: &HashMap<String, Rect>,
    names: &[&str],
) -> Vec<f64> {
    let v = serde_json::to_value(state.active_children()[0].clone()).unwrap();
    names
        .iter()
        .map(|name| {
            let id = find_id_by_name(&v, name).unwrap_or_else(|| panic!("{name} exists"));
            rects
                .get(&id)
                .unwrap_or_else(|| panic!("{name} resolved"))
                .h
        })
        .collect()
}

fn find_id_by_name(v: &serde_json::Value, name: &str) -> Option<String> {
    if v.get("name").and_then(|x| x.as_str()) == Some(name) {
        return v.get("id").and_then(|x| x.as_str()).map(String::from);
    }
    v.get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
        .find_map(|c| find_id_by_name(c, name))
}

fn stroked_card_json(id: &str, height: serde_json::Value) -> serde_json::Value {
    json!({
        "type":"frame","id":id,"name":id,"height":height,
        "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#E5E7EB"}]},
        "children":[]
    })
}

fn transparent_card_json(id: &str) -> serde_json::Value {
    json!({"type":"frame","id":id,"name":id,"height":"fit_content","children":[]})
}

fn card_row_rects(heights: [f64; 3]) -> HashMap<String, Rect> {
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "row".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 360.0,
            h: heights.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        },
    );
    for (id, h) in ["c1", "c2", "c3"].into_iter().zip(heights) {
        rects.insert(
            id.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h,
            },
        );
    }
    rects
}

fn geometry_validate_and_fix_without_card_rows(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let mut rounds = 0;
    for _ in 0..MAX_ROUNDS {
        let rects = resolved_rects(sink.state());
        let cmds = {
            let Some(root) = op_editor_core::walkers::find_node(
                sink.state().active_children(),
                &NodeId::new(root_id.to_string()),
            ) else {
                break;
            };
            let Ok(v) = serde_json::to_value(root) else {
                break;
            };
            let mut cmds = Vec::new();
            collect_scale_ops(&v, &rects, &mut cmds);
            collect_collapse_fixes(&v, &rects, &mut cmds);
            collect_text_overflow_fixes(&v, &rects, &mut cmds);
            collect_frame_overflow_fixes(&v, &rects, &mut cmds);
            collect_row_gap_fixes(&v, &rects, &mut cmds);
            collect_row_overfull_fixes(&v, &rects, &mut cmds, false);
            cmds
        };
        if cmds.is_empty() {
            break;
        }
        for cmd in cmds {
            sink.apply(cmd);
        }
        rounds += 1;
    }
    rounds
}

fn is_numbered_corpus_op(name: &str) -> bool {
    name.len() == "p01.op".len()
        && name.starts_with('p')
        && name.ends_with(".op")
        && name[1..3].chars().all(|c| c.is_ascii_digit())
}

// Cluster test modules — this file keeps the shared fixtures/helpers; each
// child mounts with `use super::*` so it sees both them and the module under
// test.
#[path = "geometry_clipped_label_tests.rs"]
mod clipped_label_tests;
#[path = "geometry_clipped_rows_tests.rs"]
mod clipped_rows_tests;
#[path = "geometry_collapse_card_tests.rs"]
mod collapse_card_tests;
#[path = "geometry_design_form_tests.rs"]
mod design_form_tests;
#[path = "geometry_echo_spill_tests.rs"]
mod echo_spill_tests;
#[path = "geometry_jam_gap_tests.rs"]
mod jam_gap_tests;
#[path = "geometry_layout_fix_tests.rs"]
mod layout_fix_tests;
#[path = "geometry_table_scale_tests.rs"]
mod table_scale_tests;
