//! Press + paint coverage for the floating overlays the web host
//! gained in the native-parity pass — shape picker, colour picker,
//! file menu, icon picker. Follows the conventions of
//! `agent_settings_press_tests.rs`: `WidgetHost::new()` + direct
//! `editor_state` access. Paint assertions drive the real
//! composition pass (`paint_editor`) against a recording backend.

use super::WidgetHost;
use op_editor_core::{EditorState, Tool};
use op_editor_ui::widgets::{PropertyPanel, ShapeChoice, ShapePicker, TOP_BAR_HEIGHT};
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

#[test]
fn shape_picker_paints_and_row_press_selects_tool() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.shape_picker_open = true;

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
    assert!(!host.editor_state.editor_ui.shape_picker_open);
}

#[test]
fn shape_picker_miss_click_closes_without_tool_change() {
    let mut host = WidgetHost::new();
    let before = host.editor_state.tool;
    host.editor_state.editor_ui.shape_picker_open = true;

    // A press far away from the dropdown dismisses it silently and is
    // swallowed (no marquee / selection change underneath).
    assert!(host.apply_press(900.0, 600.0, W, H));
    assert!(!host.editor_state.editor_ui.shape_picker_open);
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
    host.editor_state.editor_ui.shape_picker_open = true;

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
    assert!(host.editor_state.editor_ui.icon_picker_open);
    assert!(!host.editor_state.editor_ui.shape_picker_open);

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
