//! Layout walkers for the property panel — pure-math helpers that
//! emit the on-screen rect of every editable input and every
//! button/checkbox the panel paints, so hit-tests stay aligned
//! with paint at all section-visibility / fill-type combinations.
//!
//! Pulled out of `property_panel_sections.rs` to keep that file
//! under the 800-line ceiling.

use crate::widgets::property_panel::{EffectKind, EffectSummary, FillSummary, PropertyPanelAction};
use crate::widgets::property_panel_fill::fill_row_height;
use crate::widgets::property_panel_inputs::{
    COLOR_VARIABLE_BUTTON_W, COLOR_VARIABLE_GAP, CREATE_COMPONENT_BTN_H, CREATE_COMPONENT_PAD_TOP,
    HEADER_HEIGHT, INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::property_panel_stroke::{push_stroke_action_rects, stroke_section_body_height};
use crate::{Point2D, Rect};
use op_editor_core::{EffectField, FillType};

pub(crate) use crate::widgets::property_panel_visibility::SectionCapabilities;
pub use crate::widgets::property_panel_visibility::{ComponentButtonState, VisibleSections};

pub use crate::widgets::property_panel_input_layout::editable_input_rects;

/// Height (px) of an effect card's title row — `投影` label on
/// the left + remove `—` icon on the right.
pub const EFFECT_TITLE_ROW_HEIGHT: f32 = INPUT_HEIGHT;

/// Height (px) of one effect-parameter input row inside the
/// 2-column grid. Sits below the title row.
pub const EFFECT_PARAM_ROW_HEIGHT: f32 = INPUT_HEIGHT + 4.0;

/// Vertical padding above + below the card body inside the
/// outlined card.
pub const EFFECT_CARD_PAD: f32 = 6.0;

/// Gap between stacked effect cards in the section.
pub const EFFECT_CARD_GAP: f32 = 6.0;

/// Height (px) of an effect's header row. Kept under the legacy
/// name so existing tests / hit-test callers compile.
pub const EFFECT_ROW_HEIGHT: f32 = EFFECT_TITLE_ROW_HEIGHT;

/// Doc-px a single "−" / "+" stepper click moves an effect parameter.
pub const EFFECT_PARAM_STEP: f32 = 1.0;
pub const COLOR_VARIABLE_MENU_W: f32 = 210.0;
pub const COLOR_VARIABLE_MENU_ROW_H: f32 = 32.0;
pub const COLOR_VARIABLE_MENU_PAD_Y: f32 = 6.0;

/// The editable scalar params an effect kind exposes, in row order,
/// each paired with its short row label. Shadow exposes four; the
/// blur kinds expose a single radius.
pub fn effect_param_fields(kind: EffectKind) -> &'static [(EffectField, &'static str)] {
    match kind {
        EffectKind::Shadow => &[
            (EffectField::OffsetX, "X"),
            (EffectField::OffsetY, "Y"),
            (EffectField::Blur, "Blur"),
            (EffectField::Spread, "Spread"),
        ],
        EffectKind::Blur | EffectKind::BackgroundBlur => &[(EffectField::Radius, "Radius")],
    }
}

/// On-screen rect of one effect-parameter input box inside the
/// card grid. Cards lay parameters out in a 2-column grid below
/// the title row; `col` is 0 (left) or 1 (right), `row` is 0-based
/// from the first param row. `card_x` / `card_w` are the card's
/// outer geometry.
pub fn effect_param_rect(card_x: f32, card_y: f32, card_w: f32, col: usize, row: usize) -> Rect {
    let inner_x = card_x + EFFECT_CARD_PAD;
    let inner_w = card_w - EFFECT_CARD_PAD * 2.0;
    let col_w = (inner_w - 6.0) / 2.0;
    let col_x = inner_x + col as f32 * (col_w + 6.0);
    let row_y = card_y + EFFECT_TITLE_ROW_HEIGHT + row as f32 * EFFECT_PARAM_ROW_HEIGHT;
    Rect {
        origin: Point2D::new(col_x, row_y),
        size: Point2D::new(col_w, INPUT_HEIGHT),
    }
}

/// Colour row rect inside an effect card — full-width pill that
/// paints the colour swatch + `rgba(...)` text and acts as the
/// click target for the gradient-stop-style colour picker.
pub fn effect_color_rect(card_x: f32, card_y: f32, card_w: f32, param_rows: usize) -> Rect {
    let inner_x = card_x + EFFECT_CARD_PAD;
    let inner_w = card_w - EFFECT_CARD_PAD * 2.0;
    let row_y = card_y + EFFECT_TITLE_ROW_HEIGHT + param_rows as f32 * EFFECT_PARAM_ROW_HEIGHT;
    Rect {
        origin: Point2D::new(inner_x, row_y),
        size: Point2D::new(inner_w, INPUT_HEIGHT),
    }
}

/// Legacy single-input rect — preserved for callers that still
/// reach for the old layout (effects-section tests). Returns the
/// left column of the new grid layout.
pub fn effect_param_value_rect(x: f32, y: f32, width: f32) -> Rect {
    effect_param_rect(x, y - EFFECT_TITLE_ROW_HEIGHT, width, 0, 0)
}

/// Number of 2-column grid rows the card's editable params occupy:
/// `ceil(field_count / 2)`. Shadow has 4 → 2 rows; Blur/BG-Blur
/// have 1 → 1 row.
pub fn effect_param_row_count(kind: EffectKind) -> usize {
    effect_param_fields(kind).len().div_ceil(2)
}

/// Whether the card has a colour row (only Shadow does today).
pub fn effect_has_color_row(kind: EffectKind) -> bool {
    matches!(kind, EffectKind::Shadow)
}

/// Total height one effect card consumes — title + param grid +
/// optional colour row + top/bottom card padding.
pub fn effect_block_height(kind: EffectKind) -> f32 {
    let rows = effect_param_row_count(kind) as f32 * EFFECT_PARAM_ROW_HEIGHT;
    let colour = if effect_has_color_row(kind) {
        EFFECT_PARAM_ROW_HEIGHT
    } else {
        0.0
    };
    EFFECT_TITLE_ROW_HEIGHT + rows + colour + EFFECT_CARD_PAD * 2.0
}

/// Total vertical space the Effects section consumes: the section
/// header plus one block per effect (or an 8 px filler when the node
/// has none). Paint (`paint_effects_section`) and the action-rect
/// walker both consult this so their y-math can never drift.
pub fn effects_section_height(effects: &[EffectSummary]) -> f32 {
    SECTION_HEADER_HEIGHT
        + if effects.is_empty() {
            8.0
        } else {
            effects.iter().map(|e| effect_block_height(e.kind)).sum()
        }
}

/// Total vertical space the Fill section consumes: the section header
/// plus one row per fill (head row + body), plus the 12 px trailing
/// gap painted before the divider. `primary_stop_count` sizes the
/// primary fill's gradient body (the gradient-stop editor keys off the
/// primary fill). Mirrors the y-walk in `paint_fill_section`.
pub fn fills_section_height(fills: &[FillSummary], primary_stop_count: usize) -> f32 {
    let rows: f32 = fills
        .iter()
        .enumerate()
        .map(|(i, f)| fill_row_height(f.fill_type, i == 0, primary_stop_count))
        .sum();
    SECTION_HEADER_HEIGHT + rows + 12.0
}

/// State for the 5 Size checkboxes (fill / hug / clip).
#[derive(Debug, Clone, Copy)]
pub struct SizeFlags {
    pub fill_width: bool,
    pub fill_height: bool,
    pub hug_width: bool,
    pub hug_height: bool,
    pub clip_content: bool,
}

/// Height of the body that follows the head row in the Fill
/// section, per fill type. Used by every layout walker so the
/// y-offset of sections after Fill stays aligned with paint when
/// the user flips fill types.
pub fn fill_body_height(fill_type: FillType) -> f32 {
    // Legacy 2-stop assumption — kept for callers that don't yet
    // thread `stop_count`. Prefer `fill_body_height_with_stops`.
    fill_body_height_with_stops(fill_type, 2)
}

/// Stop-aware variant — accounts for the actual number of gradient
/// stops so adding / removing a stop reflows the panel correctly.
pub fn fill_body_height_with_stops(fill_type: FillType, stop_count: usize) -> f32 {
    let stops = stop_count.max(0);
    let stops_block = SECTION_HEADER_HEIGHT
        + stops as f32 * (INPUT_HEIGHT + 4.0)
        + if stops == 0 { 0.0 } else { 2.0 };
    match fill_type {
        FillType::Solid => INPUT_HEIGHT + 6.0,
        FillType::LinearGradient => INPUT_HEIGHT + 6.0 + stops_block + 6.0,
        FillType::RadialGradient => stops_block + 6.0,
        // Mesh + Shader show head-row only (per-vertex editing / shader
        // authoring deferred — shader is render-only in v1) — no body
        // block, so they contribute nothing past the head row.
        FillType::MeshGradient | FillType::Shader => 0.0,
        FillType::Image => INPUT_HEIGHT + 6.0,
    }
}

/// Height consumed by the Layer section. Polygon adds a same-row
/// side-count field; ellipse adds a second row for arc controls.
pub fn layer_section_height(visible: VisibleSections) -> f32 {
    if !visible.opacity {
        return 0.0;
    }
    let extra_arc_row = if visible.ellipse_arc {
        INPUT_HEIGHT + 6.0
    } else {
        0.0
    };
    SECTION_HEADER_HEIGHT + INPUT_HEIGHT + extra_arc_row + 12.0
}

/// Rects of every clickable button / checkbox in the panel —
/// flex-layout 3 buttons, size-options 5 checkboxes. Same y-walk
/// math as `editable_input_rects` so paint + hit-test stay in
/// sync regardless of which sections are filtered.
pub fn action_button_rects(
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    fills: &[FillSummary],
) -> Vec<(PropertyPanelAction, Rect)> {
    action_button_rects_with_fill_picker(
        panel_rect, visible, effects, fills, false, 0, false, false, false, false, false,
    )
}

/// The shared action-walker rect for the fill type dropdown at
/// `index`. Open fill-type overlays must anchor to this rect so
/// paint, hit-testing, and popup row actions stay in lockstep.
pub fn fill_type_toggle_action_rect(
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    fills: &[FillSummary],
    index: usize,
) -> Option<Rect> {
    action_button_rects(panel_rect, visible, effects, fills)
        .into_iter()
        .find_map(|(action, rect)| {
            matches!(action, PropertyPanelAction::ToggleFillTypePicker(i) if i == index)
                .then_some(rect)
        })
}

/// Height of one row in an Export-section inline select popup.
pub const EXPORT_PICKER_ROW_H: f32 = 30.0;

/// Total height (px) of the PropertyPanel's section content. The
/// scroll clamp uses it so the inspector cannot scroll past its
/// end. Computed as the furthest bottom edge across every action
/// rect + every editable-input rect (so it stays correct whichever
/// section happens to be last), plus a small trailing margin.
pub fn property_panel_content_height(
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    fills: &[FillSummary],
) -> f32 {
    let actions = action_button_rects_with_fill_picker(
        panel_rect, visible, effects, fills, false, 0, false, false, false, false, false,
    );
    let inputs = editable_input_rects(panel_rect, visible, fills);
    let bottom = actions
        .iter()
        .map(|(_, r)| r.origin.y + r.size.y)
        .chain(inputs.iter().map(|(_, r)| r.origin.y + r.size.y))
        .fold(panel_rect.origin.y, f32::max);
    (bottom - panel_rect.origin.y) + 16.0
}

/// Same as `action_button_rects` but the picker-open flags add
/// hit-rects for popup rows that overlay later sections:
/// `fill_picker_open` emits the 4 fill-type rows; the two
/// `export_*_picker_open` flags emit the Export section's inline
/// scale (3 rows) / format (5 rows) select popups. `effects` drives
/// the Effects section's per-effect "✕" and parameter-stepper rects
/// + that section's variable height.
#[allow(clippy::too_many_arguments)]
pub fn action_button_rects_with_fill_picker(
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    fills: &[FillSummary],
    fill_picker_open: bool,
    fill_type_picker_index: usize,
    font_picker_open: bool,
    font_weight_picker_open: bool,
    export_scale_picker_open: bool,
    export_format_picker_open: bool,
    padding_mode_popover_open: bool,
) -> Vec<(PropertyPanelAction, Rect)> {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let mut out: Vec<(PropertyPanelAction, Rect)> = Vec::new();

    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    if visible.create_component {
        use crate::widgets::property_panel_visibility::ComponentButtonState as CB;
        let first_row = Rect {
            origin: Point2D::new(x0 + PAD_X, y + CREATE_COMPONENT_PAD_TOP),
            size: Point2D::new(usable_w, CREATE_COMPONENT_BTN_H),
        };
        match visible.component_button {
            CB::Create => out.push((PropertyPanelAction::CreateComponent, first_row)),
            CB::DetachComponent => out.push((PropertyPanelAction::DetachComponent, first_row)),
            CB::Instance => {
                out.push((PropertyPanelAction::GoToComponent, first_row));
                out.push((
                    PropertyPanelAction::DetachInstance,
                    Rect {
                        origin: Point2D::new(
                            x0 + PAD_X,
                            first_row.origin.y
                                + CREATE_COMPONENT_BTN_H
                                + crate::widgets::property_panel_inputs::CREATE_COMPONENT_ROW_GAP,
                        ),
                        size: Point2D::new(usable_w, CREATE_COMPONENT_BTN_H),
                    },
                ));
            }
        }
        y += crate::widgets::property_panel_inputs::create_component_block_height(
            visible.component_button,
        );
    }
    // Position section.
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;

    if visible.flex_layout {
        y += SECTION_HEADER_HEIGHT;
        crate::widgets::property_panel_flex::push_flex_action_rects(
            &mut out,
            x0,
            y,
            w,
            visible.flex_layout_mode,
            visible.layout_justify,
            padding_mode_popover_open,
        );
        y += crate::widgets::property_panel_flex::flex_section_height(
            visible.flex_layout_mode,
            visible.padding_edit_mode,
        ) - SECTION_HEADER_HEIGHT;
    }

    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        // The W/H input row collapses when both dimensions are fill/hug
        // (matches paint + editable_input_rects), so the size checkbox
        // rects below shift up by the row height.
        let w_visible = !visible.size_fill_width && !visible.size_hug_width;
        let h_visible = !visible.size_fill_height && !visible.size_hug_height;
        if w_visible || h_visible {
            y += INPUT_HEIGHT + 10.0;
        }
        let row_h = 22.0;
        out.push((
            PropertyPanelAction::ToggleSizeFillWidth,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        out.push((
            PropertyPanelAction::ToggleSizeFillHeight,
            Rect {
                origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        y += row_h;
        out.push((
            PropertyPanelAction::ToggleSizeHugWidth,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        out.push((
            PropertyPanelAction::ToggleSizeHugHeight,
            Rect {
                origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        y += row_h;
        if visible.clip_content {
            out.push((
                PropertyPanelAction::ToggleSizeClipContent,
                Rect {
                    origin: Point2D::new(x0 + PAD_X, y),
                    size: Point2D::new(usable_w, row_h),
                },
            ));
            y += row_h;
        }
        y += 12.0;
        y += SECTION_GAP;
    }

    if visible.icon {
        crate::widgets::property_panel_icon::push_icon_action_rects(&mut out, x0, y, w);
        y += crate::widgets::property_panel_icon::icon_section_height();
    }

    if visible.text {
        out.extend(crate::widgets::property_panel_text::text_action_rects(
            x0, y, usable_w,
        ));
        // The font-family picker is an overlay now (searchable list,
        // `property_panel_typography.rs`) — its rows are hit-tested
        // BEFORE this walker, not emitted from it.
        let _ = font_picker_open;
        if font_weight_picker_open {
            out.extend(
                crate::widgets::property_panel_text::font_weight_picker_action_rects(
                    x0, y, usable_w,
                ),
            );
        }
        y += crate::widgets::property_panel_text::text_section_height();
        y += SECTION_GAP;
    }

    if let Some(kind) = visible.widget {
        crate::widgets::property_panel_widget::push_widget_action_rects(
            &mut out,
            kind,
            visible.widget_checked,
            x0,
            y,
            usable_w,
        );
        y += crate::widgets::property_panel_widget::widget_section_height(kind);
        y += SECTION_GAP;
    }

    if visible.image {
        crate::widgets::property_panel_image_node::push_image_action_rects(
            &mut out,
            x0,
            y,
            w,
            visible.image_warning,
        );
        y += crate::widgets::property_panel_image_node::image_section_height(visible.image_warning);
        y += SECTION_GAP;
    }

    if visible.opacity {
        y += layer_section_height(visible);
        y += SECTION_GAP;
    }
    if visible.fill {
        y = crate::widgets::property_panel_input_layout::push_fill_action_rects(
            &mut out,
            panel_rect,
            visible,
            fills,
            fill_picker_open,
            fill_type_picker_index,
            y,
        );
        y += SECTION_GAP;
    }
    if visible.stroke {
        // Mirrors paint_stroke_section: header + hex/width row + side grid.
        let section_y = y;
        push_stroke_action_rects(&mut out, x0, section_y, w, visible.stroke_mode_popover_open);
        y += SECTION_HEADER_HEIGHT;
        // The stroke hex row's leading colour swatch opens the picker.
        let show_var = visible.color_variable_count > 0 || visible.stroke_variable_bound;
        let variable_w = if show_var {
            COLOR_VARIABLE_BUTTON_W + COLOR_VARIABLE_GAP
        } else {
            0.0
        };
        // Inline width only in Single mode (matches paint_stroke_main_row);
        // in per-side mode the hex fills the row so the variable button +
        // colour-picker anchor stay aligned.
        let inline = visible.stroke_edit_mode == op_editor_core::PaddingEditMode::Single;
        let width_w = if inline { 60.0 } else { 0.0 };
        let width_gap = if inline { 8.0 } else { 0.0 };
        let hex_w = usable_w - width_w - width_gap - variable_w;
        if !visible.stroke_variable_bound {
            out.push((
                PropertyPanelAction::OpenColorPicker(op_editor_core::ColorTarget::Stroke),
                Rect {
                    origin: Point2D::new(x0 + PAD_X, y),
                    size: Point2D::new(28.0, INPUT_HEIGHT),
                },
            ));
        }
        if show_var {
            let var_rect = Rect {
                origin: Point2D::new(x0 + PAD_X + hex_w + COLOR_VARIABLE_GAP, y),
                size: Point2D::new(COLOR_VARIABLE_BUTTON_W, INPUT_HEIGHT),
            };
            out.push((
                PropertyPanelAction::ToggleColorVariablePicker(op_editor_core::ColorTarget::Stroke),
                var_rect,
            ));
            if visible.color_variable_picker_open == Some(op_editor_core::ColorTarget::Stroke) {
                push_color_variable_picker_rects(
                    &mut out,
                    op_editor_core::ColorTarget::Stroke,
                    var_rect,
                    visible.color_variable_count,
                    visible.stroke_variable_bound,
                );
            }
        }
        y += stroke_section_body_height(visible.stroke_edit_mode);
        y += SECTION_GAP;
    }
    if visible.effects {
        // Mirrors `paint_effects_section`: header + one block per
        // effect. The header's "+" opens the add-menu (Drop Shadow /
        // Layer Blur choice) rather than adding directly.
        let plus = Rect {
            origin: Point2D::new(x0 + w - PAD_X - 22.0, y),
            size: Point2D::new(28.0, SECTION_HEADER_HEIGHT),
        };
        out.push((PropertyPanelAction::AddEffect, plus));
        y += SECTION_HEADER_HEIGHT;
        for (ei, eff) in effects.iter().enumerate() {
            // Card outer rect — used to anchor the title-row remove
            // glyph (top-right) and the 2-column param grid below.
            let card_x = x0 + PAD_X;
            let card_w = w - PAD_X * 2.0;
            // Remove (`—`) glyph in the title row's right corner.
            out.push((
                PropertyPanelAction::RemoveEffect(ei),
                Rect {
                    origin: Point2D::new(card_x + card_w - EFFECT_CARD_PAD - 18.0, y + 4.0),
                    size: Point2D::new(20.0, INPUT_HEIGHT - 4.0),
                },
            ));
            // 2-column param grid — each cell is a focusable input.
            let card_inner_y = y + EFFECT_CARD_PAD;
            for (i, &(field, _)) in effect_param_fields(eff.kind).iter().enumerate() {
                let col = i % 2;
                let row = i / 2;
                let cur = eff.param_value(field);
                out.push((
                    PropertyPanelAction::FocusEffectParam {
                        effect: ei,
                        field,
                        value: cur,
                    },
                    effect_param_rect(card_x, card_inner_y, card_w, col, row),
                ));
            }
            // Color row — emits the same picker action as a gradient
            // stop swatch, but indexed by the effect; see
            // `EffectColor` outcome path on the host. Wired only for
            // Shadow today (Blur kinds carry no colour).
            if effect_has_color_row(eff.kind) {
                let row_count = effect_param_row_count(eff.kind);
                let cr = effect_color_rect(card_x, card_inner_y, card_w, row_count);
                // Swatch sub-rect on the left of the color row — clicks
                // open a colour picker scoped to `effect[ei].color`.
                out.push((
                    PropertyPanelAction::OpenEffectColorPicker(ei),
                    Rect {
                        origin: Point2D::new(cr.origin.x + 32.0, cr.origin.y),
                        size: Point2D::new(28.0, cr.size.y),
                    },
                ));
            }
            y += effect_block_height(eff.kind) + EFFECT_CARD_GAP;
        }
        if effects.is_empty() {
            y += 8.0;
        }
        y += SECTION_GAP;
    }
    if visible.export {
        // Export section: header + a row of two dropdowns (scale on
        // the left, format on the right) mirroring
        // `paint_export_section`. Each dropdown is its own toggle
        // rect; when its inline select popup is open the option rows
        // are emitted too. `hit_test_action` walks the result in
        // `rev()`, so a popup-row hit is tested before its toggle.
        y += SECTION_HEADER_HEIGHT;
        let scale_rect = Rect {
            origin: Point2D::new(x0 + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        };
        let format_rect = Rect {
            origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        };
        out.push((PropertyPanelAction::ToggleExportScalePicker, scale_rect));
        out.push((PropertyPanelAction::ToggleExportFormatPicker, format_rect));
        y += INPUT_HEIGHT + 12.0;
        // Full-width Export action button below the dropdown row.
        out.push((
            PropertyPanelAction::ExportImageNow,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(usable_w, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 12.0;
        // The Export section is the last section, pinned to the
        // bottom of the panel — so its select popups open UPWARD,
        // stacking their option rows directly above the dropdown.
        // Opening downward would clip the rows at the panel edge.
        // `paint_select_popup` derives the popup background from the
        // first / last row rect, so it follows whatever rows emit
        // here. `first_row_y` is placed so the background's bottom
        // edge lands `4 px` above the dropdown (matches the `6 px`
        // top/bottom padding `paint_select_popup` adds).
        if export_scale_picker_open {
            let count = 3.0;
            let first_row_y = scale_rect.origin.y - 4.0 - 6.0 - count * EXPORT_PICKER_ROW_H;
            for (i, scale) in [1.0_f32, 2.0, 3.0].into_iter().enumerate() {
                out.push((
                    PropertyPanelAction::SetExportScale(scale),
                    Rect {
                        origin: Point2D::new(
                            scale_rect.origin.x,
                            first_row_y + i as f32 * EXPORT_PICKER_ROW_H,
                        ),
                        size: Point2D::new(scale_rect.size.x, EXPORT_PICKER_ROW_H),
                    },
                ));
            }
        }
        if export_format_picker_open {
            let formats = [
                op_editor_core::ExportFormat::Png,
                op_editor_core::ExportFormat::Jpeg,
                op_editor_core::ExportFormat::Webp,
                op_editor_core::ExportFormat::Svg,
            ];
            let count = formats.len() as f32;
            let first_row_y = format_rect.origin.y - 4.0 - 6.0 - count * EXPORT_PICKER_ROW_H;
            for (i, fmt) in formats.into_iter().enumerate() {
                out.push((
                    PropertyPanelAction::SetExportFormat(fmt),
                    Rect {
                        origin: Point2D::new(
                            format_rect.origin.x,
                            first_row_y + i as f32 * EXPORT_PICKER_ROW_H,
                        ),
                        size: Point2D::new(format_rect.size.x, EXPORT_PICKER_ROW_H),
                    },
                ));
            }
        }
    }
    let _ = y; // suppress unused-write lint if export is last

    out
}

pub(crate) fn push_color_variable_picker_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    target: op_editor_core::ColorTarget,
    anchor: Rect,
    count: usize,
    bound: bool,
) {
    let x = anchor.origin.x + anchor.size.x - COLOR_VARIABLE_MENU_W;
    let y = anchor.origin.y + anchor.size.y + 4.0 + COLOR_VARIABLE_MENU_PAD_Y;
    let mut row = 0usize;
    if bound {
        out.push((
            PropertyPanelAction::UnbindColorVariable(target),
            Rect {
                origin: Point2D::new(x, y),
                size: Point2D::new(COLOR_VARIABLE_MENU_W, COLOR_VARIABLE_MENU_ROW_H),
            },
        ));
        row += 1;
    }
    for index in 0..count {
        out.push((
            PropertyPanelAction::BindColorVariable { target, index },
            Rect {
                origin: Point2D::new(x, y + row as f32 * COLOR_VARIABLE_MENU_ROW_H),
                size: Point2D::new(COLOR_VARIABLE_MENU_W, COLOR_VARIABLE_MENU_ROW_H),
            },
        ));
        row += 1;
    }
}
