//! Layout walkers for the property panel — pure-math helpers that
//! emit the on-screen rect of every editable input and every
//! button/checkbox the panel paints, so hit-tests stay aligned
//! with paint at all section-visibility / fill-type combinations.
//!
//! Pulled out of `property_panel_sections.rs` to keep that file
//! under the 800-line ceiling.

use crate::widgets::property_panel::{EffectKind, EffectSummary, PropertyPanelAction};
use crate::widgets::property_panel_inputs::{
    HEADER_HEIGHT, INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::{Point2D, Rect};
use op_editor_core::{EffectField, FillType, FlexLayout, PropertyFocus};

/// Per-NodeKind toggles for which property-panel sections render.
/// Mirrors the TS app's behaviour where a Line node hides the
/// fill picker, a Frame hides Text properties, etc. Sections that
/// always apply (Position / Layer / Export) aren't gated here.
/// Lives here alongside `VisibleSections` — the mask it feeds.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SectionCapabilities {
    pub(crate) flex_layout: bool,
    pub(crate) size_options: bool,
    pub(crate) opacity: bool,
    pub(crate) fill: bool,
    pub(crate) stroke: bool,
    pub(crate) effects: bool,
    pub(crate) export: bool,
}

impl SectionCapabilities {
    /// Capability mask for the multi-select aggregate snapshot.
    /// Keeps Size (so the union W/H actually paint), hides
    /// fill/stroke (no aggregation in v1), keeps Layer/Effects/
    /// Export (paint safely with the zeroed snapshot fields).
    pub(crate) fn for_multi() -> Self {
        Self {
            flex_layout: false,
            size_options: true,
            opacity: true,
            fill: false,
            stroke: false,
            effects: true,
            export: true,
        }
    }

    pub(crate) fn for_kind(kind: &crate::layout_scene::NodeKind) -> Self {
        use crate::layout_scene::NodeKind as K;
        match kind {
            // Frame: full chrome — it can host children, take auto-
            // layout, fill / stroke / effects / export.
            K::Frame => Self {
                flex_layout: true,
                size_options: true,
                opacity: true,
                fill: true,
                stroke: true,
                effects: true,
                export: true,
            },
            // Group: structural — no fill / stroke, no flex slot
            // (children own layout). Opacity + export still apply.
            K::Group | K::Other(_) => Self {
                flex_layout: false,
                size_options: false,
                opacity: true,
                fill: false,
                stroke: false,
                effects: true,
                export: true,
            },
            // Rect / Ellipse / Polygon: full leaf — every paint
            // section applies; no flex (no children).
            K::Rect | K::Ellipse | K::Polygon => Self {
                flex_layout: false,
                size_options: true,
                opacity: true,
                fill: true,
                stroke: true,
                effects: true,
                export: true,
            },
            // Line / Path: only outline — fill doesn't apply.
            K::Line | K::Path => Self {
                flex_layout: false,
                size_options: true,
                opacity: true,
                fill: false,
                stroke: true,
                effects: true,
                export: true,
            },
            // Text: stroke is rare for text, but fill = ink colour.
            K::Text => Self {
                flex_layout: false,
                size_options: true,
                opacity: true,
                fill: true,
                stroke: false,
                effects: true,
                export: true,
            },
        }
    }
}

/// Whether each section currently paints — drives the layout
/// walk so when per-kind filtering hides a section, the rects
/// that follow shift up.
#[derive(Debug, Clone, Copy)]
pub struct VisibleSections {
    pub flex_layout: bool,
    pub size_options: bool,
    /// `Opacity` from the Layer section.
    pub opacity: bool,
    /// `FillHex` from the Fill section.
    pub fill: bool,
    /// `StrokeHex` + `StrokeWidth` from the Stroke section.
    pub stroke: bool,
    /// Effects section paints (header + add chip + one block per
    /// effect). Tracked because the export-rect walker needs to know
    /// whether it consumed vertical space ahead of the Export
    /// section. The per-effect geometry is driven by the `effects`
    /// slice the walker takes alongside this struct.
    pub effects: bool,
    /// Export section paints — its scale / format dropdown rects
    /// emit only when this is true.
    pub export: bool,
    /// Active fill type — affects fill-section body height so
    /// the walk past Fill stays aligned with paint when the user
    /// flips Solid / Gradient / Image.
    pub fill_type: FillType,
}

impl VisibleSections {
    /// Every section visible — matches the legacy unfiltered layout.
    pub const ALL: Self = Self {
        flex_layout: true,
        size_options: true,
        opacity: true,
        fill: true,
        stroke: true,
        effects: true,
        export: true,
        fill_type: FillType::Solid,
    };
}

/// Height (px) of an effect's header row — the type label + "✕".
pub const EFFECT_ROW_HEIGHT: f32 = INPUT_HEIGHT + 4.0;

/// Height (px) of one effect-parameter row — label + value + the
/// "−" / "+" steppers.
pub const EFFECT_PARAM_ROW_HEIGHT: f32 = INPUT_HEIGHT + 2.0;

/// Doc-px a single "−" / "+" stepper click moves an effect parameter.
pub const EFFECT_PARAM_STEP: f32 = 1.0;

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

/// On-screen rect of an effect-parameter's editable value box — the
/// click target that focuses it for keyboard entry. Shared by the
/// action-rect walker (hit-test) and `paint_effect_param_row`
/// (paint) so the two never drift. `y` is the param row's top.
pub fn effect_param_value_rect(x: f32, y: f32, width: f32) -> Rect {
    Rect {
        origin: Point2D::new(x + width - PAD_X - 104.0, y + 3.0),
        size: Point2D::new(52.0, INPUT_HEIGHT - 6.0),
    }
}

/// Total height one effect block consumes — its header row plus one
/// row per editable parameter.
pub fn effect_block_height(kind: EffectKind) -> f32 {
    EFFECT_ROW_HEIGHT + effect_param_fields(kind).len() as f32 * EFFECT_PARAM_ROW_HEIGHT
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
    match fill_type {
        FillType::Solid => INPUT_HEIGHT + 6.0,
        FillType::LinearGradient => {
            INPUT_HEIGHT + 6.0 + SECTION_HEADER_HEIGHT + INPUT_HEIGHT + 4.0 + INPUT_HEIGHT + 6.0
        }
        FillType::RadialGradient => SECTION_HEADER_HEIGHT + INPUT_HEIGHT + 4.0 + INPUT_HEIGHT + 6.0,
        FillType::Image => INPUT_HEIGHT + 6.0,
    }
}

/// Rects of every clickable button / checkbox in the panel —
/// flex-layout 3 buttons, size-options 5 checkboxes. Same y-walk
/// math as `editable_input_rects` so paint + hit-test stay in
/// sync regardless of which sections are filtered.
pub fn action_button_rects(
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
) -> Vec<(PropertyPanelAction, Rect)> {
    action_button_rects_with_fill_picker(panel_rect, visible, effects, false, false, false)
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
) -> f32 {
    let actions =
        action_button_rects_with_fill_picker(panel_rect, visible, effects, false, false, false);
    let inputs = editable_input_rects(panel_rect, visible);
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
pub fn action_button_rects_with_fill_picker(
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    fill_picker_open: bool,
    export_scale_picker_open: bool,
    export_format_picker_open: bool,
) -> Vec<(PropertyPanelAction, Rect)> {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;

    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    y += 8.0 + 36.0 + 12.0;
    // Position section.
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;

    let mut out: Vec<(PropertyPanelAction, Rect)> = Vec::new();

    if visible.flex_layout {
        y += SECTION_HEADER_HEIGHT;
        let btn_w = 56.0;
        let gap = 8.0;
        let row_x = x0 + PAD_X;
        let modes = [
            FlexLayout::Free,
            FlexLayout::Vertical,
            FlexLayout::Horizontal,
        ];
        for (i, mode) in modes.iter().enumerate() {
            let bx = row_x + i as f32 * (btn_w + gap);
            out.push((
                PropertyPanelAction::SetFlexLayout(*mode),
                Rect {
                    origin: Point2D::new(bx, y),
                    size: Point2D::new(btn_w, 32.0),
                },
            ));
        }
        y += 32.0 + 12.0;
        y += SECTION_GAP;
    }

    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 10.0;
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
        out.push((
            PropertyPanelAction::ToggleSizeClipContent,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(usable_w, row_h),
            },
        ));
        y += row_h + 12.0;
        y += SECTION_GAP;
    }

    if visible.opacity {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 12.0;
        y += SECTION_GAP;
    }
    if visible.fill {
        y += SECTION_HEADER_HEIGHT;
        // The head-row swatch is display-only — the colour picker
        // opens from the hex-row swatch below (added further down),
        // not from here.
        let dropdown_rect = Rect {
            origin: Point2D::new(x0 + PAD_X + 22.0 + 6.0, y),
            size: Point2D::new(usable_w - 22.0 - 6.0 - 50.0 - 22.0 - 12.0, INPUT_HEIGHT),
        };
        out.push((PropertyPanelAction::ToggleFillTypePicker, dropdown_rect));
        if fill_picker_open {
            let row_h = 32.0;
            let panel_w = dropdown_rect.size.x;
            let panel_x = dropdown_rect.origin.x;
            let panel_y = dropdown_rect.origin.y + dropdown_rect.size.y + 4.0;
            let types = [
                FillType::Solid,
                FillType::LinearGradient,
                FillType::RadialGradient,
                FillType::Image,
            ];
            for (i, t) in types.iter().enumerate() {
                out.push((
                    PropertyPanelAction::SetFillType(*t),
                    Rect {
                        origin: Point2D::new(panel_x, panel_y + 6.0 + i as f32 * row_h),
                        size: Point2D::new(panel_w, row_h),
                    },
                ));
            }
        }
        // Consume the rest of the Fill section so subsequent
        // sections' y math stays aligned with paint. Mirrors the
        // y-walk in `paint_fill_section`: head row + body + divider.
        y += INPUT_HEIGHT + 6.0; // head row (swatch + dropdown + opacity + X)
                                 // Solid fill's body is the hex row; its leading 16 px colour
                                 // swatch opens the picker. `hit_test_action` runs before the
                                 // hex-input focus hit-test, so a swatch click opens the
                                 // picker instead of focusing the hex field.
        if visible.fill_type == FillType::Solid {
            out.push((
                PropertyPanelAction::OpenColorPicker(op_editor_core::ColorTarget::Fill),
                Rect {
                    origin: Point2D::new(x0 + PAD_X, y),
                    size: Point2D::new(28.0, INPUT_HEIGHT),
                },
            ));
        }
        y += fill_body_height(visible.fill_type) - 6.0 + 12.0; // body + divider gap
        y += SECTION_GAP;
    }
    if visible.stroke {
        // Mirrors paint_stroke_section: header + hex/width row.
        y += SECTION_HEADER_HEIGHT;
        // The stroke hex row's leading colour swatch opens the picker.
        out.push((
            PropertyPanelAction::OpenColorPicker(op_editor_core::ColorTarget::Stroke),
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(28.0, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 12.0;
        y += SECTION_GAP;
    }
    if visible.effects {
        // Mirrors `paint_effects_section`: header + one block per
        // effect (header row + parameter rows). The header's "+"
        // button maps to `AddEffect`.
        let plus = Rect {
            origin: Point2D::new(x0 + w - PAD_X - 22.0, y),
            size: Point2D::new(28.0, SECTION_HEADER_HEIGHT),
        };
        out.push((PropertyPanelAction::AddEffect, plus));
        y += SECTION_HEADER_HEIGHT;
        for (ei, eff) in effects.iter().enumerate() {
            // "✕" on the effect's header row → RemoveEffect.
            out.push((
                PropertyPanelAction::RemoveEffect(ei),
                Rect {
                    origin: Point2D::new(x0 + w - PAD_X - 20.0, y + 2.0),
                    size: Point2D::new(20.0, INPUT_HEIGHT),
                },
            ));
            // One "−"/"+" stepper pair per editable parameter row.
            let mut py = y + EFFECT_ROW_HEIGHT;
            for &(field, _) in effect_param_fields(eff.kind) {
                let cur = eff.param_value(field);
                out.push((
                    PropertyPanelAction::AdjustEffectParam {
                        effect: ei,
                        field,
                        new_value: cur - EFFECT_PARAM_STEP,
                    },
                    Rect {
                        origin: Point2D::new(x0 + w - PAD_X - 48.0, py + 3.0),
                        size: Point2D::new(22.0, INPUT_HEIGHT - 6.0),
                    },
                ));
                out.push((
                    PropertyPanelAction::AdjustEffectParam {
                        effect: ei,
                        field,
                        new_value: cur + EFFECT_PARAM_STEP,
                    },
                    Rect {
                        origin: Point2D::new(x0 + w - PAD_X - 22.0, py + 3.0),
                        size: Point2D::new(22.0, INPUT_HEIGHT - 6.0),
                    },
                ));
                // The value box — click to type a value directly.
                out.push((
                    PropertyPanelAction::FocusEffectParam {
                        effect: ei,
                        field,
                        value: cur,
                    },
                    effect_param_value_rect(x0, py, w),
                ));
                py += EFFECT_PARAM_ROW_HEIGHT;
            }
            y += effect_block_height(eff.kind);
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
            let count = op_editor_core::ExportFormat::ALL.len() as f32;
            let first_row_y = format_rect.origin.y - 4.0 - 6.0 - count * EXPORT_PICKER_ROW_H;
            for (i, fmt) in op_editor_core::ExportFormat::ALL.into_iter().enumerate() {
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

/// On-screen rects of every editable input. Same y walk as
/// `action_button_rects` so paint + hit-test stay aligned.
pub fn editable_input_rects(
    panel_rect: Rect,
    visible: VisibleSections,
) -> Vec<(PropertyFocus, Rect)> {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;

    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    y += 8.0 + 36.0 + 12.0;
    y += SECTION_HEADER_HEIGHT;
    let x_rect = Rect {
        origin: Point2D::new(x0 + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let y_rect = Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    y += INPUT_HEIGHT + 6.0;
    let rotation_rect = Rect {
        origin: Point2D::new(x0 + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let radius_rect = Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    if visible.flex_layout {
        y += SECTION_HEADER_HEIGHT;
        y += 32.0 + 12.0;
        y += SECTION_GAP;
    }
    let mut rects = vec![
        (PropertyFocus::PositionX, x_rect),
        (PropertyFocus::PositionY, y_rect),
        (PropertyFocus::Rotation, rotation_rect),
        (PropertyFocus::PositionR, radius_rect),
    ];
    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        rects.push((
            PropertyFocus::SizeW,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(half_w, INPUT_HEIGHT),
            },
        ));
        rects.push((
            PropertyFocus::SizeH,
            Rect {
                origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
                size: Point2D::new(half_w, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 10.0;
        let check_h = 22.0;
        y += check_h * 3.0;
        y += 12.0;
        y += SECTION_GAP;
    }
    if visible.opacity {
        y += SECTION_HEADER_HEIGHT;
        // Half-width Layer-opacity row — matches `paint_layer_section`.
        rects.push((
            PropertyFocus::Opacity,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(usable_w / 2.0 - 4.0, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 12.0;
        y += SECTION_GAP;
    }
    if visible.fill {
        y += SECTION_HEADER_HEIGHT;
        // FillOpacity input — the head row's `100 %` box, sitting
        // to the right of the fill-type dropdown. Geometry mirrors
        // `paint_fill_section`'s `pct_rect`.
        rects.push((
            PropertyFocus::FillOpacity,
            Rect {
                origin: Point2D::new(x0 + w - PAD_X - 78.0, y),
                size: Point2D::new(50.0, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 6.0;
        if matches!(visible.fill_type, FillType::Solid) {
            rects.push((
                PropertyFocus::FillHex,
                Rect {
                    origin: Point2D::new(x0 + PAD_X, y),
                    size: Point2D::new(usable_w, INPUT_HEIGHT),
                },
            ));
        }
        y += fill_body_height(visible.fill_type) - 6.0;
        y += 12.0;
        y += SECTION_GAP;
    }
    if visible.stroke {
        y += SECTION_HEADER_HEIGHT;
        let stroke_width_w = 60.0;
        let stroke_hex_w = usable_w - stroke_width_w - 8.0;
        rects.push((
            PropertyFocus::StrokeHex,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(stroke_hex_w, INPUT_HEIGHT),
            },
        ));
        rects.push((
            PropertyFocus::StrokeWidth,
            Rect {
                origin: Point2D::new(x0 + PAD_X + stroke_hex_w + 8.0, y),
                size: Point2D::new(stroke_width_w, INPUT_HEIGHT),
            },
        ));
    }
    rects
}
