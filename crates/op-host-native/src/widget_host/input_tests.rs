//! `#[cfg(test)]` companion to the input modules — extracted here so
//! the input module stays under the 800-line ceiling.
//!
//! `EditorState` is the host's source of truth, so the fixtures seed
//! `host.editor_state` from canonical-schema JSON and assert against
//! `editor_state` + the derived `LayoutScene` render scene.

use super::{CursorHint, WidgetHostNative};
use op_editor_core::ui_draft::PropertyFocus;
use op_editor_core::PenNodeExt;
use op_editor_core::{own_bounds, NodeId};
use op_editor_ui::widgets::{PropertyPanel, TopBar, TopBarHit, TOP_BAR_HEIGHT};

mod chat_panel;
mod chrome_overlays;
mod codegen_panel;
mod property_input;
mod scene_hover;
mod search_overlays;
mod selection_drag;

/// Seed a host's `editor_state` from a canonical `.op` JSON snippet.
fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn point_inside_property_panel_without_target(host: &WidgetHostNative) -> op_editor_ui::Point2D {
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(host.last_viewport_w, host.last_viewport_h);
    let mut y = rect.origin.y + rect.size.y - 12.0;
    while y > rect.origin.y {
        let mut x = rect.origin.x + 12.0;
        while x < rect.origin.x + rect.size.x - 12.0 {
            let point = op_editor_ui::Point2D::new(x, y);
            let no_action = panel.hit_test_action(rect, point).is_none();
            let no_input = panel.hit_test(rect, point).is_none();
            let no_tab = panel.tab_hover_at(rect, point).is_none();
            let no_fill_type = panel.fill_type_picker_row_at(rect, point).is_none();
            if no_action && no_input && no_tab && no_fill_type {
                return point;
            }
            x += 8.0;
        }
        y -= 8.0;
    }
    panic!("no empty property-panel point found");
}

/// Three top-level rect nodes at the given `(x, y, w, h)` boxes.
fn three_rects(boxes: [(f64, f64, f64, f64); 3], ids: [&str; 3]) -> String {
    let node = |id: &str, b: (f64, f64, f64, f64)| {
        format!(
            r#"{{"type":"rectangle","id":"{id}","name":"{id}",
               "x":{},"y":{},"width":{},"height":{}}}"#,
            b.0, b.1, b.2, b.3
        )
    };
    format!(
        r#"{{"version":"1.0.0","children":[{},{},{}]}}"#,
        node(ids[0], boxes[0]),
        node(ids[1], boxes[1]),
        node(ids[2], boxes[2]),
    )
}

fn toolbar_action_point_for_test(
    host: &WidgetHostNative,
    action: op_editor_ui::widgets::ToolbarAction,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32) {
    let toolbar = op_editor_ui::widgets::Toolbar::for_editor(host.editor_state());
    let rect = host.toolbar_rect(viewport_w, viewport_h);
    let min_x = rect.origin.x.floor() as i32;
    let max_x = (rect.origin.x + rect.size.x).ceil() as i32;
    let min_y = rect.origin.y.floor() as i32;
    let max_y = (rect.origin.y + rect.size.y).ceil() as i32;
    let mut hits = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if toolbar.hit_test(rect, op_editor_ui::Point2D::new(x as f32, y as f32))
                == Some(op_editor_ui::widgets::ToolbarHit::Action(action))
            {
                hits.push((x as f32, y as f32));
            }
        }
    }
    let (sum_x, sum_y) = hits
        .iter()
        .fold((0.0, 0.0), |(sx, sy), (x, y)| (sx + *x, sy + *y));
    let count = hits.len().max(1) as f32;
    assert!(
        !hits.is_empty(),
        "toolbar action {action:?} should expose a hit target"
    );
    (sum_x / count, sum_y / count)
}
