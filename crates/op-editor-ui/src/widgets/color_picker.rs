//! Floating HSV colour picker — opens from a Fill / Stroke swatch click in
//! the PropertyPanel. The rendering, layout, hit-test and HSV↔RGB math live
//! in jian `components::color_picker`; this wrapper owns the
//! property-panel-relative anchor, the `EditorState` plumbing, and the
//! drag-dispatch glue.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::Icon;
use crate::widgets::{PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use jian_widgets::components::color_picker::ColorPicker as JianColorPicker;
use op_editor_core::ui_draft::{ColorPickerDrag, ColorPickerState, ColorTarget};
use op_editor_core::EditorState;

// Re-export jian's public surface so hosts (op-host-native / op-host-web) and
// sibling widgets keep importing it from this module unchanged.
pub use jian_widgets::components::color_picker::{
    hsv_to_rgb, rgb_to_hsv, ColorPickerHit, PICKER_HEIGHT, PICKER_WIDTH,
};

pub struct ColorPicker {
    pub id: WidgetId,
    pub state: ColorPickerState,
    pub theme: Theme,
    /// Right-rail width — drives the picker's horizontal anchor.
    property_panel_width: f32,
    /// Clock for the hex field's caret blink.
    now_ms: u64,
}

impl ColorPicker {
    pub fn for_state(state: &EditorState, picker: ColorPickerState) -> Self {
        Self::for_state_at(state, picker, 0)
    }

    pub fn for_state_at(state: &EditorState, picker: ColorPickerState, now_ms: u64) -> Self {
        Self {
            id: WidgetId::new(5100),
            state: picker,
            theme: theme_for(&state.editor_ui),
            property_panel_width: state.editor_ui.property_panel_width,
            now_ms,
        }
    }

    /// Resolve the picker's anchor (top-left) in viewport coords — pinned near
    /// an explicit click point when available, otherwise to the right rail near
    /// the Fill section. Hosts use this for paint AND hit-test so they stay in
    /// sync. (Domain-specific to OP's right rail, so it stays here.)
    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let x = self
            .state
            .anchor_x
            .map(|anchor_x| anchored_x(anchor_x, viewport_w))
            .unwrap_or_else(|| {
                let right_rail_left = viewport_w - self.property_panel_width;
                (right_rail_left - PICKER_WIDTH - 8.0).max(8.0)
            });
        let mut y = self.state.anchor_y - PICKER_HEIGHT / 2.0;
        let top_min = crate::widgets::TOP_BAR_HEIGHT + 8.0;
        let bottom_max = (viewport_h - PICKER_HEIGHT - 8.0).max(top_min);
        if y < top_min {
            y = top_min;
        }
        if y > bottom_max {
            y = bottom_max;
        }
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(PICKER_WIDTH, PICKER_HEIGHT),
        }
    }

    /// Map a press point to a hit, or `None` if outside the panel.
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<ColorPickerHit> {
        JianColorPicker::hit(panel, point)
    }

    /// New `(sat, val)` when dragging inside the SV box.
    pub fn sv_at(&self, panel: Rect, point: Point2D) -> (f32, f32) {
        JianColorPicker::sv_at(panel, point)
    }

    /// New hue when dragging across the hue strip.
    pub fn hue_at(&self, panel: Rect, point: Point2D) -> f32 {
        JianColorPicker::hue_at(panel, point)
    }
}

fn anchored_x(anchor_x: f32, viewport_w: f32) -> f32 {
    let gap = 12.0;
    let right = anchor_x + gap;
    let max_x = (viewport_w - PICKER_WIDTH - 8.0).max(8.0);
    let preferred = if right + PICKER_WIDTH <= viewport_w - 8.0 {
        right
    } else {
        anchor_x - PICKER_WIDTH - gap
    };
    preferred.clamp(8.0, max_x)
}

impl Widget for ColorPicker {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &crate::widgets::LayoutCx) -> crate::widgets::LayoutBox {
        crate::widgets::LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(PICKER_WIDTH, PICKER_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        JianColorPicker {
            hue: self.state.hue,
            sat: self.state.sat,
            val: self.state.val,
            title: target_label(self.state.target),
            eyedropper_icon: Icon::Pencil.paths(),
            close_icon: Icon::Close.paths(),
            hex_input: self.state.hex_focused.then_some(&self.state.hex_input),
            rgb_input: self.state.rgb_focus.map(|ch| (ch, &self.state.rgb_input)),
            now_ms: self.now_ms,
        }
        .paint(
            cx.backend,
            rect,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label("Color picker");
        node
    }
}

/// Translate a press hit into the persistent drag kind the host keeps live.
pub fn drag_for_hit(hit: ColorPickerHit) -> Option<ColorPickerDrag> {
    match hit {
        ColorPickerHit::SvBox => Some(ColorPickerDrag::SvBox),
        ColorPickerHit::HueSlider => Some(ColorPickerDrag::HueSlider),
        _ => None,
    }
}

pub fn target_label(t: ColorTarget) -> &'static str {
    match t {
        ColorTarget::Fill => "Fill",
        ColorTarget::Stroke => "Stroke",
        ColorTarget::GradientStop(_) => "Gradient Stop",
        ColorTarget::EffectColor(_) => "Effect Color",
    }
}
