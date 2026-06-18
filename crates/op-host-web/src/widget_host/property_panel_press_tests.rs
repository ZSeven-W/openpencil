use super::WidgetHost;
use op_editor_core::codegen::{CodegenHover, CodegenPhase};
use op_editor_core::PropertyTab;
use op_editor_core::{ButtonPressTarget, NodeId};
use op_editor_ui::widgets::property_panel_action::CodegenAction;
use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn seed(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn property_rect(host: &WidgetHost) -> Rect {
    let width = host.editor_state.editor_ui.property_panel_width;
    Rect {
        origin: Point2D::new(VIEWPORT_W - width, TOP_BAR_HEIGHT),
        size: Point2D::new(width, VIEWPORT_H - TOP_BAR_HEIGHT),
    }
}

fn point_for_action(host: &WidgetHost, want: impl Fn(&PropertyPanelAction) -> bool) -> Point2D {
    let panel = PropertyPanel::for_selection(&host.editor_state).expect("property panel");
    let rect = property_rect(host);
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

#[test]
fn property_panel_action_press_sets_and_release_clears_pressed_button() {
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

    let point = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleSizeFillWidth)
    });
    let panel = PropertyPanel::for_selection(&host.editor_state).expect("property panel");
    let rect = property_rect(&host);
    let expected_index = panel
        .action_hover_index(rect, point)
        .expect("action maps to hover index");

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::PropertyPanel(expected_index))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn codegen_action_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.property_tab = PropertyTab::Code;
    host.editor_state.codegen.phase = CodegenPhase::Complete;
    host.editor_state.codegen.code = "fn main() {\n    println!(\"hi\");\n}\n".into();

    let point = point_for_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::Codegen(CodegenAction::Regenerate)
        )
    });

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::Codegen(CodegenHover::Regenerate))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn pick_fill_image_queues_web_file_picker_like_native() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.image_fill_popover_open = true;

    host.apply_property_action(PropertyPanelAction::PickFillImage);

    assert_eq!(
        host.editor_state.editor_ui.pending_file_action,
        Some(op_editor_core::editor_ui_state::FileAction::PickFillImage),
    );
    assert!(
        host.editor_state.editor_ui.image_fill_popover_open,
        "the image popover must stay open so Fill/Fit/Crop/Tile remain selectable",
    );
}
