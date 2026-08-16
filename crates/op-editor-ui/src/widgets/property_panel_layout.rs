//! Layout walkers for the property panel — pure-math helpers that
//! emit the on-screen rect of every editable input and every
//! button/checkbox the panel paints, so hit-tests stay aligned
//! with paint at all section-visibility / fill-type combinations.
//!
//! Pulled out of `property_panel_sections.rs` to keep that file
//! under the 800-line ceiling.

use crate::widgets::property_panel::{EffectSummary, FillSummary, PropertyPanelAction};
use crate::widgets::property_panel_fill::fill_row_height;
use crate::widgets::property_panel_inputs::{
    COLOR_VARIABLE_BUTTON_W, COLOR_VARIABLE_GAP, HEADER_HEIGHT, INPUT_HEIGHT, PAD_X, SECTION_GAP,
    SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::property_panel_interactions::{
    interactions_section_height, push_interaction_action_rects, InteractionSummary,
};
use crate::widgets::property_panel_stroke::{push_stroke_action_rects, stroke_section_body_height};
use crate::{Point2D, Rect};
use op_editor_core::FillType;

pub(crate) use crate::widgets::property_panel_visibility::SectionCapabilities;
pub use crate::widgets::property_panel_visibility::{ComponentButtonState, VisibleSections};

pub use crate::widgets::property_panel_input_layout::editable_input_rects;

pub use crate::widgets::property_panel_effects::{EFFECT_ROW_GAP, EFFECT_ROW_HEIGHT};
pub const COLOR_VARIABLE_MENU_W: f32 = 210.0;
pub const COLOR_VARIABLE_MENU_ROW_H: f32 = 32.0;
pub const COLOR_VARIABLE_MENU_PAD_Y: f32 = 6.0;

/// Total vertical space the Effects section consumes: the section
/// header plus one block per effect (or an 8 px filler when the node
/// has none). Paint (`paint_effects_section`) and the action-rect
/// walker both consult this so their y-math can never drift.
pub fn effects_section_height(effects: &[EffectSummary]) -> f32 {
    SECTION_HEADER_HEIGHT
        + if effects.is_empty() {
            8.0
        } else {
            effects.len() as f32 * (EFFECT_ROW_HEIGHT + EFFECT_ROW_GAP)
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
    let extra_compositing_row = if visible.compositing {
        crate::widgets::property_panel_compositing::COMPOSITING_ROW_HEIGHT
    } else {
        0.0
    };
    SECTION_HEADER_HEIGHT + INPUT_HEIGHT + extra_arc_row + extra_compositing_row + 12.0
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
    interactions: &InteractionSummary,
) -> Vec<(PropertyPanelAction, Rect)> {
    action_button_rects_with_fill_picker(
        panel_rect,
        visible,
        effects,
        fills,
        interactions,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
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
    action_button_rects(
        panel_rect,
        visible,
        effects,
        fills,
        &InteractionSummary::default(),
    )
    .into_iter()
    .find_map(|(action, rect)| {
        matches!(action, PropertyPanelAction::ToggleFillTypePicker(i) if i == index).then_some(rect)
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
    interactions: &InteractionSummary,
) -> f32 {
    let actions = action_button_rects_with_fill_picker(
        panel_rect,
        visible,
        effects,
        fills,
        interactions,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
    );
    let inputs = editable_input_rects(panel_rect, visible, fills, effects);
    let bottom = actions
        .iter()
        .map(|(_, r)| r.origin.y + r.size.y)
        .chain(inputs.iter().map(|(_, r)| r.origin.y + r.size.y))
        .fold(panel_rect.origin.y, f32::max);
    (bottom - panel_rect.origin.y) + 16.0
}

/// Same as `action_button_rects` but the picker-open flags add
/// hit-rects for popup rows that overlay later sections. Fill-type
/// rows use their dedicated viewport-aware overlay hit-test; the two
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
    interactions: &InteractionSummary,
    _fill_picker_open: bool,
    _fill_type_picker_index: usize,
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
        out.extend(crate::widgets::property_panel_instance::action_rects(
            x0,
            y,
            w,
            visible.component_button,
        ));
        y += crate::widgets::property_panel_inputs::create_component_block_height(
            visible.component_button,
        );
    }
    // Position section.
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    if visible.corner_per_corner {
        out.push((
            PropertyPanelAction::ToggleCornerExpand,
            crate::widgets::property_panel_corner::uniform_and_toggle_rects(x0, y, w).1,
        ));
    }
    if visible.corner_radius && visible.corner_per_corner && visible.corner_expand {
        y += INPUT_HEIGHT + crate::widgets::property_panel_corner::CORNER_GRID_EXTRA_HEIGHT + 12.0;
    } else {
        y += INPUT_HEIGHT + 12.0;
    }
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
            visible.touch_controls,
        );
        y += crate::widgets::property_panel_flex::flex_section_height(
            visible.flex_layout_mode,
            visible.padding_edit_mode,
            visible.touch_controls,
        ) - SECTION_HEADER_HEIGHT;
    }

    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        // W/H remain editable in every sizing mode. Committing a number
        // switches only that axis back to fixed sizing.
        y += INPUT_HEIGHT + 10.0;
        let row_h =
            crate::widgets::property_panel_inputs::size_check_row_height(visible.touch_controls);
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
            x0,
            y,
            usable_w,
            visible.touch_controls,
        ));
        // The font-family picker is an overlay now (searchable list,
        // `property_panel_typography.rs`) — its rows are hit-tested
        // BEFORE this walker, not emitted from it.
        let _ = font_picker_open;
        if font_weight_picker_open {
            out.extend(
                crate::widgets::property_panel_text::font_weight_picker_action_rects(
                    x0,
                    y,
                    usable_w,
                    visible.touch_controls,
                ),
            );
        }
        y += crate::widgets::property_panel_text::text_section_height(visible.touch_controls);
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
        if visible.compositing {
            let mut compositing_y = y + SECTION_HEADER_HEIGHT + INPUT_HEIGHT;
            if visible.ellipse_arc {
                compositing_y += INPUT_HEIGHT + 6.0;
            }
            compositing_y += crate::widgets::property_panel_compositing::COMPOSITING_ROW_GAP;
            for (target, rect) in
                crate::widgets::property_panel_compositing::node_trigger_rects(x0, compositing_y, w)
            {
                out.push((PropertyPanelAction::ToggleCompositingPicker(target), rect));
            }
        }
        y += layer_section_height(visible);
        y += SECTION_GAP;
    }
    if visible.fill {
        y = crate::widgets::property_panel_input_layout::push_fill_action_rects(
            &mut out, panel_rect, visible, fills, y,
        );
        y += SECTION_GAP;
    }
    if visible.stroke {
        // Mirrors paint_stroke_section: header + hex/width row + side grid.
        let section_y = y;
        push_stroke_action_rects(
            &mut out,
            x0,
            section_y,
            w,
            visible.stroke_mode_popover_open,
            visible.touch_controls,
        );
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
                    size: Point2D::new(
                        crate::widgets::property_panel_inputs::color_swatch_action_width(
                            visible.touch_controls,
                        ),
                        INPUT_HEIGHT,
                    ),
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
        }
        y += stroke_section_body_height(visible.stroke_edit_mode);
        y += SECTION_GAP;
    }
    if visible.effects {
        // Mirrors `paint_effects_section`: header + one block per
        // effect. The header's "+" opens the three-kind add-menu.
        let plus = crate::widgets::property_panel_inputs::section_add_target(
            x0,
            y,
            w,
            visible.touch_controls,
        );
        out.push((PropertyPanelAction::ToggleEffectAddPicker, plus));
        y += SECTION_HEADER_HEIGHT;
        for (ei, eff) in effects.iter().enumerate() {
            let rects = crate::widgets::property_panel_effects::effect_row_rects(
                x0,
                y,
                w,
                visible.touch_controls,
            );
            out.push((PropertyPanelAction::RemoveEffect(ei), rects.remove));
            out.push((
                PropertyPanelAction::SetEffectVisible(ei, !eff.visible),
                rects.eye,
            ));
            let field = if matches!(eff.kind, crate::widgets::property_panel::EffectKind::Shadow) {
                op_editor_core::EffectField::Blur
            } else {
                op_editor_core::EffectField::Radius
            };
            out.push((
                PropertyPanelAction::AdjustEffectParam {
                    effect: ei,
                    field,
                    new_value: eff.blur,
                },
                rects.slider,
            ));
            y += EFFECT_ROW_HEIGHT + EFFECT_ROW_GAP;
        }
        if effects.is_empty() {
            y += 8.0;
        }
        y += SECTION_GAP;
    }
    if visible.interactions {
        push_interaction_action_rects(&mut out, interactions, x0, y, w);
        y += interactions_section_height(interactions);
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
            let formats: Vec<_> = [
                op_editor_core::ExportFormat::Png,
                op_editor_core::ExportFormat::Jpeg,
                op_editor_core::ExportFormat::Webp,
                op_editor_core::ExportFormat::Svg,
            ]
            .into_iter()
            .filter(|format| format.is_implemented())
            .collect();
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
