//! Regression tests for the colour-variable picker overlay.
//!
//! The picker used to be an inline block whose rows were emitted by the
//! section walker, so a document with many `--color-*` variables pushed
//! the inspector open until it ran off the bottom of the screen, and a
//! long variable name painted straight through its own hex value. These
//! tests pin both fixes: the popup is an overlay (it must never move any
//! other panel control), and its two text columns can never overlap.

use super::press_flow::PropertyOverlayPress;
use super::property_panel::{PropertyPanel, PropertyPanelAction};
use super::property_panel_color_variables::{
    color_variable_picker_layout, row_hex_rect, row_name_budget, row_name_rect,
    ColorVariablePickerLayout, ColorVariableRow, COLOR_VARIABLE_MENU_MAX_H,
};
use super::property_panel_inputs::TAB_HEIGHT;
use super::property_panel_layout::COLOR_VARIABLE_MENU_ROW_H;
use super::property_panel_sections as sections;
use super::property_panel_test_support::{state_from, visible_for};
use crate::widgets::{PaintCx, Widget};
use crate::RenderBackend;
use crate::{Color, ImageAdjustments, ImageBlendMode, ImageDrawMode, Point2D, Rect, TextLayout};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::{ColorTarget, EditorState, NodeId};

const RECT_DOC: &str = r##"{ "version": "1.0.0", "children": [
      {"type":"rectangle","id":"rect","name":"Rect",
       "x":40,"y":40,"width":160,"height":100,
       "fill":[{"type":"solid","color":"#ffffff"}],
       "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#374151"}]}}
]}"##;

/// A rectangle selection carrying `count` colour variables.
fn state_with_variables(count: usize) -> EditorState {
    let mut state = state_from(RECT_DOC);
    state.set_single_selection(NodeId::new("rect"));
    for i in 0..count {
        assert!(state.create_variable(
            &format!("--border-subtle-{i:02}"),
            VariableKind::Color,
            VariableScalar::Str("#DBD8CB".into()),
        ));
    }
    state
}

fn panel_rect() -> Rect {
    Rect {
        origin: Point2D::new(1000.0, 48.0),
        size: Point2D::new(280.0, 700.0),
    }
}

fn open_layout(state: &EditorState, rect: Rect) -> ColorVariablePickerLayout {
    PropertyPanel::for_selection(state)
        .expect("rectangle panel")
        .color_variable_picker_layout(rect)
        .expect("open picker should lay out")
}

fn row_center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// Every panel control rect, keyed so the open / closed runs can be
/// compared field by field.
fn control_rects(state: &EditorState, rect: Rect) -> Vec<(String, Rect)> {
    let panel = PropertyPanel::for_selection(state).expect("rectangle panel");
    let visible = visible_for(&panel);
    let mut out: Vec<(String, Rect)> = sections::action_button_rects_with_fill_picker(
        rect,
        visible,
        &panel.snapshot.effects,
        &panel.snapshot.fills,
        &panel.snapshot.interactions,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
    )
    .into_iter()
    .map(|(action, r)| (format!("{action:?}"), r))
    .collect();
    out.extend(
        crate::widgets::property_panel_input_layout::editable_input_rects(
            rect,
            visible,
            &panel.snapshot.fills,
            &panel.snapshot.effects,
        )
        .into_iter()
        .map(|(focus, r)| (format!("{focus:?}"), r)),
    );
    out
}

/// The "撑开面板" regression: opening the picker is a pure overlay, so
/// no other control moves and the inspector's scroll height is unchanged.
#[test]
fn picker_overlay_never_moves_other_panel_controls() {
    let mut state = state_with_variables(40);
    let rect = panel_rect();

    let closed = control_rects(&state, rect);
    let closed_height = PropertyPanel::for_selection(&state)
        .expect("rectangle panel")
        .content_height(rect);

    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let opened = control_rects(&state, rect);
    let opened_height = PropertyPanel::for_selection(&state)
        .expect("rectangle panel")
        .content_height(rect);

    assert_eq!(
        closed.len(),
        opened.len(),
        "opening the picker must not add or drop panel controls"
    );
    for ((closed_key, closed_rect), (opened_key, opened_rect)) in closed.iter().zip(opened.iter()) {
        assert_eq!(closed_key, opened_key, "control order changed");
        assert_eq!(
            (closed_rect.origin.x, closed_rect.origin.y),
            (opened_rect.origin.x, opened_rect.origin.y),
            "{closed_key} moved when the colour-variable picker opened"
        );
        assert_eq!(
            (closed_rect.size.x, closed_rect.size.y),
            (opened_rect.size.x, opened_rect.size.y),
            "{closed_key} resized when the colour-variable picker opened"
        );
    }
    assert_eq!(
        closed_height, opened_height,
        "the picker must not grow the inspector's scrollable content"
    );
}

/// A long variable list is capped and scrolls inside the popup instead
/// of running past the bottom of the rail.
#[test]
fn long_variable_list_caps_height_and_scrolls() {
    let mut state = state_with_variables(40);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = panel_rect();
    let layout = open_layout(&state, rect);

    assert!(
        layout.popup.size.y <= COLOR_VARIABLE_MENU_MAX_H,
        "popup height {} exceeded the {COLOR_VARIABLE_MENU_MAX_H}px cap",
        layout.popup.size.y
    );
    assert!(
        layout.content_height > layout.popup.size.y,
        "40 variables should overflow the capped popup"
    );
    assert!(
        layout.max_scroll > 0.0,
        "an overflowing list must expose a scroll range"
    );
}

/// The popup is clamped into the visible rail — never off its right
/// edge and never past the bottom of the window.
#[test]
fn popup_stays_inside_the_visible_rail() {
    let rect = panel_rect();
    let rail_top = rect.origin.y + TAB_HEIGHT;
    let rail_bottom = rect.origin.y + rect.size.y;
    for count in [1usize, 6, 40] {
        let mut state = state_with_variables(count);
        state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
        let layout = open_layout(&state, rect);
        assert!(
            layout.popup.origin.x >= rect.origin.x,
            "{count} variables: popup left {} escaped the rail",
            layout.popup.origin.x
        );
        assert!(
            layout.popup.origin.x + layout.popup.size.x <= rect.origin.x + rect.size.x,
            "{count} variables: popup right escaped the rail"
        );
        assert!(
            layout.popup.origin.y >= rail_top,
            "{count} variables: popup overlapped the pinned tab strip"
        );
        assert!(
            layout.popup.origin.y + layout.popup.size.y <= rail_bottom,
            "{count} variables: popup bottom {} ran past the rail bottom {rail_bottom}",
            layout.popup.origin.y + layout.popup.size.y
        );
    }
}

/// The stroke section's `{}` gets the same overlay treatment as fill.
#[test]
fn stroke_picker_is_an_overlay_too() {
    let mut state = state_with_variables(40);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Stroke);
    let rect = panel_rect();
    let layout = open_layout(&state, rect);
    assert!(layout.popup.size.y <= COLOR_VARIABLE_MENU_MAX_H);
    assert!(layout.max_scroll > 0.0);
    assert!(layout.popup.origin.y + layout.popup.size.y <= rect.origin.y + rect.size.y);

    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    assert!(
        matches!(
            panel.color_variable_picker_action_at(rect, row_center(layout.rows[0].1)),
            Some(PropertyPanelAction::BindColorVariable {
                target: ColorTarget::Stroke,
                index: 0,
            })
        ),
        "stroke rows must bind against the stroke target"
    );
}

/// The name and hex columns are laid out as disjoint slots for every
/// plausible value width, so they can never paint on top of each other.
#[test]
fn row_name_and_hex_columns_never_overlap() {
    let row = Rect {
        origin: Point2D::new(1000.0, 200.0),
        size: Point2D::new(210.0, COLOR_VARIABLE_MENU_ROW_H),
    };
    for hex_w in [0.0_f32, 20.0, 46.2, 80.0, 150.0, 400.0] {
        let hex = row_hex_rect(row, hex_w);
        let budget = row_name_budget(row, hex_w);
        let name = row_name_rect(row, budget);
        assert!(
            name.origin.x + name.size.x <= hex.origin.x,
            "hex width {hex_w}: name column ran into the hex column"
        );
        assert!(
            budget >= 0.0,
            "hex width {hex_w}: name budget must never go negative"
        );
    }
}

/// Records what text was drawn where, so the paint pass can be checked
/// against the geometry the layout promised.
#[derive(Default)]
struct TextProbe {
    texts: Vec<(String, Point2D)>,
}

impl crate::RenderBackend for TextProbe {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, at: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push((run.content.clone(), at));
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn image_decoded(&mut self, _: u64, _: &[u8], _: u32) -> bool {
        true
    }
    fn image_resident(&mut self, _: u64) -> bool {
        true
    }
    fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
    fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
    #[allow(clippy::too_many_arguments)]
    fn draw_image_with_options_transform_blend_and_tile_scale(
        &mut self,
        _: Rect,
        _: u64,
        _: &[u8],
        _: ImageDrawMode,
        _: ImageAdjustments,
        _: f32,
        _: f32,
        _: Option<[f32; 6]>,
        _: ImageBlendMode,
        _: Option<[f32; 2]>,
        _: f32,
    ) {
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

/// A name too long for its slot is ellipsized into the budget rather
/// than painted over the hex value.
#[test]
fn painted_name_is_ellipsized_into_its_budget() {
    let mut state = state_from(RECT_DOC);
    state.set_single_selection(NodeId::new("rect"));
    assert!(state.create_variable(
        "--border-subtle-on-elevated-surface-extra-long",
        VariableKind::Color,
        VariableScalar::Str("#DBD8CB".into()),
    ));
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = panel_rect();
    let layout = open_layout(&state, rect);
    let row = layout.rows[0].1;

    let mut backend = TextProbe::default();
    {
        let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }
    let hex_w = backend.measure_text_family("#DBD8CB", 12.0, "ui-monospace");
    let budget = row_name_budget(row, hex_w);

    let (name, name_at) = backend
        .texts
        .iter()
        .find(|(text, _)| text.starts_with("----border"))
        .cloned()
        .expect("the variable name should be painted");
    let (_, hex_at) = backend
        .texts
        .iter()
        .find(|(text, _)| text == "#DBD8CB")
        .cloned()
        .expect("the resolved hex should be painted");

    assert!(
        name.ends_with('…'),
        "an over-long variable name should be ellipsized, got {name:?}"
    );
    let painted_w = backend.measure_text_family(&name, 12.0, "ui-monospace");
    assert!(
        painted_w <= budget,
        "painted name width {painted_w} exceeded its {budget} budget"
    );
    assert!(
        name_at.x + painted_w <= hex_at.x,
        "painted name overlapped the hex column"
    );
}

/// Scrolling shifts the rows by exactly the offset, and rows pushed out
/// of the viewport stop taking clicks.
#[test]
fn scrolling_shifts_rows_and_hit_test_follows() {
    let mut state = state_with_variables(40);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = panel_rect();

    let unscrolled = open_layout(&state, rect);
    let scroll = COLOR_VARIABLE_MENU_ROW_H * 3.0;
    state.editor_ui.property_color_variable_picker_scroll.offset = scroll;
    let scrolled = open_layout(&state, rect);

    assert_eq!(
        scrolled.popup, unscrolled.popup,
        "the popup itself is fixed"
    );
    for ((_, before), (_, after)) in unscrolled.rows.iter().zip(scrolled.rows.iter()) {
        assert!(
            (before.origin.y - after.origin.y - scroll).abs() < 0.01,
            "rows must shift up by exactly the scroll offset"
        );
    }

    // The first row now sits above the viewport — its own rect is no
    // longer clickable.
    assert!(
        scrolled.rows[0].1.origin.y < scrolled.viewport.origin.y,
        "three rows of scroll should push the first row out of view"
    );
    assert_eq!(
        scrolled.row_at(row_center(scrolled.rows[0].1)),
        None,
        "a row scrolled out of view must not be clickable"
    );
    // Whatever sits at the top of the viewport is what gets bound.
    let first_visible = scrolled
        .rows
        .iter()
        .find(|(_, r)| r.origin.y >= scrolled.viewport.origin.y)
        .expect("some row is still visible");
    assert_eq!(
        scrolled.row_at(row_center(first_visible.1)),
        Some(first_visible.0),
        "hit-test must agree with the scrolled row geometry"
    );
    assert!(
        matches!(first_visible.0, ColorVariableRow::Variable(index) if index >= 3),
        "scrolling three rows should reveal later variables, got {:?}",
        first_visible.0
    );
}

/// A press on the popup chrome is inside the picker (the host swallows
/// it); a press outside is not.
#[test]
fn contains_covers_the_whole_popup_only() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = panel_rect();
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    let layout = panel
        .color_variable_picker_layout(rect)
        .expect("open picker should lay out");

    assert!(panel.color_variable_picker_contains(rect, row_center(layout.popup)));
    assert!(!panel.color_variable_picker_contains(
        rect,
        Point2D::new(layout.popup.origin.x - 20.0, row_center(layout.popup).y),
    ));
    assert!(!panel.color_variable_picker_contains(
        rect,
        Point2D::new(
            row_center(layout.popup).x,
            layout.popup.origin.y + layout.popup.size.y + 20.0,
        ),
    ));
}

/// An empty variable set with nothing bound has no popup at all.
#[test]
fn empty_variable_set_has_no_popup() {
    let mut state = state_from(RECT_DOC);
    state.set_single_selection(NodeId::new("rect"));
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    assert!(panel.color_variable_picker_layout(panel_rect()).is_none());
    assert!(color_variable_picker_layout(
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(24.0, 28.0),
        },
        panel_rect(),
        0,
        false,
        0.0,
    )
    .is_none());
}

// ─── Press + hover routing ─────────────────────────────────────────────
//
// The popup's rows used to be unreachable: the host routed its presses
// through the panel's ordinary control walk, which knows nothing about
// the overlay's scrolled row rects, so every row click fell into the
// `contains` swallow and was eaten. Hover had no state at all, so the
// controls painted underneath lit up through the popup.

const VIEWPORT_W: f32 = 1280.0;
const VIEWPORT_H: f32 = 740.0;

/// The rail rect the shared press / hover flows derive themselves.
fn press_panel_rect(state: &EditorState) -> Rect {
    crate::widgets::press_flow::property_panel_rect(state, VIEWPORT_W, VIEWPORT_H)
}

fn press(state: &mut EditorState, point: Point2D) -> PropertyOverlayPress {
    crate::widgets::press_flow::press_color_variable_picker(state, VIEWPORT_W, VIEWPORT_H, point)
}

/// Run the picked action through the shared property dispatch, the way
/// both hosts' `finish_property_overlay_press` does.
fn dispatch(state: &mut EditorState, action: &PropertyPanelAction) {
    use crate::widgets::property_panel_dispatch as dispatch;
    let mut image_adjustment_drag = None;
    let mut effect_radius_drag = None;
    let _ = dispatch::apply_property_action(
        state,
        action,
        dispatch::PropertyActionContext {
            now_ms: 0,
            resolved_sizing_fallback: None,
            image_adjustment_drag: &mut image_adjustment_drag,
            effect_radius_drag: &mut effect_radius_drag,
        },
    );
}

fn hover(state: &mut EditorState, rect: Rect, point: Point2D) -> (bool, bool) {
    let panel = PropertyPanel::for_selection(state).expect("rectangle panel");
    crate::widgets::cursor_hover_flow::color_variable_picker_hover(state, Some(&panel), rect, point)
}

/// A press on a variable row binds it and the popup closes behind the
/// selection.
#[test]
fn press_on_variable_row_binds_and_closes() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);
    let (row, row_rect) = layout.rows[2];
    assert_eq!(row, ColorVariableRow::Variable(2));

    let action = match press(&mut state, row_center(row_rect)) {
        PropertyOverlayPress::Action(action) => action,
        other => panic!("row press must dispatch an action, got {other:?}"),
    };
    assert_eq!(
        action,
        PropertyPanelAction::BindColorVariable {
            target: ColorTarget::Fill,
            index: 2,
        }
    );

    dispatch(&mut state, &action);
    assert_eq!(state.editor_ui.property_color_variable_picker_open, None);
    assert_eq!(
        state.editor_ui.property_color_variable_picker_scroll.offset, 0.0,
        "closing must reset the list scroll"
    );
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    assert_eq!(
        panel.fill_variable_ref.as_deref(),
        Some("--border-subtle-02"),
        "the pressed row is the variable that got bound"
    );
}

/// Scrolling the list moves which variable a press at a given screen
/// point binds — the row rects carry the offset and the press must read
/// the same geometry paint does.
#[test]
fn press_after_scroll_binds_the_scrolled_row() {
    let mut state = state_with_variables(40);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    state.editor_ui.property_color_variable_picker_scroll.offset = COLOR_VARIABLE_MENU_ROW_H * 3.0;
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);
    let (row, row_rect) = *layout
        .rows
        .iter()
        .find(|(_, r)| r.origin.y >= layout.viewport.origin.y)
        .expect("some row is still visible");
    assert!(
        matches!(row, ColorVariableRow::Variable(index) if index >= 3),
        "three rows of scroll should reveal a later variable, got {row:?}"
    );

    match press(&mut state, row_center(row_rect)) {
        PropertyOverlayPress::Action(PropertyPanelAction::BindColorVariable { target, index }) => {
            assert_eq!(target, ColorTarget::Fill);
            assert_eq!(ColorVariableRow::Variable(index), row);
        }
        other => panic!("scrolled row press must bind that row, got {other:?}"),
    }
}

/// The leading unbind row resolves the binding back to a concrete
/// colour and closes the popup.
#[test]
fn press_on_unbind_row_unbinds() {
    let mut state = state_with_variables(4);
    assert!(state.bind_selected_color_variable(ColorTarget::Fill, "--border-subtle-01"));
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);
    assert_eq!(layout.rows[0].0, ColorVariableRow::Unbind);

    let action = match press(&mut state, row_center(layout.rows[0].1)) {
        PropertyOverlayPress::Action(action) => action,
        other => panic!("unbind row press must dispatch an action, got {other:?}"),
    };
    assert_eq!(
        action,
        PropertyPanelAction::UnbindColorVariable(ColorTarget::Fill)
    );

    dispatch(&mut state, &action);
    assert_eq!(state.editor_ui.property_color_variable_picker_open, None);
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    assert_eq!(panel.fill_variable_ref, None);
}

/// A press on the popup's own padding is swallowed — the popup stays
/// open and nothing is bound.
#[test]
fn press_on_popup_padding_is_swallowed() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);
    let padding = Point2D::new(row_center(layout.popup).x, layout.popup.origin.y + 1.0);
    assert!(
        layout.row_at(padding).is_none(),
        "fixture point must be popup chrome, not a row"
    );

    assert_eq!(press(&mut state, padding), PropertyOverlayPress::Swallow);
    assert_eq!(
        state.editor_ui.property_color_variable_picker_open,
        Some(ColorTarget::Fill),
        "chrome presses keep the popup open"
    );
}

/// A press outside the popup dismisses it.
#[test]
fn press_outside_dismisses_and_clears() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    state.editor_ui.property_color_variable_picker_hover = Some(1);
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);
    let outside = Point2D::new(layout.popup.origin.x - 20.0, row_center(layout.popup).y);

    assert_eq!(press(&mut state, outside), PropertyOverlayPress::Dismissed);
    assert_eq!(state.editor_ui.property_color_variable_picker_open, None);
    assert_eq!(state.editor_ui.property_color_variable_picker_hover, None);
}

/// A cursor over a row lights that row and reports that the popup owns
/// the point, so the host consumes the move instead of letting the rail
/// underneath hover.
#[test]
fn hover_over_row_sets_row_hover_and_owns_point() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);

    let (over_popup, changed) = hover(&mut state, rect, row_center(layout.rows[1].1));
    assert!(over_popup, "a row press point is on the popup");
    assert!(changed);
    assert_eq!(
        state.editor_ui.property_color_variable_picker_hover,
        Some(1)
    );

    // Re-entering the same row is not a repaint.
    let (over_popup, changed) = hover(&mut state, rect, row_center(layout.rows[1].1));
    assert!(over_popup);
    assert!(!changed);
}

/// Moving off the popup drops the row hover and hands the move back to
/// the surfaces below.
#[test]
fn hover_outside_popup_clears_row_hover() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    state.editor_ui.property_color_variable_picker_hover = Some(1);
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);
    let outside = Point2D::new(layout.popup.origin.x - 20.0, row_center(layout.popup).y);

    let (over_popup, changed) = hover(&mut state, rect, outside);
    assert!(!over_popup);
    assert!(changed);
    assert_eq!(state.editor_ui.property_color_variable_picker_hover, None);
}

/// Popup chrome owns the point without lighting any row.
#[test]
fn hover_over_popup_padding_owns_point_without_a_row() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    state.editor_ui.property_color_variable_picker_hover = Some(1);
    let rect = press_panel_rect(&state);
    let layout = open_layout(&state, rect);
    let padding = Point2D::new(row_center(layout.popup).x, layout.popup.origin.y + 1.0);

    let (over_popup, changed) = hover(&mut state, rect, padding);
    assert!(over_popup);
    assert!(changed);
    assert_eq!(state.editor_ui.property_color_variable_picker_hover, None);
}

/// A closed popup is inert for hover, and closing drops the retained
/// row hover with the scroll.
#[test]
fn closing_the_popup_clears_hover_and_scroll() {
    let mut state = state_with_variables(6);
    state.editor_ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    state.editor_ui.property_color_variable_picker_hover = Some(2);
    state.editor_ui.property_color_variable_picker_scroll.offset = COLOR_VARIABLE_MENU_ROW_H;

    assert!(state.editor_ui.close_color_variable_picker());
    assert_eq!(state.editor_ui.property_color_variable_picker_hover, None);
    assert_eq!(
        state.editor_ui.property_color_variable_picker_scroll.offset,
        0.0
    );

    let rect = press_panel_rect(&state);
    let (over_popup, changed) = hover(&mut state, rect, Point2D::new(1100.0, 300.0));
    assert!(!over_popup);
    assert!(!changed, "a closed popup never reports a hover change");
}
