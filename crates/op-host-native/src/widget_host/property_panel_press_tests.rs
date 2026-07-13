use super::WidgetHostNative;
use op_editor_core::codegen::{CodegenHover, CodegenPhase};
use op_editor_core::PropertyTab;
use op_editor_core::{ButtonPressTarget, NodeId, PropertyFocus};
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

fn point_for_focus(host: &WidgetHostNative, want: PropertyFocus) -> Point2D {
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if panel.hit_test(rect, point) == Some(want) {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no property-panel input point maps to {want:?}");
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

fn selected_scene_size(host: &mut WidgetHostNative) -> (f32, f32) {
    let id = host.editor_state().selection.anchor.as_str().to_string();
    let node = host
        .layout_scene()
        .active_page()
        .and_then(|page| page.find(&id))
        .expect("selected scene node present");
    (node.bounds.size.x, node.bounds.size.y)
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
fn fill_width_input_seeds_from_resolved_canvas_width() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"frame","id":"root","width":390,"height":710,
               "layout":"vertical","children":[
                 {"type":"frame","id":"fill","width":"fill_container",
                  "height":"fit_content","layout":"vertical","children":[
                   {"type":"rectangle","id":"child","width":180,"height":90}
                 ]}
               ]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("fill"));

    let point = point_for_focus(&host, PropertyFocus::SizeW);
    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state().ui.property_input.text(), "390");
}

#[test]
fn disabling_fill_height_freezes_resolved_height_then_numeric_input_resizes_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"frame","id":"screen","width":390,"height":710,
               "layout":"vertical","gap":0,"children":[
                 {"type":"frame","id":"content","name":"Content Wrapper",
                  "width":"fill_container","height":"fill_container",
                  "layout":"vertical","children":[
                    {"type":"rectangle","id":"body","width":"fill_container","height":100}
                  ]},
                 {"type":"frame","id":"nav","width":"fill_container","height":94}
               ]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("content"));

    assert_eq!(selected_scene_size(&mut host), (390.0, 616.0));

    host.apply_property_action(PropertyPanelAction::ToggleSizeFillHeight);

    let content = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("content"),
    )
    .expect("content present");
    let content_json = serde_json::to_value(content).expect("content serializes");
    assert_eq!(
        content_json["height"],
        serde_json::json!(616.0),
        "turning Fill Height off must freeze the current resolved height"
    );
    assert_eq!(selected_scene_size(&mut host), (390.0, 616.0));

    let point = point_for_focus(&host, PropertyFocus::SizeH);
    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().ui.property_input.text(), "616");
    assert!(host.apply_select_all());
    assert!(host.apply_text('2'));
    assert!(host.apply_text('0'));
    assert!(host.apply_text('0'));
    assert!(host.apply_send());

    assert_eq!(selected_scene_size(&mut host), (390.0, 200.0));
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
