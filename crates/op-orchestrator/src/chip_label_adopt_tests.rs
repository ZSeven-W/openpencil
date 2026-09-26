//! Tests for [`super`] — labels authored beside their empty chips.
//!
//! Every fit assertion keeps a wide margin (a 2-glyph 12px label in a 44px
//! chip; a 30-character 14px label against the same chip) so the outcome does
//! not depend on which face the running machine resolves — CI has no DM Sans
//! and no CJK face.

use super::adopt_empty_chip_labels;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

fn chip(name: &str, selected: bool) -> Value {
    let (fill, stroke) = if selected {
        (
            json!([{"type": "solid", "color": "#FFFFFF"}]),
            json!({"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]}),
        )
    } else {
        (json!([]), json!({"thickness": 0, "fill": []}))
    };
    json!({
        "type": "frame", "id": name, "name": name,
        "width": 44, "height": 28,
        "layout": "horizontal", "justifyContent": "center", "alignItems": "center",
        "fill": fill, "stroke": stroke, "children": []
    })
}

fn label(name: &str, content: &str) -> Value {
    json!({
        "type": "text", "id": name, "name": name, "content": content,
        "width": "fit_content", "height": "fit_content",
        "fontFamily": "DM Sans", "fontSize": 12, "lineHeight": 1.5,
        "textGrowth": "auto",
        "fill": [{"type": "solid", "color": "#0F172A"}]
    })
}

fn row(name: &str, children: Vec<Value>) -> Value {
    json!({
        "type": "frame", "id": name, "name": name,
        "width": "fit_content", "height": "fit_content",
        "layout": "horizontal", "gap": 4, "padding": 3, "alignItems": "center",
        "fill": [{"type": "solid", "color": "#F1F5F9"}],
        "children": children
    })
}

/// The `arena-m03` `range-switcher` shape: three empty chips, the middle one
/// painted as selected, each followed by its label.
fn m03_switcher() -> Value {
    row(
        "range-switcher",
        vec![
            chip("range-chip-1周", false),
            label("range-chip-text-1周", "1周"),
            chip("range-chip-1月", true),
            label("range-chip-text-1月", "1月"),
            chip("range-chip-3月", false),
            label("range-chip-text-3月", "3月"),
        ],
    )
}

fn sink_with(children: Vec<Value>) -> (VecDocSink, String) {
    let root = json!({
        "type": "frame", "id": "root", "name": "持仓",
        "width": 375, "height": 812, "layout": "vertical", "children": children
    });
    let root: PenNode = serde_json::from_value(root).expect("root fixture");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    let root_id = sink.state.active_children()[0].id_str().to_string();
    (sink, root_id)
}

fn root_json(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialized root")
}

/// `InsertSubtree` remaps ids, so fixtures are found by name.
fn named(node: &Value, name: &str) -> Value {
    fn find(node: &Value, name: &str) -> Option<Value> {
        if node.get("name").and_then(Value::as_str) == Some(name) {
            return Some(node.clone());
        }
        node.get("children")
            .and_then(Value::as_array)?
            .iter()
            .find_map(|child| find(child, name))
    }
    find(node, name).unwrap_or_else(|| panic!("no node named {name}"))
}

fn child_names(node: &Value) -> Vec<String> {
    node.get("children")
        .and_then(Value::as_array)
        .map(|kids| {
            kids.iter()
                .filter_map(|k| k.get("name").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_m03_switcher_labels_move_into_their_chips_in_order() {
    let (mut sink, root_id) = sink_with(vec![m03_switcher()]);

    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 3);

    let root = root_json(&sink);
    let switcher = named(&root, "range-switcher");
    assert_eq!(
        child_names(&switcher),
        ["range-chip-1周", "range-chip-1月", "range-chip-3月"],
        "the row keeps only the chips, in their authored order"
    );
    for key in ["1周", "1月", "3月"] {
        let chip = named(&switcher, &format!("range-chip-{key}"));
        assert_eq!(
            child_names(&chip),
            [format!("range-chip-text-{key}")],
            "each label is its own chip's only child"
        );
        let text = &chip["children"][0];
        assert!(text.get("x").is_none() && text.get("y").is_none());
    }
    let selected = named(&switcher, "range-chip-1月");
    assert_eq!(
        selected["fill"][0]["color"], "#FFFFFF",
        "the selected chip keeps its highlight"
    );
}

#[test]
fn the_pass_is_idempotent() {
    let (mut sink, root_id) = sink_with(vec![m03_switcher()]);
    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 3);
    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 0);
}

#[test]
fn a_lone_swatch_and_its_name_stay_apart() {
    let legend = row(
        "legend",
        vec![chip("swatch", true), label("swatch-name", "收入")],
    );
    let (mut sink, root_id) = sink_with(vec![legend]);
    let before = root_json(&sink);

    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

/// A label wider than its fixed-width chip is left alone rather than the chip
/// widened: it is not provably that chip's label.
#[test]
fn labels_wider_than_a_fixed_chip_stay_outside() {
    let wide = "Quarterly performance overview";
    let tabs = row(
        "tabs",
        vec![
            chip("tab-a", true),
            label("tab-a-text", wide),
            chip("tab-b", false),
            label("tab-b-text", wide),
        ],
    );
    let (mut sink, root_id) = sink_with(vec![tabs]);
    let before = root_json(&sink);

    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn empty_chips_not_followed_by_text_are_untouched() {
    let icon = |name: &str| {
        json!({
            "type": "icon_font", "id": name, "name": name,
            "iconFontName": "star", "width": 16, "height": 16
        })
    };
    let dots = row(
        "dots",
        vec![
            chip("dot-a", true),
            chip("dot-b", false),
            chip("dot-c", false),
        ],
    );
    let icons = row(
        "icons",
        vec![
            chip("slot-a", false),
            icon("icon-a"),
            chip("slot-b", false),
            icon("icon-b"),
        ],
    );
    let (mut sink, root_id) = sink_with(vec![dots, icons]);
    let before = root_json(&sink);

    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn a_status_bar_row_is_untouched() {
    let mut bar = m03_switcher();
    bar["name"] = json!("status-bar");
    bar["id"] = json!("status-bar");
    let (mut sink, root_id) = sink_with(vec![bar]);
    let before = root_json(&sink);

    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn positioned_labels_are_left_where_they_were_placed() {
    let mut switcher = m03_switcher();
    for i in [1, 3, 5] {
        switcher["children"][i]["x"] = json!(0);
        switcher["children"][i]["y"] = json!(0);
    }
    let (mut sink, root_id) = sink_with(vec![switcher]);
    let before = root_json(&sink);

    assert_eq!(adopt_empty_chip_labels(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn the_driver_mounts_the_pass_under_the_chip_checkpoint() {
    use crate::cleanup::run_cleanup_passes_with_summary;
    use crate::plan::{OrchestratorPlan, RootFrameSpec};
    use crate::repair_summary::{CheckCategory, RepairSummary};

    let (mut sink, root_id) = sink_with(vec![m03_switcher()]);
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: root_id.clone(),
            name: "持仓".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan, &[&root_id], &mut summary);

    let switcher = named(&root_json(&sink), "range-switcher");
    assert_eq!(
        child_names(&named(&switcher, "range-chip-1月")),
        ["range-chip-text-1月"],
        "the mounted pass must actually adopt the labels"
    );
    assert!(
        summary.records().iter().any(|record| {
            record.pass == "chip+ring-extract" && record.category == CheckCategory::Structure
        }),
        "the adoption must be recorded under the chip checkpoint: {:?}",
        summary.records()
    );
}
