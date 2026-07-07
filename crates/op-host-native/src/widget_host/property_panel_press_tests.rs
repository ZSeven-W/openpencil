use super::WidgetHostNative;
use op_editor_core::codegen::{CodegenHover, CodegenPhase};
use op_editor_core::PropertyTab;
use op_editor_core::{ButtonPressTarget, NodeId};
use op_editor_ui::widgets::property_panel_action::CodegenAction;
use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn point_for_action(
    host: &WidgetHostNative,
    want: impl Fn(&PropertyPanelAction) -> bool,
) -> Point2D {
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if panel
                .hit_test_action(rect, point)
                .as_ref()
                .is_some_and(&want)
            {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no property-panel action point maps to requested action");
}

fn point_inside_property_panel_without_target(host: &WidgetHostNative) -> Point2D {
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let mut y = rect.origin.y + rect.size.y - 12.0;
    while y > rect.origin.y {
        let mut x = rect.origin.x + 12.0;
        while x < rect.origin.x + rect.size.x - 12.0 {
            let point = Point2D::new(x, y);
            let no_action = panel.hit_test_action(rect, point).is_none();
            let no_input = panel.hit_test(rect, point).is_none();
            if no_action && no_input {
                return point;
            }
            x += 8.0;
        }
        y -= 8.0;
    }
    panic!("no empty property-panel point found");
}

#[test]
fn property_panel_action_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));

    let point = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleSizeFillWidth)
    });
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let expected_index = panel
        .action_hover_index(rect, point)
        .expect("action maps to hover index");

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::PropertyPanel(expected_index))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn property_panel_background_consumes_clicks() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"group","id":"shape_group","name":"Shape Group",
               "children":[
                 {"type":"rectangle","id":"box","name":"Box","width":80,"height":40}
               ]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("shape_group"));

    let point = point_inside_property_panel_without_target(&host);
    assert!(
        host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        "right inspector should own clicks inside its bounds even when no control is hit"
    );
    assert_eq!(
        host.editor_state().selection.anchor,
        NodeId::new("shape_group")
    );
}

#[test]
fn native_property_panel_group_component_button_switches_to_detach() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"frame","id":"screen","name":"Screen",
               "x":40,"y":40,"width":360,"height":640,
               "children":[
                 {"type":"group","id":"shape_group","name":"Shape Group",
                  "children":[
                    {"type":"rectangle","id":"box","name":"Box","width":80,"height":40}
                  ]}
               ]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("shape_group"));

    let create = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::CreateComponent)
    });
    assert!(host.apply_press(create.x, create.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("shape_group"))
        .is_some());

    let detach = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::DetachComponent)
    });
    assert!(host.apply_press(detach.x, detach.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("shape_group"))
        .is_none());
}

#[test]
fn codegen_action_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;
    host.editor_state_mut().codegen.phase = CodegenPhase::Complete;
    host.editor_state_mut().codegen.code = "fn main() {\n    println!(\"hi\");\n}\n".into();

    let point = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::Codegen(CodegenAction::Copy))
    });

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::Codegen(CodegenHover::Copy))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}
