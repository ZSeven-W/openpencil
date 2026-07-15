use super::WidgetHost;
use op_editor_core::ui_draft::LayerContextTarget;
use op_editor_core::NodeId;
use op_editor_ui::widgets::{LayerPanel, LayerPanelHit};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn seed(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"Header","x":0,"y":0,"width":100,"height":50},
            {"type":"rectangle","id":"n2","name":"Card","x":120,"y":0,"width":100,"height":50}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn point_for_layer_row(host: &WidgetHost, id: &str) -> Point2D {
    let panel = LayerPanel::from_editor(&host.editor_state);
    let rect = host.layer_panel_rect(VIEWPORT_H);
    let regions = panel.regions(rect);
    let mut y = regions.layers_rows_top + 2.0;
    while y < regions.layers_rows_top + regions.layers_view_h {
        let point = Point2D::new(rect.origin.x + 48.0, y);
        if matches!(
            panel.hit_test(rect, point),
            Some(LayerPanelHit::Layer(node_id)) if node_id == NodeId::new(id)
        ) {
            return point;
        }
        y += 2.0;
    }
    panic!("no layer row point found for {id}");
}

#[test]
fn layer_row_double_click_starts_inline_rename_like_native() {
    let mut host = WidgetHost::new();
    seed(&mut host);
    host.set_now_ms(1_000);

    let point = point_for_layer_row(&host, "n1");
    assert!(host.apply_click(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state.ui.layer_rename.is_none());

    host.set_now_ms(1_250);
    assert!(host.apply_click(point.x, point.y, VIEWPORT_W, VIEWPORT_H));

    let rename = host
        .editor_state
        .ui
        .layer_rename
        .as_ref()
        .expect("second click within 400 ms should start layer rename");
    assert_eq!(rename.target, LayerContextTarget::Layer(NodeId::new("n1")));
    assert_eq!(rename.input.text(), "Header");
}
