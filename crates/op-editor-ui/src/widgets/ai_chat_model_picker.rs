//! Model-picker dropdown for the AI chat panel — the upward
//! popover that lists discovered models grouped by provider.
//! Mirrors the TS `ai-chat-model-selector.tsx` `ModelDropdown`
//! (search row + grouped rows + per-provider brand icon + selected
//! check / badges).

use crate::theme::Theme;
use crate::widgets::brand_icons::{paint_brand_logo, paint_opencode_logo, BrandLogo};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_inputs::to_jian_color;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::chat::{AgentProvider, ModelEntry};

/// Height of a provider group-header row.
pub const MODEL_GROUP_H: f32 = 22.0;
/// Height of a single model row.
pub const MODEL_ROW_H: f32 = 28.0;
/// Vertical padding inside the dropdown card (top + bottom each).
pub const MODEL_PICKER_PAD_Y: f32 = 6.0;
/// Fixed search strip at the top of the dropdown.
pub const MODEL_SEARCH_H: f32 = 40.0;
/// Hard cap on the dropdown's painted height. A connected catalog
/// taller than this (e.g. OpenCode's 75+ models) scrolls inside the
/// card instead of growing off the top of the screen.
pub const MODEL_PICKER_MAX_H: f32 = 320.0;
const MODEL_EMPTY_H: f32 = 44.0;

/// Painted height of the dropdown for `models` — the content height
/// clamped to [`MODEL_PICKER_MAX_H`].
pub fn picker_view_height(models: &[ModelEntry], search: &str) -> f32 {
    picker_content_height(models, search).min(MODEL_PICKER_MAX_H)
}

/// Largest valid scroll offset for `models` — `0` when the content
/// already fits inside [`MODEL_PICKER_MAX_H`].
pub fn max_picker_scroll(models: &[ModelEntry], search: &str) -> f32 {
    let view_list_h = (picker_view_height(models, search) - MODEL_SEARCH_H).max(0.0);
    (picker_list_height(models, search) - view_list_h).max(0.0)
}

/// One laid-out row in the dropdown.
enum Row {
    /// Provider group header — carries the provider for its logo.
    Header {
        provider: AgentProvider,
        builtin: bool,
        label: String,
    },
    /// Selectable model — carries its index into the flat list.
    Model { idx: usize, first_in_group: bool },
}

fn normalized_query(search: &str) -> String {
    search.trim().to_lowercase()
}

fn is_builtin(entry: &ModelEntry) -> bool {
    entry.builtin_provider_id.is_some() || entry.value.starts_with("builtin:")
}

fn same_group(a: &ModelEntry, b: &ModelEntry) -> bool {
    a.provider == b.provider && a.builtin_provider_id == b.builtin_provider_id
}

fn model_matches(entry: &ModelEntry, q: &str) -> bool {
    q.is_empty()
        || entry.display_name.to_lowercase().contains(q)
        || entry.value.to_lowercase().contains(q)
        || provider_label(entry.provider).to_lowercase().contains(q)
        || group_label_for_entry(entry).to_lowercase().contains(q)
        || (is_builtin(entry) && "api key".contains(q))
}

pub fn visible_model_indices(models: &[ModelEntry], search: &str) -> Vec<usize> {
    let q = normalized_query(search);
    models
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| model_matches(entry, &q).then_some(idx))
        .collect()
}

/// Walk the dropdown row layout, invoking `f(row, y, height)` for
/// each row top-to-bottom starting at `top`. Paint and hit-test
/// both drive off this so they never drift apart.
fn walk_rows(models: &[ModelEntry], search: &str, top: f32, mut f: impl FnMut(&Row, f32, f32)) {
    let mut y = top + MODEL_PICKER_PAD_Y;
    let visible = visible_model_indices(models, search);
    let mut last_idx: Option<usize> = None;
    for idx in visible {
        let entry = &models[idx];
        let first_in_group = last_idx
            .map(|prev| !same_group(&models[prev], entry))
            .unwrap_or(true);
        if first_in_group {
            f(
                &Row::Header {
                    provider: entry.provider,
                    builtin: is_builtin(entry),
                    label: group_label_for_entry(entry),
                },
                y,
                MODEL_GROUP_H,
            );
            y += MODEL_GROUP_H;
        }
        f(
            &Row::Model {
                idx,
                first_in_group,
            },
            y,
            MODEL_ROW_H,
        );
        y += MODEL_ROW_H;
        last_idx = Some(idx);
    }
}

/// Total dropdown height for `models` (group headers + rows + the
/// top/bottom padding).
pub fn picker_content_height(models: &[ModelEntry], search: &str) -> f32 {
    MODEL_SEARCH_H + picker_list_height(models, search)
}

fn picker_list_height(models: &[ModelEntry], search: &str) -> f32 {
    let visible = visible_model_indices(models, search);
    if visible.is_empty() {
        return MODEL_EMPTY_H;
    }
    let mut groups = 0usize;
    let mut last_idx: Option<usize> = None;
    for idx in visible.iter().copied() {
        let entry = &models[idx];
        if last_idx
            .map(|prev| !same_group(&models[prev], entry))
            .unwrap_or(true)
        {
            groups += 1;
        }
        last_idx = Some(idx);
    }
    groups as f32 * MODEL_GROUP_H + visible.len() as f32 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0
}

/// Map a click inside the dropdown `rect` to the index of the
/// model row under it. `None` for a click on a header / padding.
/// `scroll` is the dropdown's vertical scroll offset in px — paint
/// and hit-test share it so a scrolled row resolves correctly.
pub fn model_at(
    rect: Rect,
    point: Point2D,
    models: &[ModelEntry],
    scroll: f32,
    search: &str,
) -> Option<usize> {
    if point.x < rect.origin.x
        || point.x > rect.origin.x + rect.size.x
        || point.y < rect.origin.y
        || point.y > rect.origin.y + rect.size.y
    {
        return None;
    }
    let list_rect = model_list_rect(rect);
    if point.y < list_rect.origin.y {
        return None;
    }
    let mut hit = None;
    // Walk from a scroll-shifted origin — the same offset paint
    // applies via `translate` — then keep only hits whose row band
    // actually falls inside the (unscrolled) card rect.
    walk_rows(models, search, list_rect.origin.y - scroll, |row, y, h| {
        if let Row::Model { idx, .. } = row {
            if point.y >= y
                && point.y < y + h
                && point.y >= list_rect.origin.y
                && point.y <= rect.origin.y + rect.size.y
            {
                hit = Some(*idx);
            }
        }
    });
    hit
}

fn model_list_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + MODEL_SEARCH_H),
        size: Point2D::new(rect.size.x, (rect.size.y - MODEL_SEARCH_H).max(0.0)),
    }
}

/// Paint the dropdown card + grouped rows. `selected` is the index
/// of the active model (gets a check mark), `hover` the index of the
/// row under the cursor (gets a hover wash). `rect` is the painted
/// dropdown bounds (already capped at [`MODEL_PICKER_MAX_H`]);
/// `scroll` shifts the content up when the catalog overflows.
pub fn paint_model_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    models: &[ModelEntry],
    selected: usize,
    scroll: f32,
    hover: Option<usize>,
    search: &str,
    locale: op_editor_core::Locale,
) {
    // Card background + border — painted unscrolled so the frame
    // stays put while the rows scroll inside it.
    cx.backend.fill_round_rect(rect, 10.0, theme.card);
    cx.backend.stroke_round_rect(rect, 10.0, theme.border, 1.0);
    let row_left = rect.origin.x + 12.0;
    paint_search_row(cx, theme, rect, search, locale);
    let list_rect = model_list_rect(rect);
    if visible_model_indices(models, search).is_empty() {
        let empty = op_i18n::translate(locale, "ai.noModelsFound");
        let layout = TextLayout::single_run(
            empty,
            "system-ui",
            12.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        let w = cx.backend.measure_text(empty, 12.0);
        cx.backend.draw_text(
            &layout,
            Point2D::new(
                rect.origin.x + (rect.size.x - w) / 2.0,
                list_rect.origin.y + 26.0,
            ),
        );
        return;
    }
    // Clip to the card and shift by `-scroll` so off-card rows are
    // trimmed and the visible band tracks the scroll offset.
    cx.backend.save();
    cx.backend.clip_rect(list_rect);
    cx.backend.translate(Point2D::new(0.0, -scroll));
    walk_rows(models, search, list_rect.origin.y, |row, y, h| match row {
        Row::Header {
            provider,
            builtin,
            label,
        } => {
            let logo_y = y + (h - 12.0) / 2.0;
            if *builtin {
                paint_key_glyph(
                    cx,
                    Point2D::new(row_left, logo_y),
                    12.0,
                    theme.muted_foreground,
                );
            } else {
                paint_provider_logo(
                    cx,
                    *provider,
                    Point2D::new(row_left, logo_y),
                    12.0,
                    theme.muted_foreground,
                );
            }
            let label = TextLayout::single_run(
                label,
                "system-ui",
                10.0,
                to_jian_color(theme.muted_foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&label, Point2D::new(row_left + 18.0, y + h / 2.0 + 3.0));
        }
        Row::Model {
            idx,
            first_in_group,
        } => {
            let is_selected = *idx == selected;
            let is_hovered = hover == Some(*idx);
            // Hover wash on any non-selected row the cursor is over;
            // the selected row keeps its own `muted` fill below.
            if is_hovered && !is_selected {
                cx.backend.fill_round_rect(
                    Rect {
                        origin: Point2D::new(rect.origin.x + 4.0, y + 1.0),
                        size: Point2D::new(rect.size.x - 8.0, h - 2.0),
                    },
                    6.0,
                    theme.button_hover,
                );
            }
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
            cx.backend
                .draw_text(&label, Point2D::new(row_left + 22.0, y + h / 2.0 + 4.0));
            if let Some(entry) = models.get(*idx) {
                if is_builtin(entry) {
                    paint_badge(
                        cx,
                        theme,
                        op_i18n::translate(locale, "builtin.apiKeyBadge"),
                        rect.origin.x + rect.size.x - 12.0,
                        y + (h - 16.0) / 2.0,
                    );
                } else if *first_in_group && normalized_query(search).is_empty() {
                    paint_badge(
                        cx,
                        theme,
                        op_i18n::translate(locale, "common.best"),
                        rect.origin.x + rect.size.x - 12.0,
                        y + (h - 16.0) / 2.0,
                    );
                }
            }
        }
    });
    cx.backend.restore();

    // Scrollbar thumb — drawn after `restore()` so it sits in
    // unscrolled card space. Shown only when the content overflows.
    let content_h = picker_list_height(models, search);
    let view_h = list_rect.size.y;
    if content_h > view_h + 0.5 {
        let track_h = view_h - 8.0;
        let thumb_h = (track_h * view_h / content_h).max(24.0);
        let max_scroll = (content_h - view_h).max(0.0);
        let t = if max_scroll > 0.0 {
            (scroll / max_scroll).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let thumb_y = list_rect.origin.y + 4.0 + t * (track_h - thumb_h);
        let thumb = Rect {
            origin: Point2D::new(rect.origin.x + rect.size.x - 6.0, thumb_y),
            size: Point2D::new(3.0, thumb_h),
        };
        cx.backend
            .fill_round_rect(thumb, 1.5, theme.muted_foreground);
    }
}

fn paint_search_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    search: &str,
    locale: op_editor_core::Locale,
) {
    let divider_y = rect.origin.y + MODEL_SEARCH_H - 0.5;
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(rect.origin.x, divider_y),
            size: Point2D::new(rect.size.x, 1.0),
        },
        theme.border,
    );
    let search_rect = Rect {
        origin: Point2D::new(rect.origin.x + 8.0, rect.origin.y + 7.0),
        size: Point2D::new(rect.size.x - 16.0, 24.0),
    };
    cx.backend
        .fill_round_rect(search_rect, 6.0, with_alpha(theme.muted, 0.5));
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(search_rect.origin.x + 8.0, search_rect.origin.y + 6.0),
        12.0,
        theme.muted_foreground,
        1.4,
    );
    let raw = search.trim();
    let (label, color) = if raw.is_empty() {
        (
            op_i18n::translate(locale, "ai.searchModels"),
            theme.muted_foreground,
        )
    } else {
        (raw, theme.foreground)
    };
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(search_rect.origin.x + 28.0, search_rect.origin.y + 17.0),
    );
    if !raw.is_empty() {
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(
                search_rect.origin.x + search_rect.size.x - 18.0,
                search_rect.origin.y + 7.0,
            ),
            10.0,
            theme.muted_foreground,
            1.4,
        );
    }
}

fn paint_badge(cx: &mut PaintCx<'_>, theme: &Theme, text: &str, right_x: f32, y: f32) {
    let w = cx.backend.measure_text(text, 9.0) + 8.0;
    let rect = Rect {
        origin: Point2D::new(right_x - w, y),
        size: Point2D::new(w, 16.0),
    };
    cx.backend.fill_round_rect(rect, 4.0, theme.muted);
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        9.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(rect.origin.x + 4.0, rect.origin.y + 11.0),
    );
}

fn with_alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

fn paint_key_glyph(cx: &mut PaintCx<'_>, top_left: Point2D, size: f32, color: Color) {
    let cy = top_left.y + size * 0.5;
    let ring = Rect {
        origin: Point2D::new(top_left.x, cy - size * 0.28),
        size: Point2D::new(size * 0.55, size * 0.55),
    };
    cx.backend
        .stroke_round_rect(ring, ring.size.x / 2.0, color, 1.3);
    cx.backend.stroke_line(
        Point2D::new(top_left.x + size * 0.52, cy),
        Point2D::new(top_left.x + size, cy),
        color,
        1.3,
    );
    cx.backend.stroke_line(
        Point2D::new(top_left.x + size * 0.82, cy),
        Point2D::new(top_left.x + size * 0.82, cy + size * 0.25),
        color,
        1.3,
    );
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

fn group_label(provider: AgentProvider, builtin: bool) -> &'static str {
    if builtin {
        match provider {
            AgentProvider::ClaudeCode => "ANTHROPIC API KEY",
            AgentProvider::CodexCli => "OPENAI API KEY",
            AgentProvider::GeminiCli => "GEMINI API KEY",
            AgentProvider::GithubCopilot => "COPILOT API KEY",
            AgentProvider::OpenCode => "OPENCODE API KEY",
        }
    } else {
        provider_label(provider)
    }
}

fn group_label_for_entry(entry: &ModelEntry) -> String {
    if is_builtin(entry) {
        if let Some(label) = entry
            .builtin_provider_display_name
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
        {
            return label.to_string();
        }
    }
    group_label(entry.provider, is_builtin(entry)).to_string()
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
        let expected =
            MODEL_SEARCH_H + 2.0 * MODEL_GROUP_H + 3.0 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0;
        assert!((picker_content_height(&models, "") - expected).abs() < 0.01);
    }

    #[test]
    fn model_at_resolves_row_and_skips_headers() {
        let models = vec![
            entry(AgentProvider::ClaudeCode, "a"),
            entry(AgentProvider::CodexCli, "b"),
        ];
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(200.0, picker_content_height(&models, "")),
        };
        // First model row sits below the first group header.
        let first_row_y = MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0;
        assert_eq!(
            model_at(rect, Point2D::new(100.0, first_row_y), &models, 0.0, ""),
            Some(0)
        );
        // A click on the header band resolves to nothing.
        let header_y = MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H / 2.0;
        assert_eq!(
            model_at(rect, Point2D::new(100.0, header_y), &models, 0.0, ""),
            None
        );
    }

    #[test]
    fn model_at_honors_scroll_offset() {
        // A tall catalog (one group, many rows) clamped to the cap;
        // with the content scrolled down, the row under a fixed
        // cursor point shifts to a later index.
        let models: Vec<ModelEntry> = (0..40)
            .map(|i| entry(AgentProvider::OpenCode, &format!("m{i}")))
            .collect();
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(200.0, picker_view_height(&models, "")),
        };
        let probe = Point2D::new(
            100.0,
            MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0,
        );
        let unscrolled = model_at(rect, probe, &models, 0.0, "");
        let scrolled = model_at(rect, probe, &models, MODEL_ROW_H * 3.0, "");
        assert_eq!(unscrolled, Some(0));
        assert_eq!(scrolled, Some(3));
        // The catalog overflows the cap, so scrolling is possible.
        assert!(max_picker_scroll(&models, "") > 0.0);
    }

    #[test]
    fn model_at_filters_by_search_and_returns_original_index() {
        let models = vec![
            entry(AgentProvider::ClaudeCode, "opus"),
            entry(AgentProvider::CodexCli, "gpt-5.5"),
            entry(AgentProvider::CodexCli, "gpt-4.1"),
        ];
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(220.0, picker_view_height(&models, "5.5")),
        };
        let first_filtered_row_y =
            MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0;

        assert_eq!(
            model_at(
                rect,
                Point2D::new(100.0, first_filtered_row_y),
                &models,
                0.0,
                "5.5"
            ),
            Some(1)
        );
    }

    #[test]
    fn builtin_group_header_prefers_retained_provider_display_name() {
        let mut entry = ModelEntry::builtin(
            AgentProvider::CodexCli,
            "builtin-1",
            "builtin:builtin-1:MiniMax-M2.7",
            "MiniMax-M2.7",
        );
        entry.builtin_provider_display_name = Some("MiniMax".into());

        assert_eq!(group_label_for_entry(&entry), "MiniMax");
    }

    #[test]
    fn search_matches_retained_builtin_provider_display_name() {
        let entry = ModelEntry::builtin_with_display_name(
            AgentProvider::CodexCli,
            "builtin-bailian",
            "百炼CP",
            "builtin:builtin-bailian:qwen3-coder-plus",
            "qwen3-coder-plus",
        );

        assert_eq!(visible_model_indices(&[entry], "百炼"), vec![0]);
    }

    #[test]
    fn builtin_group_header_falls_back_to_generic_label_without_retained_name() {
        let entry = ModelEntry::builtin(
            AgentProvider::CodexCli,
            "builtin-1",
            "builtin:builtin-1:deepseek-v4-pro",
            "deepseek-v4-pro",
        );

        assert_eq!(group_label_for_entry(&entry), "OPENAI API KEY");
    }

    #[test]
    fn builtin_groups_stay_separate_when_ids_differ_but_provider_matches() {
        let models = vec![
            ModelEntry::builtin(
                AgentProvider::CodexCli,
                "builtin-1",
                "builtin:builtin-1:MiniMax-M2.7",
                "MiniMax-M2.7",
            ),
            ModelEntry::builtin(
                AgentProvider::CodexCli,
                "builtin-2",
                "builtin:builtin-2:deepseek-v4-pro",
                "deepseek-v4-pro",
            ),
        ];

        let expected =
            MODEL_SEARCH_H + 2.0 * MODEL_GROUP_H + 2.0 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0;
        assert!((picker_content_height(&models, "") - expected).abs() < 0.01);
    }
}
