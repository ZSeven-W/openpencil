//! Model-picker dropdown for the AI chat panel — the upward
//! popover that lists discovered models grouped by provider.
//! Mirrors the TS `ai-chat-model-selector.tsx` `ModelDropdown`
//! (grouped rows + per-provider brand icon + selected check).
//!
//! Search is intentionally omitted in this slice — the discovered
//! catalogs are short; a search row lands with live CLI re-query.

use crate::agent_settings_state::AgentProvider;
use crate::chat_models::ModelEntry;
use crate::theme::Theme;
use crate::widgets::brand_icons::{paint_brand_logo, paint_opencode_logo, BrandLogo};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_inputs::to_jian_color;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

/// Height of a provider group-header row.
pub const MODEL_GROUP_H: f32 = 22.0;
/// Height of a single model row.
pub const MODEL_ROW_H: f32 = 28.0;
/// Vertical padding inside the dropdown card (top + bottom each).
pub const MODEL_PICKER_PAD_Y: f32 = 6.0;

/// One laid-out row in the dropdown.
enum Row {
    /// Provider group header — carries the provider for its logo.
    Header(AgentProvider),
    /// Selectable model — carries its index into the flat list.
    Model(usize),
}

/// Walk the dropdown row layout, invoking `f(row, y, height)` for
/// each row top-to-bottom starting at `top`. Paint and hit-test
/// both drive off this so they never drift apart.
fn walk_rows(models: &[ModelEntry], top: f32, mut f: impl FnMut(&Row, f32, f32)) {
    let mut y = top + MODEL_PICKER_PAD_Y;
    let mut last_provider: Option<AgentProvider> = None;
    for (idx, entry) in models.iter().enumerate() {
        if last_provider != Some(entry.provider) {
            f(&Row::Header(entry.provider), y, MODEL_GROUP_H);
            y += MODEL_GROUP_H;
            last_provider = Some(entry.provider);
        }
        f(&Row::Model(idx), y, MODEL_ROW_H);
        y += MODEL_ROW_H;
    }
}

/// Total dropdown height for `models` (group headers + rows + the
/// top/bottom padding).
pub fn picker_content_height(models: &[ModelEntry]) -> f32 {
    let mut groups = 0usize;
    let mut last: Option<AgentProvider> = None;
    for entry in models {
        if last != Some(entry.provider) {
            groups += 1;
            last = Some(entry.provider);
        }
    }
    groups as f32 * MODEL_GROUP_H
        + models.len() as f32 * MODEL_ROW_H
        + MODEL_PICKER_PAD_Y * 2.0
}

/// Map a click inside the dropdown `rect` to the index of the
/// model row under it. `None` for a click on a header / padding.
pub fn model_at(rect: Rect, point: Point2D, models: &[ModelEntry]) -> Option<usize> {
    if point.x < rect.origin.x
        || point.x > rect.origin.x + rect.size.x
        || point.y < rect.origin.y
        || point.y > rect.origin.y + rect.size.y
    {
        return None;
    }
    let mut hit = None;
    walk_rows(models, rect.origin.y, |row, y, h| {
        if let Row::Model(idx) = row {
            if point.y >= y && point.y < y + h {
                hit = Some(*idx);
            }
        }
    });
    hit
}

/// Paint the dropdown card + grouped rows. `selected` is the index
/// of the active model (gets a check mark). `rect` is the full
/// dropdown bounds as positioned by the caller.
pub fn paint_model_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    models: &[ModelEntry],
    selected: usize,
) {
    // Card background + border.
    cx.backend.fill_round_rect(rect, 10.0, theme.popover);
    cx.backend.stroke_round_rect(rect, 10.0, theme.border, 1.0);
    let row_left = rect.origin.x + 12.0;
    let row_w = rect.size.x - 12.0;
    walk_rows(models, rect.origin.y, |row, y, h| match row {
        Row::Header(provider) => {
            let logo_y = y + (h - 12.0) / 2.0;
            paint_provider_logo(
                cx,
                *provider,
                Point2D::new(row_left, logo_y),
                12.0,
                theme.muted_foreground,
            );
            let label = TextLayout::single_run(
                provider_label(*provider),
                "system-ui",
                10.0,
                to_jian_color(theme.muted_foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&label, Point2D::new(row_left + 18.0, y + h / 2.0 + 3.0));
        }
        Row::Model(idx) => {
            let is_selected = *idx == selected;
            if is_selected {
                cx.backend.fill_round_rect(
                    Rect {
                        origin: Point2D::new(rect.origin.x + 4.0, y + 1.0),
                        size: Point2D::new(rect.size.x - 8.0, h - 2.0),
                    },
                    6.0,
                    theme.muted,
                );
                draw_icon(
                    cx.backend,
                    Icon::Check,
                    Point2D::new(row_left, y + (h - 13.0) / 2.0),
                    13.0,
                    theme.foreground,
                    1.6,
                );
            }
            let color = if is_selected {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            let name = models
                .get(*idx)
                .map(|m| m.display_name.as_str())
                .unwrap_or("");
            let label = TextLayout::single_run(
                name,
                "system-ui",
                12.0,
                to_jian_color(color),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(row_left + 22.0, y + h / 2.0 + 4.0),
            );
        }
    });
    let _ = row_w;
}

/// Paint a provider's brand logo into a `size × size` square.
/// OpenCode has no single-path logo, so it routes through the
/// multi-primitive `paint_opencode_logo`.
pub fn paint_provider_logo(
    cx: &mut PaintCx<'_>,
    provider: AgentProvider,
    top_left: Point2D,
    size: f32,
    color: Color,
) {
    match provider {
        AgentProvider::ClaudeCode => {
            paint_brand_logo(cx.backend, BrandLogo::Claude, top_left, size, color)
        }
        AgentProvider::CodexCli => {
            paint_brand_logo(cx.backend, BrandLogo::OpenAI, top_left, size, color)
        }
        AgentProvider::GeminiCli => {
            paint_brand_logo(cx.backend, BrandLogo::Gemini, top_left, size, color)
        }
        AgentProvider::GithubCopilot => {
            paint_brand_logo(cx.backend, BrandLogo::Copilot, top_left, size, color)
        }
        AgentProvider::OpenCode => paint_opencode_logo(cx.backend, top_left, size, color),
    }
}

/// Uppercase provider name for the group header (matches the TS
/// dropdown's `providerName` styling).
fn provider_label(provider: AgentProvider) -> &'static str {
    match provider {
        AgentProvider::ClaudeCode => "ANTHROPIC",
        AgentProvider::CodexCli => "OPENAI",
        AgentProvider::GeminiCli => "GEMINI",
        AgentProvider::GithubCopilot => "GITHUB COPILOT",
        AgentProvider::OpenCode => "OPENCODE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(p: AgentProvider, v: &str) -> ModelEntry {
        ModelEntry::new(p, v, v)
    }

    #[test]
    fn content_height_counts_groups_and_rows() {
        let models = vec![
            entry(AgentProvider::ClaudeCode, "a"),
            entry(AgentProvider::ClaudeCode, "b"),
            entry(AgentProvider::CodexCli, "c"),
        ];
        // 2 groups + 3 rows + padding.
        let expected = 2.0 * MODEL_GROUP_H + 3.0 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0;
        assert!((picker_content_height(&models) - expected).abs() < 0.01);
    }

    #[test]
    fn model_at_resolves_row_and_skips_headers() {
        let models = vec![
            entry(AgentProvider::ClaudeCode, "a"),
            entry(AgentProvider::CodexCli, "b"),
        ];
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(200.0, picker_content_height(&models)),
        };
        // First model row sits below the first group header.
        let first_row_y = MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0;
        assert_eq!(
            model_at(rect, Point2D::new(100.0, first_row_y), &models),
            Some(0)
        );
        // A click on the header band resolves to nothing.
        let header_y = MODEL_PICKER_PAD_Y + MODEL_GROUP_H / 2.0;
        assert_eq!(model_at(rect, Point2D::new(100.0, header_y), &models), None);
    }
}
