//! PropertyPanel draft lifecycle — both ends of it.
//!
//! * `property_focus_initial` seeds the shared `property_input` draft
//!   from the panel snapshot when a row takes focus.
//! * `commit_*_focus` is the blur / Enter path that turns that draft
//!   back into a document write.
//!
//! Both hosts carried byte-identical copies of all three in
//! `widget_host/property_input_dispatch.rs` / `press_helpers.rs`. They
//! are pure work over the panel's `PropertyFocus` vocabulary, the
//! `PropertyPanel` snapshot, and this crate's hex / number helpers, so
//! they live here; the hosts keep only the surrounding glue (the
//! variables-panel commits that run first, and `mark_dirty`).
//!
//! Keeping seed and commit in one module is deliberate: several rows
//! rely on the two agreeing (e.g. `StrokeHex` seeds exactly the colour
//! the swatch paints so focusing an unset stroke doesn't flip it to
//! `#000000`), and that invariant is easiest to hold when both halves
//! are read side by side.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PropertyFocus};

use crate::util::{
    color_to_hex, color_to_hex_with_alpha, format_panel_number, format_panel_number_roundtrip,
    parse_hex_color,
};
use crate::widgets::{property_panel_corner, property_panel_fill, PropertyPanel};
use crate::Color;

/// Seed the property-input draft from the panel snapshot for the
/// freshly-focused `PropertyFocus` row.
pub fn property_focus_initial(focus: PropertyFocus, panel: &PropertyPanel) -> String {
    use PropertyFocus as F;
    match focus {
        F::PageBackgroundHex => panel.page_background.clone().unwrap_or_default(),
        F::PositionX => panel.snapshot.x.to_string(),
        F::PositionY => panel.snapshot.y.to_string(),
        F::SizeW => panel.snapshot.width.to_string(),
        F::SizeH => panel.snapshot.height.to_string(),
        F::LayoutGap => format_panel_number(panel.snapshot.layout_gap),
        F::PaddingTop | F::PaddingRight | F::PaddingBottom | F::PaddingLeft => panel
            .snapshot
            .layout_padding
            .value_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        F::Rotation => (panel.snapshot.rotation_deg.round() as i32).to_string(),
        F::PositionR => {
            if property_panel_corner::radii_are_uniform(panel.snapshot.corner_radii) {
                (panel.snapshot.corner_radius.round() as i32).to_string()
            } else {
                String::new()
            }
        }
        F::CornerTL | F::CornerTR | F::CornerBL | F::CornerBR => {
            property_panel_corner::value_for_focus(panel.snapshot.corner_radii, focus)
                .map(format_panel_number)
                .unwrap_or_else(|| "0".to_string())
        }
        F::EffectRadius(index) => panel
            .snapshot
            .effects
            .get(index)
            .map(|effect| format_panel_number(effect.blur))
            .unwrap_or_else(|| "0".to_string()),
        F::Opacity => format_panel_number(panel.snapshot.opacity_percent),
        F::PolygonSides => panel.snapshot.polygon_sides.unwrap_or(3).to_string(),
        F::EllipseStart => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.start_deg)
                .unwrap_or(0.0),
        ),
        F::EllipseSweep => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.sweep_deg)
                .unwrap_or(360.0),
        ),
        F::EllipseInnerRadius => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.inner_percent)
                .unwrap_or(0.0),
        ),
        F::FontSize => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.font_size))
            .unwrap_or_else(|| "16".to_string()),
        F::FontWeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| t.font_weight.to_string())
            .unwrap_or_else(|| "400".to_string()),
        F::LineHeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.line_height_percent))
            .unwrap_or_else(|| "120".to_string()),
        F::LetterSpacing => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.letter_spacing))
            .unwrap_or_else(|| "0".to_string()),
        // Widget-section fields — seed the input draft from the selected
        // widget's current value so editing starts from it.
        F::WidgetPlaceholder => widget_text(panel, |w| &w.placeholder),
        F::WidgetValue => widget_text(panel, |w| &w.value),
        F::WidgetLabel => widget_text(panel, |w| &w.label),
        F::WidgetLeadingIcon => widget_text(panel, |w| &w.leading_icon),
        F::WidgetTrailingIcon => widget_text(panel, |w| &w.trailing_icon),
        F::WidgetBindKey => widget_text(panel, |w| &w.bind_key),
        F::WidgetMin => widget_text(panel, |w| &w.min),
        F::WidgetMax => widget_text(panel, |w| &w.max),
        F::WidgetStep => widget_text(panel, |w| &w.step),
        F::FillOpacity(index) => {
            let opacity = panel
                .snapshot
                .fills
                .get(index)
                .map(|f| f.opacity)
                .unwrap_or(panel.snapshot.fill_opacity);
            ((opacity * 100.0).round() as i32).to_string()
        }
        F::ImageTileScale => panel
            .snapshot
            .image_fill
            .as_ref()
            .and_then(|image| image.tile_scale)
            .map(format_panel_number_roundtrip)
            .unwrap_or_else(|| "1".to_string()),
        F::FillHex(index) => panel
            .snapshot
            .fills
            .get(index)
            .map(|f| f.color)
            .or(panel.snapshot.fill)
            .map(color_to_hex_with_alpha)
            .unwrap_or_else(|| "#FFFFFF".to_string()),
        // Seed the SAME color the stroke swatch paints (the real stroke
        // when set, else the slate placeholder) so clicking the hex input
        // doesn't flip it to #000000.
        F::StrokeHex => color_to_hex(panel.snapshot.stroke_swatch_color()),
        // Seed the SAME width the inline input paints (0 when unset, and
        // un-rounded) so clicking in never changes the displayed value —
        // mirrors the `stroke_swatch_color` seed-matches-paint invariant.
        F::StrokeWidth => {
            format_panel_number(panel.snapshot.stroke.map(|s| s.width).unwrap_or(0.0))
        }
        F::StrokeTopWidth | F::StrokeRightWidth | F::StrokeBottomWidth | F::StrokeLeftWidth => {
            panel
                .snapshot
                .stroke_side_width_for(focus)
                .map(format_panel_number)
                .unwrap_or_else(|| "0".to_string())
        }
        F::GradientAngle => {
            let a = panel.snapshot.gradient_angle.unwrap_or(0.0);
            if a.fract() == 0.0 {
                format!("{}", a as i32)
            } else {
                format!("{a}")
            }
        }
        F::GradientStopHex(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            // Strip alpha so the input pill matches what paint shows.
            // Per-stop transparency rides through commit invisibly.
            .map(|s| property_panel_fill::stop_hex_rgb_only(&s.hex))
            .unwrap_or_else(|| "#000000".to_string()),
        F::GradientStopOffset(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| ((s.offset * 100.0).round() as i32).to_string())
            .unwrap_or_else(|| "0".to_string()),
    }
}

/// Read one text field off the selected widget's snapshot, defaulting to
/// the empty draft when the selection carries no widget section.
fn widget_text<F>(panel: &PropertyPanel, field: F) -> String
where
    F: FnOnce(&crate::widgets::property_panel_snapshot::WidgetSummary) -> &String,
{
    panel
        .snapshot
        .widget
        .as_ref()
        .map(|w| field(w).clone())
        .unwrap_or_default()
}

/// Commit a pending effect-parameter edit (the Effects section's
/// editable value box). Parses the shared draft and writes it via
/// `SetEffectParam`; a non-numeric draft is discarded. Returns `true`
/// when a focus was taken, i.e. the host should mark dirty.
pub fn commit_effect_param_focus(state: &mut EditorState) -> bool {
    let Some(focus) = state.editor_ui.effect_param_focus.take() else {
        return false;
    };
    state.ui.property_draft_select_all = false;
    let draft = state.ui.property_input.text().to_owned();
    state.ui.property_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_caret_pos = 0;
    if let Ok(value) = draft.trim().parse::<f32>() {
        if value.is_finite() {
            let id = state.selection.anchor.clone();
            if id.is_real() {
                // Instance-write redirect (GAP #10) — see
                // `apply_property_action` for the choke-point note.
                let instance_scope = state.begin_instance_write_for_anchor();
                state.commit_history();
                let _ = state.apply(op_editor_core::EditorCommand::SetEffectParam {
                    node_id: id,
                    index: focus.effect as u32,
                    field: focus.field,
                    value,
                });
                if let Some(scope) = instance_scope {
                    state.finish_instance_write(scope);
                }
            }
        }
    }
    true
}

/// Drop an effect-parameter draft without applying it.
///
/// Collaboration gates use this after a role/phase change so a draft that was
/// focused while editable cannot commit after the session becomes read-only.
pub fn discard_effect_param_focus(state: &mut EditorState) -> bool {
    if state.editor_ui.effect_param_focus.take().is_none() {
        return false;
    }
    clear_property_draft(state);
    true
}

/// Drop a PropertyPanel draft without applying it.
pub fn discard_property_focus(state: &mut EditorState) -> bool {
    if state.ui.property_focus.take().is_none() {
        return false;
    }
    clear_property_draft(state);
    true
}

fn clear_property_draft(state: &mut EditorState) {
    state.ui.property_draft_select_all = false;
    state.ui.property_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_caret_pos = 0;
}

/// Commit the focused property input. Returns `true` when a focus was
/// taken, i.e. the host should mark dirty.
///
/// The caller commits the variables-panel header / row drafts and the
/// effect-param draft first — those live host-side because they route
/// through host-owned panel state.
pub fn commit_property_focus(state: &mut EditorState) -> bool {
    let Some(focus) = state.ui.property_focus.take() else {
        return false;
    };
    state.ui.property_draft_select_all = false;
    let draft = state.ui.property_input.text().to_owned();
    state.ui.property_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_caret_pos = 0;
    // Instance-write redirect (GAP #10) — see `apply_property_action`
    // for the choke-point note.
    let before = state.snapshot_for_history();
    let instance_scope = state.begin_instance_write_for_anchor();
    match focus {
        PropertyFocus::PageBackgroundHex => {
            let authored = draft.trim();
            // A page with no authored background seeds an empty draft.
            // Empty/unchanged blur is deliberately a no-op; clearing is
            // an explicit panel action so focus alone cannot inflate an
            // old document with a new background field.
            if state.active_page_background_color() != Some(authored) {
                if let Some(hex) = normalized_page_background_hex(authored) {
                    let _ = state.set_active_page_background_color(Some(hex));
                }
            }
        }
        PropertyFocus::ImageTileScale => {
            if let Ok(value) = draft.trim().parse::<f32>() {
                let _ = state.set_selected_image_tile_scale(value);
            }
        }
        PropertyFocus::FillHex(index) => {
            let stripped = draft.trim().trim_start_matches('#');
            if !stripped.is_empty() {
                if let Some(color) = parse_hex_color(draft.trim()) {
                    let hex = color_to_hex_with_alpha(color);
                    // The primary fill (index 0) keeps `set_selected_color`
                    // (prepends a solid + colour-variable-aware); a
                    // non-primary row writes its own solid fill by index.
                    if index == 0 {
                        let _ = state.set_selected_color(true, &hex);
                    } else {
                        let _ = state.set_selected_fill_hex_at(index, &hex);
                    }
                }
            }
        }
        PropertyFocus::StrokeHex => {
            let stripped = draft.trim().trim_start_matches('#');
            if !stripped.is_empty() {
                if let Some(color) = parse_hex_color(draft.trim()) {
                    let _ = state.set_selected_color(false, &color_to_hex(color));
                }
            }
        }
        PropertyFocus::GradientStopHex(index) => {
            let stripped = draft.trim().trim_start_matches('#');
            if !stripped.is_empty() {
                if let Some(color) = parse_hex_color(draft.trim()) {
                    // The input pill never paints alpha digits, so
                    // re-attach the stop's existing alpha here — a
                    // transparent stop must stay transparent after the
                    // user edits its RGB.
                    let existing_alpha = state
                        .selected_node()
                        .and_then(|n| current_stop_alpha(n, index))
                        .unwrap_or(1.0);
                    let with_alpha = Color {
                        r: color.r,
                        g: color.g,
                        b: color.b,
                        a: existing_alpha,
                    };
                    let _ = state.set_selected_gradient_stop_hex(
                        index,
                        &color_to_hex_with_alpha(with_alpha),
                    );
                }
            }
        }
        PropertyFocus::WidgetPlaceholder => {
            let _ = state.set_selected_widget_text(
                op_editor_core::WidgetTextField::Placeholder,
                draft.trim(),
            );
        }
        PropertyFocus::WidgetValue => {
            let _ = state
                .set_selected_widget_text(op_editor_core::WidgetTextField::Value, draft.trim());
        }
        PropertyFocus::WidgetLabel => {
            let _ = state
                .set_selected_widget_text(op_editor_core::WidgetTextField::Label, draft.trim());
        }
        PropertyFocus::WidgetLeadingIcon => {
            let _ = state.set_selected_widget_text(
                op_editor_core::WidgetTextField::LeadingIcon,
                draft.trim(),
            );
        }
        PropertyFocus::WidgetTrailingIcon => {
            let _ = state.set_selected_widget_text(
                op_editor_core::WidgetTextField::TrailingIcon,
                draft.trim(),
            );
        }
        PropertyFocus::WidgetBindKey => {
            let _ = state.set_selected_widget_bind_value(draft.trim());
        }
        _ => {
            if let Ok(value) = draft.trim().parse::<f32>() {
                let _ = state.commit_property_edit(focus, value);
            }
        }
    }
    if let Some(scope) = instance_scope {
        state.finish_instance_write(scope);
    }
    if state.snapshot_for_history() != before {
        state.history_push_past(before);
    }
    true
}

/// Read the live alpha of gradient stop `index` on `node`, parsed out
/// of the canonical hex (8-char `#RRGGBBAA`). `None` when the first
/// fill isn't a gradient or the stop hex omits alpha — the caller
/// defaults to `1.0` in that case so opaque stops stay opaque through
/// an RGB edit.
pub fn current_stop_alpha(node: &PenNode, index: usize) -> Option<f32> {
    use jian_ops_schema::style::PenFill;
    let first = op_editor_core::fills::node_fills(node).and_then(|f| f.first())?;
    let stops = match first {
        PenFill::LinearGradient(b) => &b.stops,
        PenFill::RadialGradient(b) => &b.stops,
        _ => return None,
    };
    let hex = &stops.get(index)?.color;
    Some(op_editor_core::parse_hex_alpha(hex))
}

/// Validate and canonicalize an authored page colour without routing it
/// through the RGB-only helper (which would discard an imported alpha
/// byte).
fn normalized_page_background_hex(value: &str) -> Option<String> {
    let digits = value.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", digits.to_ascii_uppercase()))
}
