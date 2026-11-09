//! Coverage for the itemized repair record — specifically the part that is
//! easy to get wrong and impossible to notice: the `before` half of
//! `before → after`. It only exists in the document for the instant before
//! the command is applied, so every assertion here builds a real node, reads
//! the description against the pre-edit state, and checks the OLD value is
//! in the line. A record that only reported the new value would look right
//! in the UI and be useless for the question it exists to answer.

use super::*;
use crate::test_support::VecDocSink;
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};
use serde_json::json;

fn sink_with(tree: serde_json::Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    let node: PenNode = serde_json::from_value(tree).expect("fixture json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink
}

fn showcase_tree() -> serde_json::Value {
    json!({
        "type": "frame",
        "id": "root",
        "name": "Landing Page",
        "width": 1200,
        "height": 800,
        "layout": "vertical",
        "gap": 24,
        "children": [{
            "type": "frame",
            "id": "showcase",
            "name": "Showcase Band",
            "layout": "horizontal",
            "gap": 24,
            "padding": [32, 32, 32, 32],
            "justifyContent": "center",
            "fill": [{ "type": "solid", "color": "#F8FAFC" }],
            "children": []
        }]
    })
}

fn node_id_of(sink: &VecDocSink, name: &str) -> NodeId {
    fn walk(nodes: &[PenNode], name: &str) -> Option<String> {
        for node in nodes {
            if node.base().name.as_deref() == Some(name) {
                return Some(node.id_str().to_string());
            }
            if let Some(children) = node.children() {
                if let Some(hit) = walk(children, name) {
                    return Some(hit);
                }
            }
        }
        None
    }
    NodeId::new(walk(sink.state.active_children(), name).expect("fixture node"))
}

#[test]
fn layout_prop_record_carries_the_previous_value() {
    let sink = sink_with(showcase_tree());
    let node_id = node_id_of(&sink, "Showcase Band");

    let described = describe_command(
        &sink.state,
        &EditorCommand::SetNodeLayoutProp {
            node_id: node_id.clone(),
            property: "gap".into(),
            value: LayoutPropValue::Number(16.0),
        },
    );

    assert_eq!(described.node_name.as_deref(), Some("Showcase Band"));
    assert_eq!(
        described.detail, "gap 24 → 16",
        "the record must name the value the pass overwrote, not only the new one"
    );
}

#[test]
fn fill_record_carries_the_previous_colour() {
    // The user-facing question this exists for: "the polish stage changed my
    // pale showcase band — to what, from what?"
    let sink = sink_with(showcase_tree());
    let node_id = node_id_of(&sink, "Showcase Band");

    let described = describe_command(
        &sink.state,
        &EditorCommand::SetNodeFillHex {
            node_id,
            hex: "#FFFFFF".into(),
        },
    );

    assert_eq!(described.detail, "fill #F8FAFC → #FFFFFF");
}

#[test]
fn patch_record_names_every_changed_key_and_skips_unchanged_ones() {
    let sink = sink_with(showcase_tree());
    let node_id = node_id_of(&sink, "Showcase Band");

    let described = describe_command(
        &sink.state,
        &EditorCommand::PatchNodeData {
            node_id,
            // `justifyContent` is already `center`: an unchanged key must not
            // be reported as a repair detail, or the list accuses a pass of
            // an edit it did not make.
            patch_json: r#"{"justifyContent":"center","alignItems":"start"}"#.into(),
            page_id: None,
        },
    );

    assert_eq!(described.detail, "alignItems (unset) → start");
}

#[test]
fn subtree_replacement_record_diffs_the_nodes_that_changed() {
    let sink = sink_with(showcase_tree());
    let root_id = node_id_of(&sink, "Landing Page");
    let before = op_editor_core::walkers::find_node(sink.state.active_children(), &root_id)
        .expect("root")
        .clone();

    // What every `apply_root_transform` pass does: hand back a rebuilt root.
    let mut after_json = serde_json::to_value(&before).expect("serialize");
    after_json["children"][0]["padding"] = json!([16, 16, 16, 16]);
    after_json["children"][0]["justifyContent"] = json!("start");
    let after: PenNode = serde_json::from_value(after_json).expect("rebuilt root");

    let described = describe_command(
        &sink.state,
        &EditorCommand::ReplaceSubtree {
            node_id: root_id,
            node: Box::new(after),
            drop_children: true,
            page_id: None,
        },
    );

    assert!(
        described.detail.contains("Showcase Band"),
        "the diff must name the node that changed: {}",
        described.detail
    );
    assert!(
        described
            .detail
            .contains("padding [32,32,32,32] → [16,16,16,16]"),
        "the diff must carry before → after for the changed field: {}",
        described.detail
    );
    assert!(
        described.detail.contains("justifyContent center → start"),
        "alignment changes are exactly what a user disputes: {}",
        described.detail
    );
}

#[test]
fn deletion_record_names_what_was_removed_before_it_is_gone() {
    let sink = sink_with(showcase_tree());
    let node_id = node_id_of(&sink, "Showcase Band");

    let described = describe_command(
        &sink.state,
        &EditorCommand::DeleteNode {
            node_id,
            page_id: None,
        },
    );

    assert_eq!(described.node_name.as_deref(), Some("Showcase Band"));
    assert!(
        described.detail.starts_with("removed"),
        "unexpected detail: {}",
        described.detail
    );
}

#[test]
fn unknown_commands_still_produce_a_record_rather_than_a_gap() {
    // The count is `records.len()`, so a command with no bespoke description
    // must still yield one — a silent skip would make the credential's number
    // disagree with its own list.
    let sink = sink_with(showcase_tree());

    let described = describe_command(&sink.state, &EditorCommand::PromoteLegacyWidgets);

    assert!(!described.detail.is_empty());
}

#[test]
fn record_line_reads_as_pass_node_and_change() {
    let record = RepairRecord {
        pass: "table-gap".into(),
        category: CheckCategory::Layout,
        node_id: "n42".into(),
        node_name: Some("Pricing Row".into()),
        detail: "gap 0 → 16".into(),
    };

    assert_eq!(
        record.line(),
        "layout · table-gap · Pricing Row [n42] · gap 0 → 16"
    );
}

#[test]
fn record_line_degrades_without_a_node_name() {
    let record = RepairRecord {
        pass: "theme-variable-polarity".into(),
        category: CheckCategory::Palette,
        node_id: String::new(),
        node_name: None,
        detail: "variable surface → #FFFFFF".into(),
    };

    assert_eq!(
        record.line(),
        "palette · theme-variable-polarity · variable surface → #FFFFFF"
    );
}

#[test]
fn compound_values_read_as_what_they_are_not_as_json() {
    let animations = serde_json::json!([{
        "trigger": "inView", "durationMs": 320,
        "keyframes": [{"offset": 0, "values": {"opacity": 0}}, {"offset": 1, "values": {"opacity": 1}}]
    }]);
    assert_eq!(super::render_value(&animations), "[inView animation 320ms]");
    let fills = serde_json::json!([
        {"type": "solid", "color": "#F8FAFC"},
        {"type": "linear_gradient", "stops": []}
    ]);
    assert_eq!(super::render_value(&fills), "[#F8FAFC, linear_gradient]");
    assert_eq!(
        super::render_value(&serde_json::json!({"a": 1, "b": 2})),
        "{2 fields}"
    );
    // Number lists keep their compact form.
    assert_eq!(
        super::render_value(&serde_json::json!([8, 16, 8, 16])),
        "[8,16,8,16]"
    );
}
