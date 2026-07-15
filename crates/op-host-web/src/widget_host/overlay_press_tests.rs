//! Press + paint coverage for the floating overlays the web host
//! gained in the native-parity pass — shape picker, colour picker,
//! file menu, icon picker. Follows the conventions of
//! `agent_settings_press_tests.rs`: `WidgetHost::new()` + direct
//! `editor_state` access. Paint assertions drive the real
//! composition pass (`paint_editor`) against a recording backend.

use super::WidgetHost;
use op_editor_core::{EditorState, NodeId, PathAnchorMenuState, Tool};
use op_editor_ui::widgets::path_anchor_context_menu::PathAnchorContextMenu;
use op_editor_ui::widgets::{
    DesignMdPanel, LayerPanel, LayerPanelHit, PropertyPanel, ShapeChoice, ShapePicker, Toolbar,
    TopBarHit, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

const W: f32 = 1200.0;
const H: f32 = 800.0;

/// Recording backend — captures fill ops so tests can assert an
/// overlay actually painted inside its rect.
#[derive(Default)]
struct CaptureBackend {
    fills: Vec<Rect>,
    round_fills: Vec<Rect>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, _: Color) {
        self.fills.push(rect);
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
        self.round_fills.push(rect);
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

/// True when some captured fill sits inside `target` (1 px slack) and
/// covers at least half its area — i.e. the overlay's own background
/// painted there, not an unrelated chrome op.
fn painted_inside(backend: &CaptureBackend, target: Rect) -> bool {
    backend
        .fills
        .iter()
        .chain(backend.round_fills.iter())
        .any(|r| {
            r.origin.x >= target.origin.x - 1.0
                && r.origin.y >= target.origin.y - 1.0
                && r.origin.x + r.size.x <= target.origin.x + target.size.x + 1.0
                && r.origin.y + r.size.y <= target.origin.y + target.size.y + 1.0
                && r.size.x * r.size.y >= 0.5 * target.size.x * target.size.y
        })
}

fn seed_layer_doc(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"Header","x":0,"y":0,"width":100,"height":50}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn point_for_layer_row(host: &WidgetHost, id: &str) -> Point2D {
    let panel = LayerPanel::from_editor(&host.editor_state);
    let rect = host.layer_panel_rect(H);
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
fn topmost_design_panel_right_press_does_not_open_layer_context_menu() {
    let mut host = WidgetHost::new();
    seed_layer_doc(&mut host);
    host.editor_state.editor_ui.design_md_panel_open = true;
    host.editor_state.editor_ui.design_md_panel_pos = Some((0.0, TOP_BAR_HEIGHT));
    let point = point_for_layer_row(&host, "n1");

    assert!(host.apply_right_press(point.x, point.y, W, H));

    assert!(host.editor_state.editor_ui.layer_context_menu.is_none());
}

#[test]
fn topmost_design_panel_cursor_move_does_not_hover_toolbar_underneath() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    let toolbar_rect = host.toolbar_rect(W);
    let toolbar = Toolbar::for_editor(&host.editor_state);
    let mut point = None;
    let mut y = toolbar_rect.origin.y;
    while y < toolbar_rect.origin.y + toolbar_rect.size.y && point.is_none() {
        let mut x = toolbar_rect.origin.x;
        while x < toolbar_rect.origin.x + toolbar_rect.size.x {
            let p = Point2D::new(x, y);
            if toolbar.hit_test(toolbar_rect, p).is_some() {
                point = Some(p);
                break;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    let point = point.expect("toolbar hover point");
    host.editor_state.editor_ui.design_md_panel_open = true;
    host.editor_state.editor_ui.design_md_panel_pos =
        Some((toolbar_rect.origin.x - 8.0, toolbar_rect.origin.y - 8.0));
    let panel_rect = host
        .design_md_panel_rect(W, H)
        .expect("design-md panel rect");
    assert!(
        panel_rect.contains(point),
        "fixture should cover the toolbar point"
    );
    host.editor_state.editor_ui.design_md_hover =
        DesignMdPanel::for_editor(&host.editor_state).and_then(|p| p.hover_at(panel_rect, point));

    let _ = host.apply_cursor_move(point.x, point.y);

    assert_eq!(
        host.editor_state.editor_ui.toolbar_hover, None,
        "hover must not pass through the topmost Design-MD panel"
    );
}

#[test]
fn topmost_design_panel_cursor_move_does_not_hover_path_anchor_menu_underneath() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    let point = Point2D::new(160.0, 120.0);
    host.editor_state.ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("n1"),
        anchor_index: 0,
        x: 120.0,
        y: 100.0,
        menu: Default::default(),
    });
    let menu = PathAnchorContextMenu::for_state(
        &host.editor_state,
        host.editor_state
            .ui
            .path_anchor_menu
            .clone()
            .expect("menu open"),
    );
    assert!(
        menu.hovered_row_at(point).is_some(),
        "fixture point should hover the lower path-anchor menu"
    );

    host.editor_state.editor_ui.design_md_panel_open = true;
    host.editor_state.editor_ui.design_md_panel_pos = Some((110.0, 80.0));
    let panel_rect = host
        .design_md_panel_rect(W, H)
        .expect("design-md panel rect");
    assert!(
        panel_rect.contains(point),
        "topmost panel should cover the lower menu point"
    );
    let design_hover = DesignMdPanel::for_editor(&host.editor_state)
        .and_then(|panel| panel.hover_at(panel_rect, point));
    host.editor_state.editor_ui.design_md_hover = design_hover;

    let _ = host.apply_cursor_move(point.x, point.y);

    let menu = host
        .editor_state
        .ui
        .path_anchor_menu
        .as_ref()
        .expect("menu still open");
    assert_eq!(
        menu.menu.hover, None,
        "hover must not pass through the topmost Design-MD panel"
    );
}

#[test]
fn topmost_design_panel_cursor_move_clears_stale_path_anchor_menu_hover() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    let point = Point2D::new(160.0, 120.0);
    host.editor_state.ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("n1"),
        anchor_index: 0,
        x: 120.0,
        y: 100.0,
        menu: Default::default(),
    });
    host.editor_state
        .ui
        .path_anchor_menu
        .as_mut()
        .expect("menu open")
        .menu
        .hover = Some(0);
    host.editor_state.editor_ui.design_md_panel_open = true;
    host.editor_state.editor_ui.design_md_panel_pos = Some((110.0, 80.0));
    let panel_rect = host
        .design_md_panel_rect(W, H)
        .expect("design-md panel rect");
    assert!(panel_rect.contains(point));
    host.editor_state.editor_ui.design_md_hover = DesignMdPanel::for_editor(&host.editor_state)
        .and_then(|panel| panel.hover_at(panel_rect, point));

    let _ = host.apply_cursor_move(point.x, point.y);

    let menu = host
        .editor_state
        .ui
        .path_anchor_menu
        .as_ref()
        .expect("menu still open");
    assert_eq!(
        menu.menu.hover, None,
        "stale lower-menu hover should clear under the topmost panel"
    );
}

#[test]
fn file_menu_cursor_move_clears_stale_layer_hover() {
    let mut host = WidgetHost::new();
    seed_layer_doc(&mut host);
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host.editor_state.editor_ui.file_menu_open = true;
    host.editor_state.editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    let menu_rect = host.file_menu_rect(W).expect("file menu rect");
    let menu =
        op_editor_ui::widgets::file_menu::FileMenu::from_editor_ui(&host.editor_state.editor_ui, 0);
    let x = menu_rect.origin.x + 80.0;
    let mut y = menu_rect.origin.y + 2.0;
    let mut point = None;
    while y < menu_rect.origin.y + menu_rect.size.y {
        let p = Point2D::new(x, y);
        if menu.hovered_at(menu_rect, p).is_some() {
            point = Some(p);
            break;
        }
        y += 2.0;
    }
    let point = point.expect("file menu row point");
    assert!(host.over_dropdown_overlay(point.x, point.y, W, H));

    assert!(host.apply_cursor_move(point.x, point.y));

    assert_eq!(host.editor_state.editor_ui.hovered_layer_id, None);
    assert_eq!(host.editor_state.editor_ui.hovered_page_index, None);
    assert!(
        host.editor_state.editor_ui.file_menu.hover.is_some(),
        "file-menu hover itself should still be active"
    );
}

#[test]
fn cursor_move_over_layer_row_sets_hover_like_native() {
    let mut host = WidgetHost::new();
    seed_layer_doc(&mut host);
    host.last_viewport_w = W;
    host.last_viewport_h = H;

    let layer_rect = host.layer_panel_rect(H);
    let panel = LayerPanel::from_editor(&host.editor_state);
    let mut point = None;
    let mut y = layer_rect.origin.y;
    while y < layer_rect.origin.y + layer_rect.size.y {
        let mut x = layer_rect.origin.x;
        while x < layer_rect.origin.x + layer_rect.size.x {
            let p = Point2D::new(x, y);
            if matches!(panel.hit_test(layer_rect, p), Some(LayerPanelHit::Layer(_))) {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        if point.is_some() {
            break;
        }
        y += 4.0;
    }
    let point = point.expect("layer row point");

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state.editor_ui.hovered_layer_id,
        Some(NodeId::new("n1"))
    );
    assert_eq!(host.editor_state.editor_ui.hovered_page_index, None);
}

#[test]
fn opening_file_menu_clears_stale_layer_hover() {
    let mut host = WidgetHost::new();
    seed_layer_doc(&mut host);
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host.editor_state.editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    host.editor_state.editor_ui.hovered_page_index = Some(0);

    let top_bar_rect = host.top_bar_rect(W);
    let top_bar = host.top_bar();
    let mut file_button = None;
    let mut x = top_bar_rect.origin.x;
    while x < top_bar_rect.origin.x + top_bar_rect.size.x {
        let p = Point2D::new(x, TOP_BAR_HEIGHT / 2.0);
        if top_bar.hit_test(top_bar_rect, p) == Some(TopBarHit::ToggleFileMenu) {
            file_button = Some(p);
            break;
        }
        x += 1.0;
    }
    let point = file_button.expect("top-bar file button point");

    assert!(host.apply_press(point.x, point.y, W, H));

    assert!(host.editor_state.editor_ui.file_menu_open);
    assert_eq!(host.editor_state.editor_ui.hovered_layer_id, None);
    assert_eq!(host.editor_state.editor_ui.hovered_page_index, None);
}

#[test]
fn variables_panel_right_press_is_swallowed_like_native() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.variables_panel_open = true;
    let rect = host
        .variables_panel_rect(W, H)
        .expect("variables panel fits in the test viewport");
    let point = Point2D::new(rect.origin.x + 24.0, rect.origin.y + 24.0);

    assert!(host.apply_right_press(point.x, point.y, W, H));
}

#[test]
fn variables_panel_open_does_not_paint_legacy_modal() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.variables_panel_open = true;
    let mut backend = CaptureBackend::default();

    host.paint_editor(&mut backend, W, H);

    // The retired legacy variables surface was a near full-viewport modal
    // backdrop; the current surface is the floating VariablesPanel
    // (820x480, anchored beside the toolbar). Guard only against a round
    // fill spanning almost the whole viewport — the earlier "> half
    // viewport" heuristic false-tripped once the property panel collapsed
    // (nothing-selected default) widened the canvas so the legitimate
    // floating panel paints unclamped.
    assert!(
        !backend
            .round_fills
            .iter()
            .any(|r| r.size.x >= W * 0.9 && r.size.y >= H * 0.9),
        "variables_panel_open should not paint the legacy full-viewport modal"
    );
    // ...and the floating panel itself must actually paint.
    let panel = host
        .variables_panel_rect(W, H)
        .expect("variables panel fits in the test viewport");
    assert!(
        painted_inside(&backend, panel),
        "the floating VariablesPanel background should paint"
    );
}

#[test]
fn shape_picker_paints_and_row_press_selects_tool() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.shape_picker.open = true;

    // Painted: the open dropdown's panel background lands at its rect.
    let picker_rect = host.shape_picker_rect(W, H);
    let mut backend = CaptureBackend::default();
    host.paint_editor(&mut backend, W, H);
    assert!(
        painted_inside(&backend, picker_rect),
        "shape picker panel should paint at {picker_rect:?}"
    );

    // Interactive: a press on the Ellipse row selects the tool and
    // closes the dropdown. Find the row via the widget's own hit-test
    // so the test doesn't bake in row metrics.
    let picker = ShapePicker::for_editor_ui(&host.editor_state.editor_ui);
    let x = picker_rect.origin.x + picker_rect.size.x / 2.0;
    let mut probe = picker_rect.origin.y + 2.0;
    let mut row_y = None;
    while probe < picker_rect.origin.y + picker_rect.size.y {
        if matches!(
            picker.hit_test(picker_rect, Point2D::new(x, probe)),
            Some(ShapeChoice::Tool(Tool::Ellipse))
        ) {
            row_y = Some(probe);
            break;
        }
        probe += 2.0;
    }
    let row_y = row_y.expect("ellipse row present in the shape picker");

    assert!(host.apply_press(x, row_y, W, H));
    assert_eq!(host.editor_state.tool, Tool::Ellipse);
    assert_eq!(host.editor_state.editor_ui.shape_tool, Tool::Ellipse);
    assert!(!host.editor_state.editor_ui.shape_picker.open);
}

#[test]
fn shape_picker_miss_click_closes_without_tool_change() {
    let mut host = WidgetHost::new();
    let before = host.editor_state.tool;
    host.editor_state.editor_ui.shape_picker.open = true;

    // A press far away from the dropdown dismisses it silently and is
    // swallowed (no marquee / selection change underneath).
    assert!(host.apply_press(900.0, 600.0, W, H));
    assert!(!host.editor_state.editor_ui.shape_picker.open);
    assert_eq!(host.editor_state.tool, before);
}

#[test]
fn color_picker_opens_from_swatch_press_and_paints() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::sample();
    // Select the rectangle node — its section stack (no typography)
    // keeps the Fill swatch above the 800 px fold, so the scan below
    // doesn't depend on panel scrolling.
    host.editor_state
        .set_single_selection(op_editor_core::NodeId::new("n13"));
    host.mark_dirty();

    let panel = PropertyPanel::for_selection(&host.editor_state)
        .expect("sample state selects a node so the right rail is up");
    let pw = host.editor_state.editor_ui.property_panel_width;
    let property_rect = Rect {
        origin: Point2D::new(W - pw, TOP_BAR_HEIGHT),
        size: Point2D::new(pw, H - TOP_BAR_HEIGHT),
    };
    // Locate the fill/stroke swatch through the panel's own action
    // hit-test so the test doesn't bake in section layout math.
    let mut swatch = None;
    let mut y = property_rect.origin.y + 2.0;
    'scan: while y < property_rect.origin.y + property_rect.size.y {
        let mut x = property_rect.origin.x + 2.0;
        while x < property_rect.origin.x + property_rect.size.x {
            if matches!(
                panel.hit_test_action(property_rect, Point2D::new(x, y)),
                Some(op_editor_ui::widgets::PropertyPanelAction::OpenColorPicker(
                    _
                ))
            ) {
                swatch = Some((x, y));
                break 'scan;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let (sx, sy) = swatch.expect("a colour swatch is present for the sample selection");

    assert!(host.apply_press(sx, sy, W, H));
    let state = host
        .editor_state
        .ui
        .color_picker
        .clone()
        .expect("swatch press opens the colour picker");

    // Painted: the floating picker's surface lands at its rect.
    let picker =
        op_editor_ui::widgets::color_picker::ColorPicker::for_state(&host.editor_state, state);
    let picker_rect = picker.rect(W, H);
    let mut backend = CaptureBackend::default();
    host.paint_editor(&mut backend, W, H);
    assert!(
        painted_inside(&backend, picker_rect),
        "colour picker should paint at {picker_rect:?}"
    );

    // An outside press closes the picker (and falls through).
    let _ = host.apply_press(100.0, 700.0, W, H);
    assert!(host.editor_state.ui.color_picker.is_none());
}

#[test]
fn file_menu_paints_and_miss_click_closes() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.file_menu_open = true;

    let menu_rect = host.file_menu_rect(W).expect("open file menu has a rect");
    let mut backend = CaptureBackend::default();
    host.paint_editor(&mut backend, W, H);
    assert!(
        painted_inside(&backend, menu_rect),
        "file menu should paint at {menu_rect:?}"
    );

    // Miss-click far from the dropdown: dismisses without raising a
    // file action, and the press is swallowed.
    assert!(host.apply_press(900.0, 500.0, W, H));
    assert!(!host.editor_state.editor_ui.file_menu_open);
    assert!(host.editor_state.editor_ui.pending_file_action.is_none());
}

#[test]
fn file_menu_row_press_raises_pending_file_action() {
    use op_editor_ui::widgets::file_menu::FileMenu;

    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.file_menu_open = true;

    let menu_rect = host.file_menu_rect(W).expect("open file menu has a rect");
    let menu = FileMenu::from_editor_ui(&host.editor_state.editor_ui, 0);
    let x = menu_rect.origin.x + menu_rect.size.x / 2.0;
    let mut probe = menu_rect.origin.y + 2.0;
    let mut row = None;
    while probe < menu_rect.origin.y + menu_rect.size.y {
        if menu.hit_test(menu_rect, Point2D::new(x, probe)).is_some() {
            row = Some(probe);
            break;
        }
        probe += 2.0;
    }
    let row_y = row.expect("file menu has at least one actionable row");

    assert!(host.apply_press(x, row_y, W, H));
    assert!(!host.editor_state.editor_ui.file_menu_open);
    assert!(
        host.editor_state.editor_ui.pending_file_action.is_some(),
        "a row press raises the host-level pending file action"
    );
}

#[test]
fn shape_picker_icon_row_opens_icon_picker_panel() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.shape_picker.open = true;

    let picker_rect = host.shape_picker_rect(W, H);
    let picker = ShapePicker::for_editor_ui(&host.editor_state.editor_ui);
    let x = picker_rect.origin.x + picker_rect.size.x / 2.0;
    let mut probe = picker_rect.origin.y + 2.0;
    let mut row_y = None;
    while probe < picker_rect.origin.y + picker_rect.size.y {
        if matches!(
            picker.hit_test(picker_rect, Point2D::new(x, probe)),
            Some(ShapeChoice::OpenIconPicker)
        ) {
            row_y = Some(probe);
            break;
        }
        probe += 2.0;
    }
    let row_y = row_y.expect("icon row present in the shape picker");

    assert!(host.apply_press(x, row_y, W, H));
    assert!(host.editor_state.editor_ui.icon_picker.open);
    assert!(!host.editor_state.editor_ui.shape_picker.open);

    // The icon-picker panel paints at its centred rect.
    let panel_rect = host
        .icon_picker_panel_rect(W, H)
        .expect("open icon picker has a rect");
    let mut backend = CaptureBackend::default();
    host.paint_editor(&mut backend, W, H);
    assert!(
        painted_inside(&backend, panel_rect),
        "icon picker panel should paint at {panel_rect:?}"
    );
}

#[test]
fn icon_picker_load_more_press_sets_and_release_clears_pressed() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.icon_picker.open = true;
    host.editor_state.editor_ui.icon_picker_search = "unlikely-remote-only".to_string();

    let panel_rect = host.icon_picker_panel_rect(W, H).expect("icon picker rect");
    let panel = op_editor_ui::widgets::IconPickerPanel::for_editor(&host.editor_state)
        .expect("open icon picker");
    let mut point = None;
    let mut y = panel_rect.origin.y;
    while y <= panel_rect.origin.y + panel_rect.size.y && point.is_none() {
        let mut x = panel_rect.origin.x;
        while x <= panel_rect.origin.x + panel_rect.size.x {
            let p = Point2D::new(x, y);
            if matches!(
                panel.hit_test(panel_rect, p),
                Some(op_editor_ui::widgets::IconPickerHit::LoadMore)
            ) {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("load more row is hittable");

    assert!(host.apply_press(point.x, point.y, W, H));
    assert_eq!(
        host.editor_state.editor_ui.icon_picker.pressed,
        Some(op_editor_ui::widgets::icon_picker_panel::ICON_PICKER_LOAD_MORE_HOVER)
    );

    assert!(host.apply_release_with_viewport(W, H));
    assert_eq!(host.editor_state.editor_ui.icon_picker.pressed, None);
}
