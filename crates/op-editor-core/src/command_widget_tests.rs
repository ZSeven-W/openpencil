//! Phase D2 — form-widget tool + InsertNode kind-build coverage.
//!
//! Two construction paths must build real widget `PenNode`s with the
//! spec default props:
//!
//!   - the canvas-click path ([`EditorState::create_node_for_tool`]),
//!     selected via one of the ten [`Tool::WIDGETS`] tools, and
//!   - the command / MCP path
//!     ([`EditorCommand::InsertNode { kind, .. }`]) with a widget kind
//!     string.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::state_with;
use crate::tool::Tool;
use jian_ops_schema::node::{BoolOrExpression, NumberOrExpression, PenNode};
use jian_ops_schema::sizing::SizingBehavior;

/// Build an `InsertNode` command for `kind` at the page root with a
/// zero box (the factory writes the literal box; widgets ignore the
/// fill_hex slot). Width/height are supplied explicitly here so the
/// MCP-style path is exercised end-to-end.
fn insert(kind: &str, w: i32, h: i32) -> EditorCommand {
    EditorCommand::InsertNode {
        kind: kind.into(),
        name: kind.into(),
        x: 5,
        y: 6,
        width: w,
        height: h,
        fill_hex: None,
        target_parent: NodeId::NONE,
        page_id: None,
    }
}

#[test]
fn insert_node_builds_a_text_input_with_default_placeholder() {
    let mut s = state_with(vec![]);
    assert!(s.apply(insert("text_input", 240, 40)));
    assert_eq!(s.active_children().len(), 1);
    let PenNode::TextInput(ti) = &s.active_children()[0] else {
        panic!(
            "expected a TextInput node, got {:?}",
            s.active_children()[0]
        );
    };
    assert_eq!(ti.placeholder.as_deref(), Some("Enter text"));
    assert_eq!(ti.width, Some(SizingBehavior::Number(240.0)));
    assert_eq!(ti.height, Some(SizingBehavior::Number(40.0)));
    assert_eq!(ti.base.x, Some(5.0));
}

#[test]
fn insert_node_builds_a_slider_with_default_range() {
    let mut s = state_with(vec![]);
    assert!(s.apply(insert("slider", 240, 20)));
    let PenNode::Slider(sl) = &s.active_children()[0] else {
        panic!("expected a Slider node");
    };
    assert_eq!(sl.min, Some(0.0));
    assert_eq!(sl.max, Some(100.0));
    assert_eq!(sl.step, Some(1.0));
    assert_eq!(sl.value, Some(NumberOrExpression::Number(50.0)));
}

#[test]
fn insert_node_builds_a_checkbox_with_label_and_unchecked() {
    let mut s = state_with(vec![]);
    assert!(s.apply(insert("checkbox", 18, 18)));
    let PenNode::Checkbox(cb) = &s.active_children()[0] else {
        panic!("expected a Checkbox node");
    };
    assert_eq!(cb.label.as_deref(), Some("Label"));
    assert_eq!(cb.checked, Some(BoolOrExpression::Bool(false)));
}

#[test]
fn insert_node_builds_a_progress_with_default_value_and_max() {
    let mut s = state_with(vec![]);
    assert!(s.apply(insert("progress", 240, 8)));
    let PenNode::Progress(p) = &s.active_children()[0] else {
        panic!("expected a Progress node");
    };
    assert_eq!(p.value, Some(NumberOrExpression::Number(40.0)));
    assert_eq!(p.max, Some(100.0));
}

#[test]
fn insert_node_builds_a_select_and_tabs_container() {
    let mut s = state_with(vec![]);
    assert!(s.apply(insert("select", 240, 40)));
    assert!(s.apply(insert("tabs", 320, 200)));
    let PenNode::Select(sel) = &s.active_children()[0] else {
        panic!("expected a Select node");
    };
    assert_eq!(sel.placeholder.as_deref(), Some("Select\u{2026}"));
    assert_eq!(sel.options.as_deref(), Some(&[][..]));
    let PenNode::Tabs(tabs) = &s.active_children()[1] else {
        panic!("expected a Tabs node");
    };
    assert_eq!(tabs.tabs.as_deref(), Some(&[][..]));
    // Tabs is a container — it must start with an (empty) children vec
    // so a follow-up reparent / panel insert has a slot.
    assert!(tabs.children.is_some());
    assert!(s.active_children()[1].is_container());
}

#[test]
fn insert_node_accepts_all_ten_widget_kinds() {
    for tool in Tool::WIDGETS {
        let kind = tool.widget_kind().expect("widget tool maps to a kind");
        let mut s = state_with(vec![]);
        let (w, h) = crate::widget_default_size(kind).expect("widget has a default size");
        assert!(
            s.apply(insert(kind, w, h)),
            "InsertNode should accept widget kind {kind:?}"
        );
        assert_eq!(s.active_children().len(), 1, "kind {kind:?} should insert");
    }
}

#[test]
fn create_node_for_widget_tool_uses_spec_default_size() {
    // Click-create a TextInput: the host passes a tiny 1x1 drag-init
    // box, but the widget should land at its spec default 240x40.
    let mut s = state_with(vec![]);
    let mut next = 0u64;
    let id = s
        .create_node_for_tool(Tool::TextInput, &mut next, 100.0, 200.0, 1.0, 1.0)
        .expect("widget tool is creatable");
    assert!(id.is_real());
    let PenNode::TextInput(ti) = &s.active_children()[0] else {
        panic!("expected a TextInput node");
    };
    assert_eq!(ti.width, Some(SizingBehavior::Number(240.0)));
    assert_eq!(ti.height, Some(SizingBehavior::Number(40.0)));
    assert_eq!(ti.base.x, Some(100.0));
    assert_eq!(ti.base.y, Some(200.0));
    assert_eq!(ti.placeholder.as_deref(), Some("Enter text"));
}

#[test]
fn create_node_for_switch_tool_builds_a_switch_at_default_box() {
    let mut s = state_with(vec![]);
    let mut next = 0u64;
    s.create_node_for_tool(Tool::Switch, &mut next, 0.0, 0.0, 999.0, 999.0)
        .expect("switch tool is creatable");
    let PenNode::Switch(sw) = &s.active_children()[0] else {
        panic!("expected a Switch node");
    };
    // Spec default 44x24 — NOT the 999x999 drag-init the caller passed.
    assert_eq!(sw.width, Some(SizingBehavior::Number(44.0)));
    assert_eq!(sw.height, Some(SizingBehavior::Number(24.0)));
    assert_eq!(sw.checked, Some(BoolOrExpression::Bool(false)));
}

// ---------------------------------------------------------------------------
// Phase D3 — property-panel widget-prop setters.
// ---------------------------------------------------------------------------

/// Insert a widget of `kind`, select it, and return the state + the
/// selected node's id so a setter can be exercised against it.
fn state_with_selected_widget(kind: &str, w: i32, h: i32) -> (crate::EditorState, NodeId) {
    let mut s = state_with(vec![]);
    assert!(s.apply(insert(kind, w, h)));
    let id = NodeId::new(s.active_children()[0].id_str().to_string());
    s.set_single_selection(id.clone());
    (s, id)
}

#[test]
fn set_selected_widget_text_writes_placeholder_on_text_input() {
    let (mut s, _) = state_with_selected_widget("text_input", 240, 40);
    assert!(s.set_selected_widget_text(crate::WidgetTextField::Placeholder, "Your name"));
    let PenNode::TextInput(ti) = &s.active_children()[0] else {
        panic!("expected a TextInput node");
    };
    assert_eq!(ti.placeholder.as_deref(), Some("Your name"));
}

#[test]
fn set_selected_widget_text_clears_prop_on_empty_string() {
    let (mut s, _) = state_with_selected_widget("text_input", 240, 40);
    // The factory seeds a placeholder; an empty edit clears it.
    assert!(s.set_selected_widget_text(crate::WidgetTextField::Placeholder, ""));
    let PenNode::TextInput(ti) = &s.active_children()[0] else {
        panic!("expected a TextInput node");
    };
    assert_eq!(ti.placeholder, None);
}

#[test]
fn set_selected_widget_text_writes_value_on_select() {
    let (mut s, _) = state_with_selected_widget("select", 240, 40);
    assert!(s.set_selected_widget_text(crate::WidgetTextField::Value, "opt-a"));
    let PenNode::Select(sel) = &s.active_children()[0] else {
        panic!("expected a Select node");
    };
    assert_eq!(sel.value.as_deref(), Some("opt-a"));
}

#[test]
fn set_selected_widget_text_rejects_field_the_variant_lacks() {
    // A TextInput has no `label`; the write must report failure.
    let (mut s, _) = state_with_selected_widget("text_input", 240, 40);
    assert!(!s.set_selected_widget_text(crate::WidgetTextField::Label, "nope"));
}

#[test]
fn set_selected_widget_checked_toggles_switch() {
    let (mut s, _) = state_with_selected_widget("switch", 44, 24);
    assert!(s.set_selected_widget_checked(true));
    let PenNode::Switch(sw) = &s.active_children()[0] else {
        panic!("expected a Switch node");
    };
    assert_eq!(sw.checked, Some(BoolOrExpression::Bool(true)));
}

#[test]
fn set_selected_widget_checked_rejects_non_toggle_kind() {
    let (mut s, _) = state_with_selected_widget("text_input", 240, 40);
    assert!(!s.set_selected_widget_checked(true));
}

#[test]
fn commit_property_edit_writes_slider_min_max_step() {
    use crate::PropertyFocus;
    let (mut s, _) = state_with_selected_widget("slider", 240, 20);
    assert!(s.commit_property_edit(PropertyFocus::WidgetMin, 10.0));
    assert!(s.commit_property_edit(PropertyFocus::WidgetMax, 200.0));
    assert!(s.commit_property_edit(PropertyFocus::WidgetStep, 5.0));
    let PenNode::Slider(sl) = &s.active_children()[0] else {
        panic!("expected a Slider node");
    };
    assert_eq!(sl.min, Some(10.0));
    assert_eq!(sl.max, Some(200.0));
    assert_eq!(sl.step, Some(5.0));
}

#[test]
fn widget_number_field_value_overwrites_expression_binding() {
    let (mut s, id) = state_with_selected_widget("number_input", 240, 40);
    // Seed an expression binding, then a literal value edit replaces it.
    if let PenNode::NumberInput(n) =
        crate::walkers::find_node_mut(s.active_children_mut(), &id).unwrap()
    {
        n.value = Some(NumberOrExpression::Expression("$count".into()));
    }
    assert!(s.commit_property_edit(crate::PropertyFocus::WidgetMax, 42.0));
    let PenNode::NumberInput(n) = &s.active_children()[0] else {
        panic!("expected a NumberInput node");
    };
    assert_eq!(n.max, Some(42.0));
}

#[test]
fn widget_kind_round_trips_through_default_size() {
    // Every widget tool maps to a kind, and every such kind has a
    // default size — the two tables stay in lock-step.
    for tool in Tool::WIDGETS {
        let kind = tool.widget_kind().expect("widget tool has a kind");
        assert!(
            crate::widget_default_size(kind).is_some(),
            "kind {kind:?} should have a default size"
        );
        assert!(
            crate::command_node::kind_is_valid(kind),
            "kind {kind:?} should be a buildable kind"
        );
    }
    // Non-widget tools map to no widget kind.
    assert_eq!(Tool::Select.widget_kind(), None);
    assert_eq!(Tool::Rect.widget_kind(), None);
}
