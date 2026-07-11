//! Footer toolbar geometry and paint for the AI chat panel.
//!
//! Computes the bottom single-row toolbar rects:
//!   model pill (LEFT) | [gap] | ⚡ parallel-agents chip | attach | send (RIGHT)
//!
//! As of #38 the ⚡/📎/🎨 cluster moved from the LEFT (between model and gap) to
//! the RIGHT. As of #42 the cluster sits snug against the single send/stop
//! circle (no reserved stop gap). The model pill remains at PAD.
//!
//! The `stop` rect shares the `send` slot — the circle toggles send↑ ↔ stop◻
//! in place. The caller paints stop while streaming and send otherwise; the
//! hit-test checks `streaming && stop` before `send` so the same rect routes
//! to the right action.
//!
//! The ⚡ chip is the "PARALLEL AGENTS" chip (#32): shows `"{N}x"` in gold
//! where N = `ChatState::agent_team_size` (1–6). Clicking it opens a small
//! overlay listing 1x–6x; selecting a row sets the multiplier.
//!
//! Note: `effort_level` / `cycle_effort_level` remain on `ChatState` for
//! future use but are no longer wired to a footer chip as of #32.

use super::ai_chat_panel::{
    chat_neutral_feedback_color, footer_label_width, AIChatPlaceholder, FooterLayout,
    INPUT_TOOLBAR_HEIGHT, PAD,
};
use crate::widgets::ai_chat_panel_controls::draw_label;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

/// Width of the model-picker pill. Sized to fit typical model names;
/// very long names truncate with "…" inside the pill.
pub(crate) const FOOTER_MODEL_PILL_W: f32 = 150.0;

/// Width of the parallel-agents chip — ⚡ icon (12px) + "Nx" label.
/// "6x" is the longest label (~16px at 11pt) + icon(12) + left-pad(4) + gap(3) = 35px.
/// Round up to 40px for comfortable click target.
pub(crate) const FOOTER_SPEED_W: f32 = 40.0;

/// Number of rows in the Parallel Agents picker (1x..6x).
pub(crate) const PARALLEL_AGENTS_COUNT: u32 = 6;
/// Height of each row in the Parallel Agents picker overlay.
/// Exported as `_PUB` so the hit-test module can use it without a re-export dance.
pub(crate) const PARALLEL_AGENTS_ROW_H_PUB: f32 = 28.0;
const PARALLEL_AGENTS_ROW_H: f32 = PARALLEL_AGENTS_ROW_H_PUB;
/// Width of the Parallel Agents picker overlay.
const PARALLEL_AGENTS_PICKER_W: f32 = 130.0;

/// Width of the bare-icon buttons (attach) in the toolbar.
pub(crate) const FOOTER_ICON_W: f32 = 24.0;

/// Diameter of the circular send/stop buttons.
pub(crate) const FOOTER_CIRCLE_D: f32 = 28.0;

/// Gap between consecutive toolbar items.
const FOOTER_GAP: f32 = 4.0;

impl<'a> AIChatPlaceholder<'a> {
    pub(crate) fn footer_layout(
        &self,
        rect: Rect,
        _input_rect: Rect,
        toolbar_top: f32,
    ) -> FooterLayout {
        let toolbar_center_y = toolbar_top + INPUT_TOOLBAR_HEIGHT / 2.0;
        let cy = toolbar_center_y;

        // Left anchor — model pill stays at PAD (unchanged by #38).
        let model_x = rect.origin.x + PAD;
        let model_h = 28.0;
        let model = Rect::xywh(model_x, cy - model_h / 2.0, FOOTER_MODEL_PILL_W, model_h);

        // Agent-team chip — zero-width logical rect for schema compat; contains() = false.
        let agent_team = Rect::xywh(model_x + FOOTER_MODEL_PILL_W, cy - 11.0, 0.0, 22.0);

        // Right cluster (#38/#42 layout) — ⚡ chip | 📎 attach | send,
        // laid out right-to-left from right_edge (stop shares the send slot).
        let right_edge = rect.origin.x + rect.size.x - PAD;

        // Send circle — rightmost.
        let send = Rect::xywh(
            right_edge - FOOTER_CIRCLE_D,
            cy - FOOTER_CIRCLE_D / 2.0,
            FOOTER_CIRCLE_D,
            FOOTER_CIRCLE_D,
        );

        // Stop circle — shares the send slot. The send arrow toggles to a
        // stop square in place while streaming, so the icon cluster sits snug
        // against the single circle with no reserved gap between 🎨 and send (#42).
        let stop = send;

        // Attach icon — immediately left of the send/stop circle. (The
        // palette slot is removed until the feature behind it exists.)
        let attach_x = send.origin.x - FOOTER_GAP - FOOTER_ICON_W;
        let attach = Rect::xywh(
            attach_x,
            cy - FOOTER_ICON_W / 2.0,
            FOOTER_ICON_W,
            FOOTER_ICON_W,
        );

        // Parallel-agents chip (⚡ + "Nx") — immediately left of attach.
        let speed_x = attach_x - FOOTER_GAP - FOOTER_SPEED_W;
        let speed_h = 22.0;
        let speed = Rect::xywh(speed_x, cy - speed_h / 2.0, FOOTER_SPEED_W, speed_h);

        FooterLayout {
            model,
            speed,
            agent_team,
            attach,
            stop,
            send,
        }
    }
}

pub(crate) fn fit_footer_label(label: &str, size: f32, max_w: f32) -> String {
    if footer_label_width(label, size) <= max_w {
        return label.to_string();
    }
    let ellipsis_w = footer_label_width("…", size);
    let budget = (max_w - ellipsis_w).max(0.0);
    let mut out = String::new();
    let mut w = 0.0;
    for ch in label.chars() {
        let next = footer_label_width(&ch.to_string(), size);
        if w + next > budget {
            break;
        }
        out.push(ch);
        w += next;
    }
    out.push('…');
    out
}

pub(crate) fn footer_label_baseline(center_y: f32, size: f32) -> f32 {
    center_y + size * 0.35
}

/// Short display label for the effort level.
///
/// Effort level is no longer wired to the footer chip as of #32 (the ⚡ chip
/// was repurposed to the Parallel Agents multiplier). This function is kept for
/// any future affordance that surfaces the effort level.
#[allow(dead_code)]
pub(crate) fn effort_speed_label(level: op_editor_core::chat::EffortLevel) -> &'static str {
    use op_editor_core::chat::EffortLevel;
    match level {
        EffortLevel::Low => "Low",
        EffortLevel::Medium => "Med",
        EffortLevel::High => "High",
        EffortLevel::Max => "Max",
    }
}

/// Returns the bounding rect of the Parallel Agents picker overlay anchored
/// above the speed chip. The overlay floats upward from the chip's top edge
/// with a small gap, so it never clips into the toolbar strip.
///
/// Since #38 the ⚡ chip is in the RIGHT cluster. The picker right-aligns to the
/// stop button's right edge so it doesn't overflow the panel boundary.
pub(crate) fn parallel_agents_picker_rect(footer: &FooterLayout) -> Rect {
    let total_h = PARALLEL_AGENTS_ROW_H * PARALLEL_AGENTS_COUNT as f32 + 32.0; // 32 = title row
    let bottom = footer.speed.origin.y - 4.0;
    // Right-align the picker to the stop button's right edge (keeps it within the panel).
    let picker_right = footer.stop.origin.x + footer.stop.size.x;
    let picker_x = picker_right - PARALLEL_AGENTS_PICKER_W;
    Rect::xywh(
        picker_x,
        bottom - total_h,
        PARALLEL_AGENTS_PICKER_W,
        total_h,
    )
}

/// Paint the Parallel Agents picker overlay above the ⚡ chip.
///
/// Draws a card with a "PARALLEL AGENTS" header and 6 rows "1x"–"6x".
/// The current value (`selected`) is highlighted. The hovered row
/// (`hover`) receives a subtle wash.
pub(crate) fn paint_parallel_agents_picker(
    cx: &mut PaintCx<'_>,
    theme: &crate::theme::Theme,
    footer: &FooterLayout,
    selected: u32,
    hover: Option<u32>,
) {
    let picker = parallel_agents_picker_rect(footer);

    // Card background + border.
    cx.backend.fill_round_rect(picker, 8.0, theme.card);
    cx.backend
        .stroke_round_rect(picker, 8.0, (theme.border).with_alpha(0.8), 1.0);

    // Title row — "PARALLEL AGENTS" in muted small caps.
    let title_x = picker.origin.x + 10.0;
    let title_y = picker.origin.y + 20.0; // baseline
    let title_label = TextLayout::single_run(
        "PARALLEL AGENTS",
        "system-ui",
        9.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&title_label, Point2D::new(title_x, title_y));

    // Rows 1x–6x.
    let rows_top = picker.origin.y + 32.0;
    for i in 1..=PARALLEL_AGENTS_COUNT {
        let row_y = rows_top + (i - 1) as f32 * PARALLEL_AGENTS_ROW_H;
        let row = Rect::xywh(
            picker.origin.x + 4.0,
            row_y,
            picker.size.x - 8.0,
            PARALLEL_AGENTS_ROW_H,
        );

        // Hover / selected highlight.
        let is_selected = i == selected;
        let is_hovered = hover == Some(i);
        if is_selected {
            cx.backend
                .fill_round_rect(row, 5.0, (theme.primary).with_alpha(0.18));
        } else if is_hovered {
            cx.backend
                .fill_round_rect(row, 5.0, (theme.muted).with_alpha(0.25));
        }

        // "Nx" label — gold when selected, muted-foreground otherwise.
        let label = format!("{}x", i);
        let label_color = if is_selected {
            theme.speed_accent
        } else {
            theme.muted_foreground
        };
        let label_baseline = row_y + PARALLEL_AGENTS_ROW_H / 2.0 + 11.0 * 0.35;
        let text = TextLayout::single_run(
            &label,
            "system-ui",
            11.0,
            label_color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&text, Point2D::new(row.origin.x + 10.0, label_baseline));

        // ⚡ icon in gold to the right for the selected row, for visual flair.
        if is_selected {
            draw_icon(
                cx.backend,
                Icon::Zap,
                Point2D::new(
                    row.origin.x + row.size.x - 20.0,
                    row_y + (PARALLEL_AGENTS_ROW_H - 12.0) / 2.0,
                ),
                12.0,
                theme.speed_accent,
                1.6,
            );
        }
    }
}

/// Paint the bottom-toolbar row of the AI chat panel (#27 / #32 layout).
///
/// Draws: model pill | [gap] | ⚡ parallel-agents chip | 📎 attach | ↑ send (◻ stop while streaming)
///
/// The ⚡ chip shows "{N}x" in gold (N = `agent_team_size`) and opens the
/// Parallel Agents picker on click.
///
/// Parameters pre-computed by the caller so this function stays
/// side-effect-free with respect to layout:
///   `footer`           — rects from `footer_layout()`
///   `toolbar_center_y` — vertical centre of the toolbar strip
///   `streaming`        — true while an assistant turn is in progress
///   `send_active`      — true when input is non-empty and a model is available
pub(crate) fn paint_bottom_toolbar(
    cx: &mut PaintCx<'_>,
    widget: &AIChatPlaceholder<'_>,
    footer: &FooterLayout,
    toolbar_center_y: f32,
    streaming: bool,
    send_active: bool,
) {
    use op_editor_core::ChatFooterButton;

    // --- Model picker pill (left anchor) ---
    cx.backend
        .fill_round_rect(footer.model, 8.0, (widget.theme.muted).with_alpha(0.3));
    cx.backend.stroke_round_rect(
        footer.model,
        8.0,
        (widget.theme.border).with_alpha(0.75),
        1.0,
    );
    if widget.footer_hover == Some(ChatFooterButton::ModelPicker)
        || widget.footer_pressed == Some(ChatFooterButton::ModelPicker)
    {
        cx.backend.fill_round_rect(
            footer.model,
            8.0,
            chat_neutral_feedback_color(
                &widget.theme,
                widget.footer_pressed == Some(ChatFooterButton::ModelPicker),
            ),
        );
    }
    cx.backend.save();
    cx.backend.clip_rect(footer.model);
    let selected = widget.state.selected_model_entry();
    let chip_color = widget.theme.muted_foreground;
    let logo_y = toolbar_center_y - 7.0;
    let logo_x = footer.model.origin.x + 8.0;
    match selected {
        Some(entry)
            if entry.builtin_provider_id.is_some() || entry.value.starts_with("builtin:") =>
        {
            crate::widgets::ai_chat_model_picker::paint_key_glyph(
                cx,
                Point2D::new(logo_x, logo_y),
                14.0,
                chip_color,
            )
        }
        Some(entry) => crate::widgets::ai_chat_model_picker::paint_provider_logo(
            cx,
            entry.provider,
            Point2D::new(logo_x, logo_y),
            14.0,
            chip_color,
        ),
        None => draw_icon(
            cx.backend,
            Icon::Sparkles,
            Point2D::new(logo_x, logo_y),
            14.0,
            chip_color,
            1.4,
        ),
    }
    let model_label_x = logo_x + 20.0;
    let model_name: &str = selected
        .map(|m| m.display_name.as_str())
        .unwrap_or(widget.label_no_models.as_str());
    // Reserve space for the chevron-down on the right of the pill.
    let label_w = (footer.model.origin.x + footer.model.size.x - 18.0 - model_label_x).max(0.0);
    let model_name_fit = fit_footer_label(model_name, 11.0, label_w);
    let model_label = TextLayout::single_run(
        &model_name_fit,
        "system-ui",
        11.0,
        (chip_color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &model_label,
        Point2D::new(model_label_x, footer_label_baseline(toolbar_center_y, 11.0)),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(
            footer.model.origin.x + footer.model.size.x - 14.0,
            toolbar_center_y - 6.0,
        ),
        12.0,
        widget.theme.muted_foreground,
        1.4,
    );
    cx.backend.restore();

    // --- Parallel-agents chip (#32) — ⚡ in gold + "{N}x" label, no background ---
    // Repurposed from the old effort/speed chip. The chip shows the current
    // agent_team_size multiplier and opens the Parallel Agents picker on click.
    let speed_hover = widget.footer_hover == Some(ChatFooterButton::SpeedChip)
        || widget.footer_pressed == Some(ChatFooterButton::SpeedChip);
    if speed_hover {
        cx.backend.fill_round_rect(
            footer.speed,
            5.0,
            chat_neutral_feedback_color(
                &widget.theme,
                widget.footer_pressed == Some(ChatFooterButton::SpeedChip),
            ),
        );
    }
    let speed_color = widget.theme.speed_accent;
    draw_icon(
        cx.backend,
        Icon::Zap,
        Point2D::new(footer.speed.origin.x + 4.0, toolbar_center_y - 6.0),
        12.0,
        speed_color,
        1.6,
    );
    // Show "{N}x" where N = agent_team_size (1–6).
    let agents_lbl = format!("{}x", widget.state.agent_team_size);
    draw_label(
        cx,
        &agents_lbl,
        11.0,
        speed_color,
        footer.speed.origin.x + 19.0,
        footer_label_baseline(toolbar_center_y, 11.0),
    );

    // --- Attach button — bare paperclip icon ---
    let attach_rect = footer.attach;
    let attach_hover = widget.footer_hover == Some(ChatFooterButton::AddAttachment)
        || widget.footer_pressed == Some(ChatFooterButton::AddAttachment);
    if attach_hover && !streaming {
        cx.backend.fill_round_rect(
            attach_rect,
            6.0,
            chat_neutral_feedback_color(
                &widget.theme,
                widget.footer_pressed == Some(ChatFooterButton::AddAttachment),
            ),
        );
    }
    let attach_icon_offset = (FOOTER_ICON_W - 12.0) / 2.0;
    draw_icon(
        cx.backend,
        Icon::Paperclip,
        Point2D::new(
            attach_rect.origin.x + attach_icon_offset,
            attach_rect.origin.y + attach_icon_offset,
        ),
        12.0,
        widget.theme.muted_foreground,
        1.4,
    );

    // --- Stop circle — shown only while a turn streams ---
    if streaming {
        let stop_rect = footer.stop;
        let stop_pressed = widget.footer_pressed == Some(ChatFooterButton::Stop);
        let stop_hovered = widget.footer_hover == Some(ChatFooterButton::Stop);
        let stop_bg = if stop_pressed {
            (widget.theme.muted).with_alpha(0.55)
        } else if stop_hovered {
            (widget.theme.muted).with_alpha(0.45)
        } else {
            (widget.theme.muted).with_alpha(0.35)
        };
        cx.backend
            .fill_round_rect(stop_rect, FOOTER_CIRCLE_D / 2.0, stop_bg);
        cx.backend.stroke_round_rect(
            stop_rect,
            FOOTER_CIRCLE_D / 2.0,
            (widget.theme.border).with_alpha(0.6),
            1.0,
        );
        // Filled red stop square at the center — the universal "recording /
        // running, press to stop" affordance; a muted 8px outline read as a
        // mystery dot (user feedback 2026-07-12).
        let stop_glyph_size = 12.0;
        let stop_red = Color::rgba_u8(0xef, 0x44, 0x44, 1.0);
        cx.backend.fill_round_rect(
            Rect {
                origin: Point2D::new(
                    stop_rect.origin.x + (FOOTER_CIRCLE_D - stop_glyph_size) / 2.0,
                    stop_rect.origin.y + (FOOTER_CIRCLE_D - stop_glyph_size) / 2.0,
                ),
                size: Point2D::new(stop_glyph_size, stop_glyph_size),
            },
            2.5,
            stop_red,
        );
    }

    // --- Send circle — shown only when NOT streaming; while a turn streams the
    //     stop circle above occupies the same slot (toggle in place, #42). ---
    if !streaming {
        let send_rect = footer.send;
        let send_pressed = widget.footer_pressed == Some(ChatFooterButton::Send);
        let send_hovered = widget.footer_hover == Some(ChatFooterButton::Send);
        let (send_bg, send_icon_color) = if send_active {
            // shadcn-style primary feedback: rest 1.0 → hover 0.9 → press 0.8
            // (the panel bg shows through the dimmed alpha as a subtle darken).
            let alpha = if send_pressed {
                0.8
            } else if send_hovered {
                0.9
            } else {
                1.0
            };
            (
                (widget.theme.primary).with_alpha(alpha),
                widget.theme.primary_foreground,
            )
        } else {
            // Disabled state — faded.
            (
                (widget.theme.muted).with_alpha(0.25),
                Color {
                    a: 0.3,
                    ..widget.theme.muted_foreground
                },
            )
        };
        cx.backend
            .fill_round_rect(send_rect, FOOTER_CIRCLE_D / 2.0, send_bg);
        // Up-arrow glyph at 12px, centered in the circle.
        let send_glyph_size = 12.0;
        draw_icon(
            cx.backend,
            Icon::ArrowUp,
            Point2D::new(
                send_rect.origin.x + (FOOTER_CIRCLE_D - send_glyph_size) / 2.0,
                send_rect.origin.y + (FOOTER_CIRCLE_D - send_glyph_size) / 2.0,
            ),
            send_glyph_size,
            send_icon_color,
            1.6,
        );
    }
}
