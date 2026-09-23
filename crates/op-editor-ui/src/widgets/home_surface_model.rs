//! Model-access chrome on the Home composer: the model button on the
//! submit row, the label it derives from the chat selection, and the
//! geometry of the Home-anchored chat model picker that opens above it.

use super::{fade, HomeLayout, HomeSurface, StudioPalette};
use crate::widgets::ai_chat_model_picker::picker_view_height;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::{EditorState, ModelEntry};

/// Model button height — the submit row's quieter left action.
pub const MODEL_CHIP_H: f32 = 38.0;
/// Leading pad + the 7 px status dot + its 7 px gap to the label.
pub const MODEL_CHIP_PREFIX_W: f32 = 22.0;
/// Trailing zone: 12 px gap + the 15 px chevron + right pad.
pub const MODEL_CHIP_CHEVRON_W: f32 = 33.0;
/// Widest the button may grow before the label clips inside it, so the
/// primary 开始设计 button next to it keeps its clearance.
pub const MODEL_CHIP_MAX_W: f32 = 240.0;

/// Width of the Home-anchored model picker card.
pub const HOME_MODEL_PICKER_W: f32 = 300.0;
/// The card floats this far above the button.
pub const HOME_MODEL_PICKER_GAP: f32 = 8.0;
/// The trailing "接入更多模型…" action row hangs below the card.
pub const CONNECT_MORE_ROW_H: f32 = 34.0;
pub const CONNECT_MORE_ROW_GAP: f32 = 6.0;

/// The label the Home model button shows. Reuses the exact derivation
/// the chat panel's bottom-left model pill paints
/// (`selected_model_entry`'s display name); when no agent can answer it
/// becomes the localized connect hint instead.
pub fn model_chip_label(state: &EditorState) -> String {
    let entry = state
        .has_usable_chat_agent()
        .then(|| state.chat.selected_model_entry())
        .flatten();
    match entry {
        Some(entry) => entry.display_name.clone(),
        None => op_i18n::translate(state.editor_ui.locale, "home.connect.chipEmpty").to_string(),
    }
}

/// Natural button width for `label` (prefix + measured label + chevron),
/// clamped so an extreme model name cannot swallow the submit row.
pub fn model_chip_width(label: &str) -> f32 {
    let label_w = crate::widgets::ai_chat_panel::footer_label_width(label, 12.0);
    (MODEL_CHIP_PREFIX_W + label_w + MODEL_CHIP_CHEVRON_W).min(MODEL_CHIP_MAX_W)
}

/// The picker card anchored above the submit row's model button plus
/// the trailing connect-more row under it. `None` when the button is
/// not laid out (zero-width) or the viewport cannot hold the card.
/// The Home-anchored model picker's card.
///
/// It used to return a second rect for an external 接入更多模型 row
/// painted BELOW the card. The picker carries that action as its own
/// footer now, so the external row was the same thing twice — and
/// because it sat outside the card, crossing the gap to reach it read as
/// leaving the popover.
pub fn home_model_picker_rects(
    layout: &HomeLayout,
    viewport_w: f32,
    models: &[ModelEntry],
    search: &str,
) -> Option<Rect> {
    let chip = layout.model_chip;
    if chip.size.x <= 0.0 {
        return None;
    }
    let height = picker_view_height(models, search);
    let bottom = chip.origin.y - HOME_MODEL_PICKER_GAP;
    let top = (bottom - height).max(8.0);
    let x = (chip.origin.x + chip.size.x / 2.0 - HOME_MODEL_PICKER_W / 2.0)
        .clamp(8.0, (viewport_w - HOME_MODEL_PICKER_W - 8.0).max(8.0));
    Some(Rect::xywh(x, top, HOME_MODEL_PICKER_W, height))
}

/// Paint the model button: a 38 px outline pill with the green status
/// dot, the 12 px label, and a trailing chevron. The empty state (no
/// usable agent) drops the dot's green for grey.
pub(super) fn paint_model_chip(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    palette: StudioPalette,
) {
    let usable = surface.usable_agent;
    let label = surface.chip_label.as_str();
    let hovered = surface.state.hover == Some(op_editor_core::HomeHit::ModelChip);
    let pressed = surface.state.pressed == Some(op_editor_core::HomeHit::ModelChip);
    let fill = if hovered || pressed {
        palette.button_hover
    } else {
        palette.raised
    };
    cx.backend.fill_round_rect(rect, 9.0, fill);
    cx.backend.stroke_round_rect(
        rect,
        9.0,
        if hovered {
            palette.button_hover_line
        } else {
            palette.raised_line
        },
        1.0,
    );
    let dot_color = if usable {
        palette.status_green
    } else {
        fade(palette.muted, 0.55)
    };
    cx.backend.fill_oval(
        Rect::xywh(
            rect.origin.x + 12.0,
            rect.origin.y + (rect.size.y - 7.0) / 2.0,
            7.0,
            7.0,
        ),
        dot_color,
    );
    cx.backend.save();
    cx.backend.clip_rect(rect);
    let label_color = if usable { palette.ink } else { palette.blue };
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        label_color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            rect.origin.x + MODEL_CHIP_PREFIX_W,
            jian_widgets::centered_text_baseline_y(rect, 12.0),
        ),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(
            rect.origin.x + rect.size.x - 21.0,
            rect.origin.y + (rect.size.y - 15.0) / 2.0,
        ),
        15.0,
        palette.muted,
        1.6,
    );
    cx.backend.restore();
}

/// Paint the picker's trailing "接入更多模型…" row — the card-styled
/// action under the dropdown that routes to the Agents settings tab.
pub fn paint_connect_more_row(
    cx: &mut PaintCx<'_>,
    theme: &crate::theme::Theme,
    rect: Rect,
    label: &str,
    hovered: bool,
) {
    cx.backend.fill_round_rect(rect, 10.0, theme.card);
    if hovered {
        cx.backend.fill_round_rect(rect, 10.0, theme.muted);
    }
    cx.backend.stroke_round_rect(rect, 10.0, theme.border, 1.0);
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(rect.origin.x + 10.0, rect.origin.y + 10.0),
        13.0,
        theme.muted_foreground,
        1.4,
    );
    let text = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        theme.muted_foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &text,
        Point2D::new(rect.origin.x + 30.0, rect.origin.y + 22.0),
    );
}
