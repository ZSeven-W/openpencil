use super::WidgetHost;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::editor_ui_state::EffectParamFocus;
use op_editor_core::ui_draft::PropertyFocus;
use op_editor_core::{own_bounds, EffectField, NodeId, PropertyTab, Tool};
use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction, Toolbar, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

const W: f32 = 1280.0;
const H: f32 = 900.0;

fn seed(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

#[test]
fn web_move_fill_action_dispatches_as_one_undoable_edit() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{"version":"0.8.0","children":[{
          "type":"rectangle","id":"rect","name":"Rect",
          "x":0,"y":0,"width":10,"height":10,
          "fill":[
            {"type":"solid","color":"#111111"},
            {"type":"solid","color":"#222222"},
            {"type":"solid","color":"#333333"}
          ]
        }]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("rect"));

    host.apply_property_action(PropertyPanelAction::MoveFill { from: 2, to: 0 });

    let node = op_editor_core::walkers::find_node(
        host.editor_state.active_children(),
        &NodeId::new("rect"),
    )
    .expect("rect exists");
    let colors: Vec<_> = op_editor_core::fills::node_fills(node)
        .expect("fills exist")
        .iter()
        .map(|fill| match fill {
            PenFill::Solid(body) => body.color.as_str(),
            other => panic!("expected solid, got {other:?}"),
        })
        .collect();
    assert_eq!(colors, ["#333333", "#111111", "#222222"]);
    assert_eq!(host.editor_state.history.past.len(), 1);
}

fn point_for_property_focus(host: &WidgetHost, want: PropertyFocus) -> (f32, f32) {
    let panel = PropertyPanel::for_selection(&host.editor_state)
        .expect("fixture selection shows property panel");
    let pw = host.editor_state.editor_ui.property_panel_width;
    let rect = Rect {
        origin: Point2D::new(W - pw, TOP_BAR_HEIGHT),
        size: Point2D::new(pw, H - TOP_BAR_HEIGHT),
    };
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            if panel.hit_test(rect, Point2D::new(x, y)) == Some(want) {
                return (x, y);
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no property-panel point maps to {want:?}");
}

fn point_for_property_action(
    host: &WidgetHost,
    want: impl Fn(&PropertyPanelAction) -> bool,
) -> (f32, f32) {
    let panel = PropertyPanel::for_selection(&host.editor_state)
        .expect("fixture selection shows property panel");
    let pw = host.editor_state.editor_ui.property_panel_width;
    let rect = Rect {
        origin: Point2D::new(W - pw, TOP_BAR_HEIGHT),
        size: Point2D::new(pw, H - TOP_BAR_HEIGHT),
    };
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            if panel
                .hit_test_action(rect, Point2D::new(x, y))
                .as_ref()
                .is_some_and(&want)
            {
                return (x, y);
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no property-panel action point maps to requested action");
}

fn point_for_toolbar_tool(host: &mut WidgetHost, want: Tool) -> (f32, f32) {
    let rect = host.toolbar_rect(W);
    let toolbar = Toolbar::for_editor(&host.editor_state);
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            if toolbar.hit_test(rect, Point2D::new(x, y))
                == Some(op_editor_ui::widgets::ToolbarHit::Tool(want))
            {
                return (x, y);
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no toolbar point maps to {want:?}");
}

fn selected_ref(host: &WidgetHost) -> &jian_ops_schema::node::RefNode {
    match op_editor_core::walkers::find_node(
        host.editor_state.active_children(),
        &NodeId::new("inst1"),
    ) {
        Some(PenNode::Ref(r)) => r,
        other => panic!("inst1 must stay a Ref, got {other:?}"),
    }
}

fn node_bounds(host: &WidgetHost, id: &str) -> op_editor_core::DocRect {
    let id = NodeId::new(id);
    let node = op_editor_core::walkers::find_node(host.editor_state.active_children(), &id)
        .expect("fixture node exists");
    own_bounds(node)
}

#[test]
fn web_property_action_press_commits_prior_property_input() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");

    let (x, y) = point_for_property_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::SetPropertyTab(PropertyTab::Code)
        )
    });
    assert!(host.apply_press(x, y, W, H));

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state.ui.property_focus.is_none());
    assert_eq!(host.editor_state.ui.property_input.text(), "");
    assert_eq!(host.editor_state.editor_ui.property_tab, PropertyTab::Code);
}

#[test]
fn web_toolbar_press_commits_property_input_before_tool_switch() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");

    let (x, y) = point_for_toolbar_tool(&mut host, Tool::Text);
    assert!(host.apply_press(x, y, W, H));

    assert_eq!(node_bounds(&host, "n62").w, 321.0);
    assert!(host.editor_state.ui.property_focus.is_none());
    assert_eq!(host.editor_state.ui.property_input.text(), "");
    assert_eq!(host.editor_state.tool, Tool::Text);
}

#[test]
fn property_input_uses_text_input_state_for_web_editing() {
    let mut host = WidgetHost::new();
    {
        let ui = &mut host.editor_state.ui;
        ui.property_focus = Some(PropertyFocus::PositionX);
        ui.property_input.set_text("1234");
    }

    assert!(host.apply_property_caret(false));
    assert!(host.apply_property_caret(false));
    assert_eq!(host.editor_state.ui.property_input.caret(), 2);

    assert!(host.apply_text('9'));
    assert_eq!(host.editor_state.ui.property_input.text(), "12934");
    assert_eq!(host.editor_state.ui.property_input.caret(), 3);

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state.ui.property_input.text(), "1234");
    assert_eq!(host.editor_state.ui.property_input.caret(), 2);
}

#[test]
fn web_property_press_reseeds_from_committed_snapshot() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    let (x, y) = point_for_property_focus(&host, PropertyFocus::SizeW);
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");

    assert!(host.apply_press(x, y, W, H));

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert_eq!(
        host.editor_state.ui.property_focus,
        Some(PropertyFocus::SizeW)
    );
    assert_eq!(host.editor_state.ui.property_input.text(), "321");
}

#[test]
fn web_effect_param_refocus_reseeds_from_committed_snapshot() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Shadowed",
               "x":40,"y":40,"width":180,"height":120,
               "effects":[{"type":"shadow","offsetX":12,"offsetY":4,
                 "blur":8,"spread":0,"color":"#00000040"}],
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    host.editor_state.editor_ui.effect_param_focus = Some(EffectParamFocus {
        effect: 0,
        field: EffectField::OffsetX,
    });
    host.editor_state.ui.property_input.set_text("20");

    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::FocusEffectParam {
            effect: 0,
            field: EffectField::OffsetX,
            value: 12.0,
        },
    );

    assert_eq!(host.editor_state.ui.property_input.text(), "20");
    assert_eq!(
        host.editor_state.editor_ui.effect_param_focus,
        Some(EffectParamFocus {
            effect: 0,
            field: EffectField::OffsetX,
        })
    );
}

#[test]
fn web_effect_param_commit_on_instance_creates_override() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version":"0.8.0", "children":[
              {"type":"rectangle","id":"card","name":"Card","reusable":true,
               "x":0,"y":0,"width":200,"height":100,
               "effects":[{"type":"shadow","offsetX":12,"offsetY":4,
                 "blur":8,"spread":0,"color":"#00000040"}],
               "fill":[{"type":"solid","color":"#222222"}]},
              {"type":"ref","id":"inst1","ref":"card","x":300,"y":50}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("inst1"));
    host.editor_state.editor_ui.effect_param_focus = Some(EffectParamFocus {
        effect: 0,
        field: EffectField::OffsetX,
    });
    host.editor_state.ui.property_input.set_text("24");

    host.commit_effect_param_focus_if_any();

    let over = selected_ref(&host)
        .descendants
        .as_ref()
        .and_then(|d| d.get("card"))
        .expect("effect override routed under descendants[card]");
    assert_eq!(
        over.pointer("/effects/0/offsetX").and_then(|v| v.as_f64()),
        Some(24.0)
    );
}

#[test]
fn web_effect_param_focus_and_typing_use_text_input_state() {
    let mut host = WidgetHost::new();

    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::FocusEffectParam {
            effect: 0,
            field: op_editor_core::EffectField::OffsetX,
            value: 12.5,
        },
    );

    assert_eq!(
        host.editor_state.editor_ui.effect_param_focus,
        Some(op_editor_core::editor_ui_state::EffectParamFocus {
            effect: 0,
            field: op_editor_core::EffectField::OffsetX,
        })
    );
    assert_eq!(host.editor_state.ui.property_input.text(), "12.5");

    assert!(host.apply_text('6'));
    assert_eq!(host.editor_state.ui.property_input.text(), "12.56");
}

#[test]
fn web_effect_param_focus_commits_prior_property_input() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");

    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::FocusEffectParam {
            effect: 0,
            field: EffectField::OffsetX,
            value: 12.0,
        },
    );

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state.ui.property_focus.is_none());
    assert_eq!(
        host.editor_state.editor_ui.effect_param_focus,
        Some(EffectParamFocus {
            effect: 0,
            field: EffectField::OffsetX,
        })
    );
    assert_eq!(host.editor_state.ui.property_input.text(), "12");
}

#[test]
fn web_property_focus_commit_flushes_variable_row_focus_first() {
    let mut host = WidgetHost::new();
    assert!(host.editor_state.create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into())
    ));
    host.editor_state.editor_ui.variable_row_focus =
        Some(op_editor_core::editor_ui_state::VariableRowFocus::Name(0));
    host.editor_state
        .editor_ui
        .variable_row_input
        .set_text("brand");

    host.commit_property_focus_if_any();

    assert!(host.editor_state.editor_ui.variable_row_focus.is_none());
    let vars = host.editor_state.doc.variables.as_ref().unwrap();
    assert!(vars.contains_key("brand"));
    assert!(!vars.contains_key("color-1"));
}

#[test]
fn web_property_focus_commit_reads_text_input_state() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");

    host.commit_property_focus_if_any();

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state.ui.property_input.text().is_empty());
}

#[test]
fn web_property_focus_commit_is_undoable() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");

    host.commit_property_focus_if_any();

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state.history.can_undo());

    assert!(host.editor_state.undo());
    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 180.0);
}

#[test]
fn web_property_focus_commit_on_instance_undo_restores_ref() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version":"0.8.0", "children":[
              {"type":"rectangle","id":"card","name":"Card","reusable":true,
               "x":0,"y":0,"width":200,"height":100,
               "fill":[{"type":"solid","color":"#222222"}]},
              {"type":"ref","id":"inst1","ref":"card","x":300,"y":50}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("inst1"));
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("240");

    host.commit_property_focus_if_any();

    let over = selected_ref(&host)
        .descendants
        .as_ref()
        .and_then(|d| d.get("card"))
        .expect("width override routed under descendants[card]");
    assert_eq!(over.pointer("/width").and_then(|v| v.as_f64()), Some(240.0));
    assert!(host.editor_state.history.can_undo());

    assert!(host.editor_state.undo());
    assert!(selected_ref(&host)
        .descendants
        .as_ref()
        .map(|d| !d.contains_key("card"))
        .unwrap_or(true));
}

#[test]
fn web_property_delete_uses_text_input_state() {
    let mut host = WidgetHost::new();
    host.editor_state.set_single_selection(NodeId::new("n10"));
    {
        let ui = &mut host.editor_state.ui;
        ui.property_focus = Some(PropertyFocus::PositionX);
        ui.property_input.set_text("123");
    }
    assert!(host.apply_property_caret(false));

    assert!(host.apply_delete());

    assert_eq!(host.editor_state.ui.property_input.text(), "12");
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("n10"));
}

#[test]
fn web_property_escape_clears_text_input_state() {
    let mut host = WidgetHost::new();
    {
        let ui = &mut host.editor_state.ui;
        ui.property_focus = Some(PropertyFocus::PositionX);
        ui.property_input.set_text("123");
    }

    assert!(host.apply_escape());

    assert!(host.editor_state.ui.property_focus.is_none());
    assert!(host.editor_state.ui.property_input.text().is_empty());
}
