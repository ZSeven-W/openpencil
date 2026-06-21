//! `EditorState::apply(EditorCommand)` tests — ported from
//! shell-core's `mcp/replace_node_tests.rs` / `copy_node_tests.rs` /
//! `write_tools_tests.rs` apply branches, retargeted onto the
//! canonical `EditorState` + `EditorCommand`.
//!
//! Every test exercises the pre-validate-then-mutate discipline: a
//! rejected command leaves the document byte-for-byte unchanged.

#![cfg(test)]

use crate::command::{BatchInsertItem, EditorCommand, NodeFlag, VariableScalarPayload};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{flex_frame, flow_rect, rect, sample, state_with, text};
use crate::walkers::find_node;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::{VariableDefinition, VariableKind, VariableScalar, VariableValue};

/// Removing a fill must also drop the node's `fill_refs` variable binding.
/// Otherwise the scene resolver's `fill_for` (a registered fill ref wins
/// over `container.fill`) keeps painting the variable colour after the
/// fill row is gone — the "deleted the fill but the colour stays" bug on
/// token-based (old .op) designs.
#[test]
fn remove_selected_fill_clears_the_variable_ref() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(NodeId::new("n1"));
    // Mirror a `$ref` fill: bind writes `$name` into fill[0] + fill_refs.
    let node = crate::walkers::find_node_mut(s.active_children_mut(), &NodeId::new("n1")).unwrap();
    crate::fills::set_primary_fill_hex(node, "$color-info-bg");
    s.ui
        .variables
        .fill_refs
        .insert(NodeId::new("n1"), "color-info-bg".to_string());

    assert!(s.remove_selected_fill(0), "remove must report success");

    let node = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert!(
        crate::fills::node_fills(node)
            .map(|f| f.is_empty())
            .unwrap_or(true),
        "container.fill must be cleared"
    );
    assert!(
        !s.ui.variables.fill_refs.contains_key(&NodeId::new("n1")),
        "fill_ref must clear too, else fill_for keeps painting the variable colour"
    );
}

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

// --- MoveNode --------------------------------------------------------

#[test]
fn move_node_reparents_into_container() {
    let mut s = sample();
    // Move Title (n11) under the Button group (n12).
    assert!(s.apply(EditorCommand::MoveNode {
        node_id: id("n11"),
        target_parent: id("n12"),
        page_id: None,
        index: None,
    }));
    let group = find_node(s.active_children(), &id("n12")).unwrap();
    assert!(group
        .children()
        .unwrap()
        .iter()
        .any(|c| c.id_str() == "n11"));
}

#[test]
fn move_node_rejects_cycle() {
    let mut s = sample();
    // Frame n10 contains n12; moving n10 under n12 would cycle.
    assert!(!s.apply(EditorCommand::MoveNode {
        node_id: id("n10"),
        target_parent: id("n12"),
        page_id: None,
        index: None,
    }));
    // n10 still at root.
    assert!(s.active_children().iter().any(|c| c.id_str() == "n10"));
}

#[test]
fn move_node_to_page_root_with_none_target() {
    let mut s = sample();
    assert!(s.apply(EditorCommand::MoveNode {
        node_id: id("n11"),
        target_parent: NodeId::NONE,
        page_id: None,
        index: None,
    }));
    // n11 now at the page root, alongside n10.
    assert!(s.active_children().iter().any(|c| c.id_str() == "n11"));
}

// --- CopyNode --------------------------------------------------------

#[test]
fn copy_node_clones_subtree_with_fresh_ids() {
    let mut s = sample();
    let pre_max = s.max_node_id();
    assert!(s.apply(EditorCommand::CopyNode {
        node_id: id("n12"),
        target_parent: NodeId::NONE,
        overrides_json: None,
        page_id: None,
    }));
    // A clone landed at the page root; its id is fresh.
    assert!(s.max_node_id() > pre_max);
    // No duplicate ids in the document.
    assert!(s.find_duplicate_id().is_none());
}

#[test]
fn copy_node_rejects_unknown_source() {
    let mut s = sample();
    assert!(!s.apply(EditorCommand::CopyNode {
        node_id: id("ghost"),
        target_parent: NodeId::NONE,
        overrides_json: None,
        page_id: None,
    }));
}

// --- ReplaceNode -----------------------------------------------------

#[test]
fn replace_node_swaps_at_same_slot() {
    let mut s = sample();
    let pre_max = s.max_node_id();
    assert!(s.apply(EditorCommand::ReplaceNode {
        node_id: id("n11"),
        kind: "rect".into(),
        name: "Swapped".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: Some("#ff0000".into()),
        drop_children: false,
        page_id: None,
    }));
    let frame = find_node(s.active_children(), &id("n10")).unwrap();
    let kids = frame.children().unwrap();
    assert!(kids.iter().all(|n| n.id_str() != "n11"));
    assert_eq!(kids[0].base().name.as_deref(), Some("Swapped"));
    assert!(s.max_node_id() > pre_max);
    // Sibling group still at slot 1.
    assert_eq!(kids[1].id_str(), "n12");
}

#[test]
fn replace_node_refuses_to_drop_container_children() {
    let mut s = sample();
    let pre_max = s.max_node_id();
    // n10 (Frame) has children — refuse without consent.
    assert!(!s.apply(EditorCommand::ReplaceNode {
        node_id: id("n10"),
        kind: "rect".into(),
        name: "WouldNuke".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: None,
        drop_children: false,
        page_id: None,
    }));
    let frame = find_node(s.active_children(), &id("n10")).unwrap();
    assert_eq!(frame.children().unwrap().len(), 2);
    assert_eq!(s.max_node_id(), pre_max); // no id minted
}

#[test]
fn replace_node_drops_container_children_when_opted_in() {
    let mut s = sample();
    assert!(s.apply(EditorCommand::ReplaceNode {
        node_id: id("n10"),
        kind: "rect".into(),
        name: "ExplicitNuke".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: None,
        drop_children: true,
        page_id: None,
    }));
    let root = s.active_children();
    assert!(root.iter().all(|n| n.id_str() != "n10"));
    assert_eq!(root.len(), 1);
    assert_eq!(root[0].base().name.as_deref(), Some("ExplicitNuke"));
}

#[test]
fn replace_node_atomic_on_bad_fill_hex() {
    let mut s = sample();
    let pre_max = s.max_node_id();
    assert!(!s.apply(EditorCommand::ReplaceNode {
        node_id: id("n11"),
        kind: "rect".into(),
        name: "WouldFail".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: Some("not-hex".into()),
        drop_children: false,
        page_id: None,
    }));
    // n11 still present, no id minted.
    let frame = find_node(s.active_children(), &id("n10")).unwrap();
    assert!(frame
        .children()
        .unwrap()
        .iter()
        .any(|c| c.id_str() == "n11"));
    assert_eq!(s.max_node_id(), pre_max);
}

// --- BatchInsert -----------------------------------------------------

#[test]
fn batch_insert_appends_all_items() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::BatchInsert {
        items: vec![
            BatchInsertItem {
                kind: "rect".into(),
                name: "a".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
            },
            BatchInsertItem {
                kind: "ellipse".into(),
                name: "b".into(),
                x: 20,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: Some("#00ff00".into()),
            },
        ],
        page_id: None,
    }));
    assert_eq!(s.active_children().len(), 2);
    assert!(s.find_duplicate_id().is_none());
}

#[test]
fn batch_insert_rejects_whole_batch_on_one_bad_item() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::BatchInsert {
        items: vec![
            BatchInsertItem {
                kind: "rect".into(),
                name: "ok".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
            },
            BatchInsertItem {
                kind: "bogus".into(), // bad kind kills the batch
                name: "bad".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
            },
        ],
        page_id: None,
    }));
    // Nothing inserted — atomic.
    assert!(s.active_children().is_empty());
}

#[test]
fn batch_insert_rejects_empty() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::BatchInsert {
        items: vec![],
        page_id: None,
    }));
}

// --- Per-node attribute writers --------------------------------------

#[test]
fn set_node_fill_and_name() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::SetNodeFillHex {
        node_id: id("n1"),
        hex: "#123456".into(),
    }));
    assert!(s.apply(EditorCommand::SetNodeName {
        node_id: id("n1"),
        name: "Fresh".into(),
    }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().name.as_deref(), Some("Fresh"));
    assert_eq!(crate::fills::first_solid_fill_hex(n), Some("#123456"));
}

#[test]
fn set_node_fill_rejects_bad_hex() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::SetNodeFillHex {
        node_id: id("n1"),
        hex: "zzz".into(),
    }));
}

#[test]
fn set_node_rotation_writes_degrees() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::SetNodeRotation {
        node_id: id("n1"),
        degrees: 45.0,
    }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().rotation, Some(45.0));
}

#[test]
fn set_node_text_rejects_non_text_kind() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::SetNodeText {
        node_id: id("n1"),
        text: "no".into(),
    }));
}

#[test]
fn set_node_text_on_text_node() {
    let mut s = sample(); // n11 is a Text node
    assert!(s.apply(EditorCommand::SetNodeText {
        node_id: id("n11"),
        text: "Updated".into(),
    }));
    match find_node(s.active_children(), &id("n11")).unwrap() {
        PenNode::Text(t) => match &t.content {
            jian_ops_schema::node::TextContent::Plain(p) => assert_eq!(p, "Updated"),
            _ => panic!("expected plain content"),
        },
        _ => panic!("expected text node"),
    }
}

#[test]
fn set_node_font_weight_range_checked() {
    let mut s = sample();
    assert!(s.apply(EditorCommand::SetNodeFontWeight {
        node_id: id("n11"),
        font_weight: 700,
    }));
    // Out of range → rejected.
    assert!(!s.apply(EditorCommand::SetNodeFontWeight {
        node_id: id("n11"),
        font_weight: 0,
    }));
}

#[test]
fn set_node_stroke_width_zero_clears_stroke() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::SetNodeStrokeHex {
        node_id: id("n1"),
        hex: "#000000".into(),
    }));
    assert!(crate::fills::first_solid_stroke_hex(
        find_node(s.active_children(), &id("n1")).unwrap()
    )
    .is_some());
    assert!(s.apply(EditorCommand::SetNodeStrokeWidth {
        node_id: id("n1"),
        width: 0.0,
    }));
    // Stroke gone.
    assert!(crate::fills::first_solid_stroke_hex(
        find_node(s.active_children(), &id("n1")).unwrap()
    )
    .is_none());
}

#[test]
fn commit_property_edit_writes_stroke_width() {
    // Regression: committing the stroke-width input used to be a no-op
    // in `commit_property_edit`, so the typed value reset on commit.
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::SetNodeStrokeHex {
        node_id: id("n1"),
        hex: "#114194".into(),
    }));
    s.set_single_selection(id("n1"));
    assert!(s.commit_property_edit(crate::ui_draft::PropertyFocus::StrokeWidth, 5.0));
    let w = crate::fills::node_stroke_width(find_node(s.active_children(), &id("n1")).unwrap());
    assert_eq!(w, Some(5.0));
}

#[test]
fn commit_property_edit_clamps_font_size() {
    // TS parity (text-section.tsx:134): font-size input is min=1 max=999.
    let mut s = state_with(vec![text("t1", "Title", 0.0, 0.0, 100.0, 30.0, "Hi")]);
    s.set_single_selection(id("t1"));
    assert!(s.commit_property_edit(crate::ui_draft::PropertyFocus::FontSize, 5000.0));
    let PenNode::Text(t) = find_node(s.active_children(), &id("t1")).unwrap() else {
        panic!("text node");
    };
    assert_eq!(t.font_size, Some(999.0), "over-max clamps to 999");
    assert!(s.commit_property_edit(crate::ui_draft::PropertyFocus::FontSize, 0.4));
    let PenNode::Text(t) = find_node(s.active_children(), &id("t1")).unwrap() else {
        panic!("text node");
    };
    assert_eq!(t.font_size, Some(1.0), "under-min clamps to 1");
}

#[test]
fn set_node_corner_radius_rejects_negative() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::SetNodeCornerRadius {
        node_id: id("n1"),
        radius: -1.0,
    }));
    assert!(s.apply(EditorCommand::SetNodeCornerRadius {
        node_id: id("n1"),
        radius: 8.0,
    }));
}

#[test]
fn set_node_flag_hidden_and_locked() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::SetNodeFlag {
        node_id: id("n1"),
        flag: NodeFlag::Hidden,
        value: true,
    }));
    assert!(s.apply(EditorCommand::SetNodeFlag {
        node_id: id("n1"),
        flag: NodeFlag::Locked,
        value: true,
    }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().visible, Some(false));
    assert_eq!(n.base().locked, Some(true));
}

#[test]
fn set_node_flag_collapsed_is_unsupported() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    // No `collapsed` field on the canonical schema → rejected.
    assert!(!s.apply(EditorCommand::SetNodeFlag {
        node_id: id("n1"),
        flag: NodeFlag::Collapsed,
        value: true,
    }));
}

// --- Selection -------------------------------------------------------

#[test]
fn set_selection_rejects_off_page_id() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::SetSelection {
        node_id: id("ghost")
    }));
    assert!(s.apply(EditorCommand::SetSelection { node_id: id("n1") }));
    assert_eq!(s.selection.anchor, id("n1"));
}

#[test]
fn set_selection_set_drops_unknown_ids() {
    let mut s = state_with(vec![
        rect("n1", "r", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "r", 20.0, 0.0, 10.0, 10.0),
    ]);
    assert!(s.apply(EditorCommand::SetSelectionSet {
        node_ids: vec![id("n1"), id("ghost"), id("n2")],
    }));
    assert_eq!(s.selection.set, vec![id("n1"), id("n2")]);
}

#[test]
fn clear_selection_command() {
    let mut s = sample();
    assert!(s.apply(EditorCommand::ClearSelection));
    assert!(s.selection.is_empty());
}

// --- Selection-scoped tree ops + history -----------------------------

#[test]
fn delete_selected_pushes_history() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(id("n1"));
    assert!(s.apply(EditorCommand::DeleteSelected));
    assert!(s.active_children().is_empty());
    assert!(s.history.can_undo());
}

#[test]
fn delete_selected_no_op_on_empty_selection() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.clear_selection();
    assert!(!s.apply(EditorCommand::DeleteSelected));
    assert!(!s.history.can_undo());
}

#[test]
fn nudge_selected_translates_and_records_history() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(id("n1"));
    assert!(s.apply(EditorCommand::NudgeSelected { dx: 5, dy: 7 }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().x, Some(5.0));
    assert_eq!(n.base().y, Some(7.0));
    assert!(s.history.can_undo());
}

#[test]
fn nudge_selected_flex_child_is_no_op_without_history() {
    let mut s = state_with(vec![flex_frame(
        "n1",
        "Flex",
        0.0,
        0.0,
        200.0,
        300.0,
        vec![flow_rect("n2", "A", 80.0, 24.0)],
    )]);
    s.set_single_selection(id("n2"));

    assert!(!s.apply(EditorCommand::NudgeSelected { dx: 5, dy: 7 }));

    let n = find_node(s.active_children(), &id("n2")).unwrap();
    assert_eq!(n.base().x, None);
    assert_eq!(n.base().y, None);
    assert!(!s.history.can_undo());
}

#[test]
fn duplicate_selected_clones_with_fresh_id() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(id("n1"));
    assert!(s.apply(EditorCommand::DuplicateSelected { offset_px: 10 }));
    assert_eq!(s.active_children().len(), 2);
    assert!(s.find_duplicate_id().is_none());
}

#[test]
fn undo_redo_round_trip_via_command() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(id("n1"));
    assert!(s.apply(EditorCommand::DeleteSelected));
    assert!(s.active_children().is_empty());
    assert!(s.apply(EditorCommand::Undo));
    assert_eq!(s.active_children().len(), 1);
    assert!(s.apply(EditorCommand::Redo));
    assert!(s.active_children().is_empty());
}

// --- Clipboard -------------------------------------------------------

#[test]
fn copy_then_paste_clipboard() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(id("n1"));
    assert!(s.apply(EditorCommand::CopySelected));
    assert!(s.apply(EditorCommand::PasteClipboard { offset_px: 10 }));
    assert_eq!(s.active_children().len(), 2);
}

#[test]
fn paste_clipboard_no_op_when_empty() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::PasteClipboard { offset_px: 10 }));
}

// --- Tool + viewport -------------------------------------------------

#[test]
fn set_active_tool_command() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::SetActiveTool {
        tool: "ellipse".into()
    }));
    assert_eq!(s.tool, crate::tool::Tool::Ellipse);
    assert!(!s.apply(EditorCommand::SetActiveTool {
        tool: "bogus".into()
    }));
}

#[test]
fn set_viewport_partial_axes() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::SetViewport {
        pan_x: Some(100),
        pan_y: None,
        zoom_percent: Some(200),
    }));
    assert_eq!(s.viewport.pan_x, 100.0);
    assert_eq!(s.viewport.zoom, 2.0);
}

// --- Pages -----------------------------------------------------------

#[test]
fn add_then_set_active_page() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::AddPage {
        name: None,
        children: None,
    }));
    assert_eq!(s.page_count(), 2);
    assert!(s.apply(EditorCommand::SetActivePage { index: 0 }));
    assert_eq!(s.ui.active_page_index, 0);
    assert!(!s.apply(EditorCommand::SetActivePage { index: 9 }));
}

#[test]
fn add_page_command_accepts_custom_name() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::AddPage {
        name: Some("Checkout".into()),
        children: None,
    }));
    let pages = s.doc.pages.as_ref().expect("multi-page doc");
    assert_eq!(pages[1].name, "Checkout");
}

#[test]
fn duplicate_page_command_accepts_custom_name() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::AddPage {
        name: Some("Checkout".into()),
        children: None,
    }));
    assert!(s.apply(EditorCommand::DuplicatePage {
        index: 1,
        name: Some("Checkout copy 2".into()),
    }));
    let pages = s.doc.pages.as_ref().expect("multi-page doc");
    assert_eq!(pages[2].name, "Checkout copy 2");
}

// --- Variables -------------------------------------------------------

#[test]
fn create_then_set_variable_via_command() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::CreateVariable {
        name: "brand".into(),
        kind: "color".into(),
        default_value: "#ff0000".into(),
    }));
    assert!(s.apply(EditorCommand::SetVariableColor {
        name: "brand".into(),
        hex: "#00ff00".into(),
    }));
    match s.resolve_variable("brand") {
        Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#00ff00"),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn create_variable_rejects_bad_number_default() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::CreateVariable {
        name: "n".into(),
        kind: "number".into(),
        default_value: "not-a-number".into(),
    }));
}

#[test]
fn set_variable_scalar_number() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::CreateVariable {
        name: "gap".into(),
        kind: "number".into(),
        default_value: "8".into(),
    }));
    assert!(s.apply(EditorCommand::SetVariableScalar {
        name: "gap".into(),
        scalar: VariableScalarPayload::Number(16.0),
    }));
    match s.resolve_variable("gap") {
        Some(VariableScalar::Num(n)) => assert_eq!(*n, 16.0),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn set_variables_command_merges_and_replaces_definitions() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::CreateVariable {
        name: "brand".into(),
        kind: "color".into(),
        default_value: "#ff0000".into(),
    }));

    let mut incoming = std::collections::BTreeMap::new();
    incoming.insert(
        "gap".into(),
        VariableDefinition {
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(8.0)),
        },
    );
    assert!(s.apply(EditorCommand::SetVariables {
        variables: incoming,
        replace: false,
    }));
    let vars = s.doc.variables.as_ref().expect("variables");
    assert!(vars.contains_key("brand"));
    assert!(vars.contains_key("gap"));

    let mut replacement = std::collections::BTreeMap::new();
    replacement.insert(
        "enabled".into(),
        VariableDefinition {
            kind: VariableKind::Boolean,
            value: VariableValue::Scalar(VariableScalar::Bool(true)),
        },
    );
    assert!(s.apply(EditorCommand::SetVariables {
        variables: replacement,
        replace: true,
    }));
    let vars = s.doc.variables.as_ref().expect("variables");
    assert!(!vars.contains_key("brand"));
    assert!(!vars.contains_key("gap"));
    assert!(vars.contains_key("enabled"));
}

#[test]
fn set_themes_command_merges_and_replaces_axes() {
    let mut s = state_with(vec![]);
    let mut themes = std::collections::BTreeMap::new();
    themes.insert("Mode".into(), vec!["Light".into(), "Dark".into()]);
    assert!(s.apply(EditorCommand::SetThemes {
        themes,
        replace: false,
    }));
    assert_eq!(
        s.doc.themes.as_ref().and_then(|t| t.get("Mode")).cloned(),
        Some(vec!["Light".into(), "Dark".into()])
    );

    let mut replacement = std::collections::BTreeMap::new();
    replacement.insert(
        "Density".into(),
        vec!["Compact".into(), "Comfortable".into()],
    );
    assert!(s.apply(EditorCommand::SetThemes {
        themes: replacement,
        replace: true,
    }));
    let themes = s.doc.themes.as_ref().expect("themes");
    assert!(!themes.contains_key("Mode"));
    assert!(themes.contains_key("Density"));
}

// --- Group / ungroup / align -----------------------------------------

#[test]
fn group_then_ungroup_via_command() {
    let mut s = state_with(vec![
        rect("n1", "r", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "r", 20.0, 0.0, 10.0, 10.0),
    ]);
    s.selection.set = vec![id("n1"), id("n2")];
    s.selection.anchor = id("n2");
    assert!(s.apply(EditorCommand::GroupSelected));
    // The single root is now a Group.
    assert_eq!(s.active_children().len(), 1);
    assert!(s.active_children()[0].is_group());
    assert!(s.apply(EditorCommand::UngroupSelected));
    assert_eq!(s.active_children().len(), 2);
}

#[test]
fn align_selected_via_command() {
    let mut s = state_with(vec![
        rect("n1", "r", 10.0, 0.0, 40.0, 20.0),
        rect("n2", "r", 50.0, 100.0, 30.0, 20.0),
    ]);
    s.selection.set = vec![id("n1"), id("n2")];
    s.selection.anchor = id("n2");
    assert!(s.apply(EditorCommand::AlignSelected {
        action: "left".into()
    }));
    assert!(!s.apply(EditorCommand::AlignSelected {
        action: "bogus".into()
    }));
}
