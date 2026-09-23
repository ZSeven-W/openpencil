//! Footer toolbar geometry and paint for the AI chat panel.
//!
//! Computes the bottom single-row toolbar rects:
//!   model pill (LEFT) | library | 🧠 thinking | ⚡ parallel-agents | attach |
//!   send (RIGHT)
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
    chat_neutral_feedback_color, AIChatPlaceholder, FooterLayout, INPUT_TOOLBAR_HEIGHT, PAD,
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
/// Width of the Prompt Center library button.
pub(crate) const FOOTER_PROMPT_W: f32 = 24.0;
/// Width of the thinking-mode toggle — same bare-icon slot as attach, so
/// the right cluster keeps one rhythm.
pub(crate) const FOOTER_THINKING_W: f32 = 24.0;
/// Width of the image-generation toggle — another bare-icon slot in the
/// same rhythm as thinking/attach.
pub(crate) const FOOTER_IMAGE_GEN_W: f32 = 24.0;
/// Narrowest the model pill may be squeezed to before the row drops the
/// thinking toggle instead. A pill this size still shows a readable model
/// name; below it the user cannot tell which model is selected, and an
/// unreadable model chip is a worse trade than a hidden thinking toggle
/// (which keeps its per-tab default either way).
const FOOTER_MODEL_PILL_MIN_W: f32 = 72.0;

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

        // Left anchor — model pill starts at PAD. Its width contracts at
        // the minimum chat width so the new library button never overlaps
        // the fixed right-hand cluster.
        let model_x = rect.origin.x + PAD;
        let model_h = 28.0;

        // Right cluster (#38/#42 layout) — library | ⚡ | 📎 | send,
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

        // Image-generation toggle (immediately left of ⚡) and thinking-mode
        // toggle (immediately left of that): both answer "how does the next
        // turn run". A row too narrow for everything degrades instead of
        // squeezing the model pill below its floor — and the image-gen
        // toggle pays first: it is the row's newest, most optional control,
        // so the thinking toggle keeps its "survives down to the minimum
        // panel width" contract. Both drop to zero-width rects.
        let cluster_room = speed_x - FOOTER_GAP - FOOTER_PROMPT_W - FOOTER_GAP - model_x;
        let both_fit =
            cluster_room - FOOTER_GAP - FOOTER_IMAGE_GEN_W - FOOTER_GAP - FOOTER_THINKING_W
                >= FOOTER_MODEL_PILL_MIN_W;
        let thinking_fits =
            cluster_room - FOOTER_GAP - FOOTER_THINKING_W >= FOOTER_MODEL_PILL_MIN_W;
        let (image_gen_w, thinking_w) = if both_fit {
            (FOOTER_IMAGE_GEN_W, FOOTER_THINKING_W)
        } else if thinking_fits {
            (0.0, FOOTER_THINKING_W)
        } else {
            (0.0, 0.0)
        };
        let image_gen_x = speed_x - FOOTER_GAP - image_gen_w;
        let image_gen = Rect::xywh(
            image_gen_x,
            cy - FOOTER_IMAGE_GEN_W / 2.0,
            image_gen_w,
            FOOTER_IMAGE_GEN_W,
        );

        let thinking_x =
            image_gen_x - if image_gen_w > 0.0 { FOOTER_GAP } else { 0.0 } - thinking_w;
        let thinking = Rect::xywh(
            thinking_x,
            cy - FOOTER_THINKING_W / 2.0,
            thinking_w,
            FOOTER_THINKING_W,
        );

        // Prompt Center — immediately left of the thinking toggle.
        let prompt_x =
            thinking_x - if thinking_w > 0.0 { FOOTER_GAP } else { 0.0 } - FOOTER_PROMPT_W;
        let prompt_center = Rect::xywh(
            prompt_x,
            cy - FOOTER_PROMPT_W / 2.0,
            FOOTER_PROMPT_W,
            FOOTER_PROMPT_W,
        );

        let model_w = FOOTER_MODEL_PILL_W.min((prompt_x - FOOTER_GAP - model_x).max(0.0));
        let model = Rect::xywh(model_x, cy - model_h / 2.0, model_w, model_h);

        // Agent-team chip — zero-width logical rect kept for schema compat.
        // Zero width does NOT make it unhittable: `Rect::contains` is
        // inclusive on both edges, so this rect still owns the pixel column
        // at `model_x + model_w`. The hit-test and hover paths guard on
        // `size.x > 0.0` explicitly; do not drop those guards on the belief
        // that the geometry alone is inert.
        let agent_team = Rect::xywh(model_x + model_w, cy - 11.0, 0.0, 22.0);

        FooterLayout {
            model,
            prompt_center,
            thinking,
            image_gen,
            speed,
            agent_team,
            attach,
            stop,
            send,
        }
    }
}

/// Ellipsize a footer chip label to `max_w`.
///
/// Measured through the backend in the family the run paints with, NOT
/// through [`footer_label_width`]. That estimator hard-codes the bundled
/// Roboto's 0.55-em ASCII advance, which is narrower than the `system-ui`
/// face this label is drawn in — fitting against it returns a string the
/// widget believes fits, and the chip's clip then shears the last glyph in
/// half with no ellipsis to show for it. `footer_label_width` keeps its
/// backend-free callers (chip geometry, computed before a painter exists).
///
/// [`footer_label_width`]: super::ai_chat_panel::footer_label_width
pub(crate) fn fit_footer_label(
    backend: &mut dyn crate::RenderBackend,
    label: &str,
    size: f32,
    max_w: f32,
) -> String {
    crate::widgets::text_metrics::fit_chrome(backend, label, max_w, size)
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
/// Draws: model pill | library | 🧠 thinking | ⚡ parallel-agents | 📎 attach |
/// ↑ send
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
    let model_name_fit = fit_footer_label(cx.backend, model_name, 11.0, label_w);
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

    // --- Prompt Center library button ---
    let prompt_hover = widget.footer_hover == Some(ChatFooterButton::PromptCenter)
        || widget.footer_pressed == Some(ChatFooterButton::PromptCenter);
    if prompt_hover {
        cx.backend.fill_round_rect(
            footer.prompt_center,
            6.0,
            chat_neutral_feedback_color(
                &widget.theme,
                widget.footer_pressed == Some(ChatFooterButton::PromptCenter),
            ),
        );
    }
    let prompt_icon_offset = (FOOTER_PROMPT_W - 14.0) / 2.0;
    draw_icon(
        cx.backend,
        Icon::Library,
        Point2D::new(
            footer.prompt_center.origin.x + prompt_icon_offset,
            footer.prompt_center.origin.y + prompt_icon_offset,
        ),
        14.0,
        widget.theme.muted_foreground,
        1.4,
    );

    // --- Thinking-mode toggle — 🧠, left of the ⚡ chip ---
    crate::widgets::ai_chat_thinking_toggle::paint_thinking_toggle(
        cx,
        &widget.theme,
        footer.thinking,
        widget.state.thinking_mode,
        widget.footer_hover == Some(ChatFooterButton::ThinkingMode),
        widget.footer_pressed == Some(ChatFooterButton::ThinkingMode),
    );

    // --- Image-generation toggle — 🖼, left of the ⚡ chip ---
    // Unconfigured paints as disabled (faded; the press routes to the
    // Images settings tab). Configured+on uses the theme accent; the
    // deliberate-off state carries the same slash the thinking toggle
    // uses, so "off" never rests on a muted-vs-muted colour step alone.
    let image_gen_hover = widget.footer_hover == Some(ChatFooterButton::ImageGen)
        || widget.footer_pressed == Some(ChatFooterButton::ImageGen);
    if footer.image_gen.size.x > 0.0 {
        if image_gen_hover {
            cx.backend.fill_round_rect(
                footer.image_gen,
                6.0,
                chat_neutral_feedback_color(
                    &widget.theme,
                    widget.footer_pressed == Some(ChatFooterButton::ImageGen),
                ),
            );
        }
        let (image_gen_color, image_gen_slash) = if !widget.image_gen_available {
            ((widget.theme.muted_foreground).with_alpha(0.38), false)
        } else if widget.image_gen_enabled {
            (widget.theme.primary, false)
        } else {
            (widget.theme.muted_foreground, true)
        };
        let image_gen_icon_size = 14.0;
        let image_gen_origin = Point2D::new(
            footer.image_gen.origin.x + (footer.image_gen.size.x - image_gen_icon_size) / 2.0,
            footer.image_gen.origin.y + (footer.image_gen.size.y - image_gen_icon_size) / 2.0,
        );
        draw_icon(
            cx.backend,
            Icon::Image,
            image_gen_origin,
            image_gen_icon_size,
            image_gen_color,
            1.4,
        );
        if image_gen_slash {
            cx.backend.stroke_line(
                Point2D::new(
                    image_gen_origin.x + 1.0,
                    image_gen_origin.y + image_gen_icon_size - 1.0,
                ),
                Point2D::new(
                    image_gen_origin.x + image_gen_icon_size - 1.0,
                    image_gen_origin.y + 1.0,
                ),
                image_gen_color,
                1.4,
            );
        }
    }

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
