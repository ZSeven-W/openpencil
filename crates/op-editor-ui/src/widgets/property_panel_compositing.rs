//! Layer / fill compositing controls shared by paint and hit-test.
//!
//! BlendMode has 16 choices, so its popup uses two columns of eight
//! rows. This keeps every mode reachable without adding a second
//! scroll owner to the already-scrollable property rail.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{
    CompositingTarget, NodeSnapshot, PropertyPanel, PropertyPanelAction,
};
use crate::widgets::property_panel_inputs::{INPUT_HEIGHT, PAD_X};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use jian_ops_schema::node::base::MaskType;
use jian_ops_schema::style::BlendMode;
use jian_widgets::components::select::{SelectHit, SelectState};

pub const COMPOSITING_ROW_GAP: f32 = 6.0;
pub const COMPOSITING_ROW_HEIGHT: f32 = INPUT_HEIGHT + COMPOSITING_ROW_GAP;
const POPUP_ROW_H: f32 = 30.0;
const POPUP_PAD: f32 = 4.0;
const BLEND_COLS: usize = 2;

/// Layer-section trigger geometry. Both fields share one compact row.
pub fn node_trigger_rects(x: f32, y: f32, width: f32) -> [(CompositingTarget, Rect); 2] {
    let usable = width - PAD_X * 2.0;
    let half = (usable - 8.0) / 2.0;
    [
        (
            CompositingTarget::NodeBlend,
            Rect::xywh(x + PAD_X, y, half, INPUT_HEIGHT),
        ),
        (
            CompositingTarget::NodeMask,
            Rect::xywh(x + PAD_X + half + 8.0, y, half, INPUT_HEIGHT),
        ),
    ]
}

/// Per-fill Blend trigger geometry, painted after that fill's
/// type-specific body.
pub fn fill_trigger_rect(x: f32, y: f32, width: f32) -> Rect {
    Rect::xywh(x + PAD_X, y, width - PAD_X * 2.0, INPUT_HEIGHT)
}

fn blend_choices() -> [Option<BlendMode>; 16] {
    [
        None,
        Some(BlendMode::Darken),
        Some(BlendMode::Multiply),
        Some(BlendMode::Screen),
        Some(BlendMode::Overlay),
        Some(BlendMode::Lighten),
        Some(BlendMode::Difference),
        Some(BlendMode::Hue),
        Some(BlendMode::Saturation),
        Some(BlendMode::Color),
        Some(BlendMode::Luminosity),
        Some(BlendMode::SoftLight),
        Some(BlendMode::ColorDodge),
        Some(BlendMode::ColorBurn),
        Some(BlendMode::HardLight),
        Some(BlendMode::Exclusion),
    ]
}

fn mask_choices() -> [Option<MaskType>; 4] {
    [
        None,
        Some(MaskType::Alpha),
        Some(MaskType::Vector),
        Some(MaskType::Luminance),
    ]
}

fn blend_key(mode: Option<&BlendMode>) -> &'static str {
    match mode {
        None | Some(BlendMode::Normal) => "blendMode.normal",
        Some(BlendMode::Darken) => "blendMode.darken",
        Some(BlendMode::Multiply) => "blendMode.multiply",
        Some(BlendMode::Screen) => "blendMode.screen",
        Some(BlendMode::Overlay) => "blendMode.overlay",
        Some(BlendMode::Lighten) => "blendMode.lighten",
        Some(BlendMode::Difference) => "blendMode.difference",
        Some(BlendMode::Hue) => "blendMode.hue",
        Some(BlendMode::Saturation) => "blendMode.saturation",
        Some(BlendMode::Color) => "blendMode.color",
        Some(BlendMode::Luminosity) => "blendMode.luminosity",
        Some(BlendMode::SoftLight) => "blendMode.softLight",
        Some(BlendMode::ColorDodge) => "blendMode.colorDodge",
        Some(BlendMode::ColorBurn) => "blendMode.colorBurn",
        Some(BlendMode::HardLight) => "blendMode.hardLight",
        Some(BlendMode::Exclusion) => "blendMode.exclusion",
    }
}

fn mask_key(mode: Option<MaskType>) -> &'static str {
    match mode {
        None => "maskType.none",
        Some(MaskType::Alpha) => "maskType.alpha",
        Some(MaskType::Vector) => "maskType.vector",
        Some(MaskType::Luminance) => "maskType.luminance",
    }
}

fn translated(locale: op_editor_core::Locale, key: &'static str) -> &'static str {
    op_i18n::translate(locale, key)
}

fn trigger_label(
    locale: op_editor_core::Locale,
    title_key: &'static str,
    value: &'static str,
) -> String {
    format!("{} · {}", translated(locale, title_key), value)
}

pub fn paint_node_triggers(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) {
    let blend = translated(locale, blend_key(snapshot.blend_mode.as_ref()));
    let mask = translated(locale, mask_key(snapshot.mask_type));
    let labels = [
        trigger_label(locale, "layer.blendMode", blend),
        trigger_label(locale, "layer.maskType", mask),
    ];
    for ((_, rect), label) in node_trigger_rects(x, y, width)
        .into_iter()
        .zip(labels.iter())
    {
        paint_trigger(cx, theme, rect, label);
    }
}

pub fn paint_fill_trigger(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    mode: Option<&BlendMode>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) {
    let value = translated(locale, blend_key(mode));
    let label = trigger_label(locale, "fill.blendMode", value);
    paint_trigger(cx, theme, fill_trigger_rect(x, y, width), &label);
}

const TRIGGER_FONT_SIZE: f32 = 11.0;

fn paint_trigger(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, label: &str) {
    // jian's SelectTrigger clips its value instead of ellipsizing it — see
    // `text_metrics::fit_select_trigger_label`.
    let label = crate::widgets::text_metrics::fit_select_trigger_label(
        cx.backend,
        label,
        rect,
        TRIGGER_FONT_SIZE,
    );
    jian_widgets::components::select_trigger::SelectTrigger {
        icon_paths: None,
        label: &label,
        placeholder: "",
        hovered: false,
        pressed: false,
        enabled: true,
        font_size: TRIGGER_FONT_SIZE,
        bordered: true,
    }
    .paint(
        cx.backend,
        rect,
        &crate::widgets::button::tokens_from_theme(theme),
    );
}

fn row_count(target: CompositingTarget) -> usize {
    match target {
        CompositingTarget::NodeMask => mask_choices().len(),
        CompositingTarget::NodeBlend | CompositingTarget::FillBlend(_) => blend_choices().len(),
    }
}

fn popup_columns(target: CompositingTarget) -> usize {
    match target {
        CompositingTarget::NodeMask => 1,
        CompositingTarget::NodeBlend | CompositingTarget::FillBlend(_) => BLEND_COLS,
    }
}

fn popup_rows(target: CompositingTarget) -> usize {
    row_count(target).div_ceil(popup_columns(target))
}

pub fn popup_rect(anchor: Rect, viewport: Rect, target: CompositingTarget) -> Rect {
    let columns = popup_columns(target);
    let width = if columns == 2 {
        240.0
    } else {
        anchor.size.x.max(160.0)
    }
    .min(viewport.size.x.max(0.0));
    let height = popup_rows(target) as f32 * POPUP_ROW_H + POPUP_PAD * 2.0;
    let max_x = (viewport.origin.x + viewport.size.x - width).max(viewport.origin.x);
    let x = anchor.origin.x.clamp(viewport.origin.x, max_x);
    let below = anchor.origin.y + anchor.size.y + 4.0;
    let bottom = viewport.origin.y + viewport.size.y;
    let y = if below + height <= bottom {
        below
    } else {
        (anchor.origin.y - height - 4.0).max(viewport.origin.y)
    };
    Rect::xywh(
        x,
        y.min((bottom - height).max(viewport.origin.y)),
        width,
        height,
    )
}

fn row_rect(popup: Rect, target: CompositingTarget, index: usize) -> Option<Rect> {
    if index >= row_count(target) {
        return None;
    }
    let columns = popup_columns(target);
    let rows = popup_rows(target);
    let column = if columns == 1 { 0 } else { index / rows };
    let row = if columns == 1 { index } else { index % rows };
    let cell_w = (popup.size.x - POPUP_PAD * 2.0) / columns as f32;
    Some(Rect::xywh(
        popup.origin.x + POPUP_PAD + column as f32 * cell_w,
        popup.origin.y + POPUP_PAD + row as f32 * POPUP_ROW_H,
        cell_w,
        POPUP_ROW_H,
    ))
}

fn selected_index(target: CompositingTarget, snapshot: &NodeSnapshot) -> Option<usize> {
    match target {
        CompositingTarget::NodeMask => mask_choices()
            .iter()
            .position(|choice| *choice == snapshot.mask_type),
        CompositingTarget::NodeBlend => blend_choices()
            .iter()
            .position(|choice| choice.as_ref() == snapshot.blend_mode.as_ref()),
        CompositingTarget::FillBlend(index) => {
            let active = snapshot.fills.get(index)?.blend_mode.as_ref();
            blend_choices()
                .iter()
                .position(|choice| choice.as_ref() == active)
        }
    }
}

fn row_label(
    locale: op_editor_core::Locale,
    target: CompositingTarget,
    index: usize,
) -> Option<&'static str> {
    match target {
        CompositingTarget::NodeMask => mask_choices()
            .get(index)
            .map(|choice| translated(locale, mask_key(*choice))),
        CompositingTarget::NodeBlend | CompositingTarget::FillBlend(_) => blend_choices()
            .get(index)
            .map(|choice| translated(locale, blend_key(choice.as_ref()))),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    anchor: Rect,
    viewport: Rect,
    target: CompositingTarget,
    state: &SelectState,
    snapshot: &NodeSnapshot,
    locale: op_editor_core::Locale,
) {
    if !state.open {
        return;
    }
    let popup = popup_rect(anchor, viewport, target);
    cx.backend.fill_round_rect(popup, 7.0, theme.popover);
    cx.backend.stroke_round_rect(popup, 7.0, theme.border, 1.0);
    let selected = selected_index(target, snapshot);
    for index in 0..row_count(target) {
        let Some(rect) = row_rect(popup, target, index) else {
            continue;
        };
        let active = selected == Some(index);
        let wash = if state.pressed == Some(index) {
            Some(theme.button_hover.with_alpha(theme.button_hover.a * 1.8))
        } else if state.hover == Some(index) {
            Some(theme.button_hover)
        } else if active {
            Some(theme.row_selected_primary)
        } else {
            None
        };
        if let Some(wash) = wash {
            cx.backend.fill_round_rect(
                Rect::xywh(
                    rect.origin.x + 2.0,
                    rect.origin.y + 2.0,
                    rect.size.x - 4.0,
                    rect.size.y - 4.0,
                ),
                5.0,
                wash,
            );
        }
        let Some(label) = row_label(locale, target, index) else {
            continue;
        };
        let text = TextLayout::single_run(
            label,
            "system-ui",
            11.0,
            (if active {
                theme.primary
            } else {
                theme.foreground
            })
            .to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &text,
            Point2D::new(rect.origin.x + 24.0, rect.origin.y + 19.0),
        );
        if active {
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(rect.origin.x + 6.0, rect.origin.y + 8.0),
                12.0,
                theme.primary,
                1.5,
            );
        }
    }
}

pub fn hit(
    state: &SelectState,
    anchor: Rect,
    viewport: Rect,
    target: CompositingTarget,
    point: Point2D,
) -> SelectHit {
    if !state.open {
        return SelectHit::Outside;
    }
    let popup = popup_rect(anchor, viewport, target);
    if !popup.contains(point) {
        return SelectHit::Outside;
    }
    for index in 0..row_count(target) {
        if row_rect(popup, target, index).is_some_and(|rect| rect.contains(point)) {
            return SelectHit::Row(index);
        }
    }
    SelectHit::Inside
}

pub fn action_for_row(target: CompositingTarget, index: usize) -> Option<PropertyPanelAction> {
    match target {
        CompositingTarget::NodeMask => Some(PropertyPanelAction::SetNodeMaskType(
            mask_choices().get(index).copied()?,
        )),
        CompositingTarget::NodeBlend => Some(PropertyPanelAction::SetNodeBlendMode(
            blend_choices().get(index)?.clone(),
        )),
        CompositingTarget::FillBlend(fill_index) => Some(PropertyPanelAction::SetFillBlendMode {
            index: fill_index,
            mode: blend_choices().get(index)?.clone(),
        }),
    }
}

impl PropertyPanel {
    fn compositing_picker_viewport(panel_rect: Rect) -> Rect {
        let top = panel_rect.origin.y + crate::widgets::property_panel_inputs::TAB_HEIGHT;
        Rect::xywh(
            panel_rect.origin.x,
            top,
            panel_rect.size.x,
            (panel_rect.origin.y + panel_rect.size.y - top).max(0.0),
        )
    }

    fn compositing_trigger_rect(
        &self,
        panel_rect: Rect,
        target: CompositingTarget,
    ) -> Option<Rect> {
        crate::widgets::property_panel_sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            &self.snapshot.fills,
            &self.snapshot.interactions,
            self.fill_type_picker.open,
            self.fill_type_picker_index,
            self.font_picker.open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        )
        .into_iter()
        .find_map(|(action, rect)| {
            matches!(action, PropertyPanelAction::ToggleCompositingPicker(t) if t == target)
                .then_some(rect)
        })
    }

    pub fn compositing_picker_hit(&self, panel_rect: Rect, point: Point2D) -> SelectHit {
        self.compositing_picker_hit_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    pub(super) fn compositing_picker_hit_logical(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> SelectHit {
        if self.is_multi || !self.compositing_picker.open {
            return SelectHit::Outside;
        }
        let Some(target) = self.compositing_picker_target else {
            return SelectHit::Outside;
        };
        let Some(anchor) = self.compositing_trigger_rect(panel_rect, target) else {
            return SelectHit::Outside;
        };
        hit(
            &self.compositing_picker,
            anchor,
            Self::compositing_picker_viewport(panel_rect),
            target,
            point,
        )
    }

    pub fn compositing_picker_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        match self.compositing_picker_hit_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        ) {
            SelectHit::Row(index) => Some(index),
            SelectHit::Inside | SelectHit::Outside => None,
        }
    }

    pub fn compositing_picker_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.compositing_picker_contains_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    pub(super) fn compositing_picker_contains_logical(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> bool {
        !matches!(
            self.compositing_picker_hit_logical(panel_rect, point),
            SelectHit::Outside
        )
    }

    pub fn compositing_picker_action_at(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<PropertyPanelAction> {
        let target = self.compositing_picker_target?;
        let SelectHit::Row(index) = self.compositing_picker_hit_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        ) else {
            return None;
        };
        action_for_row(target, index)
    }

    pub(crate) fn paint_compositing_picker(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        if self.is_multi || !self.compositing_picker.open {
            return;
        }
        let Some(target) = self.compositing_picker_target else {
            return;
        };
        let Some(anchor) = self.compositing_trigger_rect(panel_rect, target) else {
            return;
        };
        paint_picker(
            cx,
            &self.theme,
            anchor,
            Self::compositing_picker_viewport(panel_rect),
            target,
            &self.compositing_picker,
            &self.snapshot,
            self.locale,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_popup_exposes_all_sixteen_modes_in_two_columns() {
        let target = CompositingTarget::NodeBlend;
        assert_eq!(row_count(target), 16);
        let popup = popup_rect(
            Rect::xywh(20.0, 20.0, 100.0, 30.0),
            Rect::xywh(0.0, 0.0, 280.0, 500.0),
            target,
        );
        assert_eq!(popup_rows(target), 8);
        assert!(row_rect(popup, target, 15).is_some());
        assert!(matches!(
            action_for_row(target, 15),
            Some(PropertyPanelAction::SetNodeBlendMode(Some(
                BlendMode::Exclusion
            )))
        ));
    }

    #[test]
    fn mask_none_and_canonical_modes_are_all_reachable() {
        let target = CompositingTarget::NodeMask;
        assert_eq!(row_count(target), 4);
        assert_eq!(
            action_for_row(target, 0),
            Some(PropertyPanelAction::SetNodeMaskType(None))
        );
        assert_eq!(
            action_for_row(target, 3),
            Some(PropertyPanelAction::SetNodeMaskType(Some(
                MaskType::Luminance
            )))
        );
    }

    fn legacy_mask_state() -> op_editor_core::EditorState {
        let doc = jian_ops_schema::load_str(
            r##"{
                "version":"1.0.0",
                "children":[{
                    "type":"path",
                    "id":"legacy-mask",
                    "name":"Legacy mask",
                    "d":"M0 0H40V40Z",
                    "width":40,
                    "height":40,
                    "mask":true,
                    "blendMode":"normal",
                    "fill":[{"type":"solid","color":"#ffffff","blendMode":"normal"}]
                }]
            }"##,
        )
        .expect("legacy compositing fixture")
        .value;
        let mut state = op_editor_core::EditorState::from_document(doc);
        state.set_single_selection(op_editor_core::NodeId::new("legacy-mask"));
        state
    }

    #[test]
    fn snapshot_normalizes_explicit_normal_and_surfaces_legacy_mask_as_alpha() {
        let state = legacy_mask_state();
        let panel = PropertyPanel::for_selection(&state).expect("path inspector");
        assert_eq!(panel.snapshot.blend_mode, None);
        assert_eq!(panel.snapshot.mask_type, Some(MaskType::Alpha));
        assert_eq!(panel.snapshot.fills[0].blend_mode, None);
        assert_eq!(
            selected_index(CompositingTarget::NodeBlend, &panel.snapshot),
            Some(0)
        );
    }

    #[test]
    fn open_panel_picker_hits_last_exclusion_choice() {
        let mut state = legacy_mask_state();
        state.editor_ui.compositing_picker.open = true;
        state.editor_ui.compositing_picker_target = Some(CompositingTarget::NodeBlend);
        let panel = PropertyPanel::for_selection(&state).expect("path inspector");
        let panel_rect = Rect::xywh(0.0, 0.0, 280.0, 900.0);
        let target = CompositingTarget::NodeBlend;
        let anchor = panel.compositing_trigger_rect(panel_rect, target).unwrap();
        let popup = popup_rect(
            anchor,
            PropertyPanel::compositing_picker_viewport(panel_rect),
            target,
        );
        let last = row_rect(popup, target, 15).unwrap();
        let point = Point2D::new(last.origin.x + 5.0, last.origin.y + 5.0);
        assert_eq!(
            panel.hit_test_action(panel_rect, point),
            Some(PropertyPanelAction::SetNodeBlendMode(Some(
                BlendMode::Exclusion
            )))
        );
    }
}
