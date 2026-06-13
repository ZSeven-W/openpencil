//! Model-picker dropdown for the AI chat panel — the upward
//! popover that lists discovered models grouped by provider.
//! Mirrors the TS `ai-chat-model-selector.tsx` `ModelDropdown`
//! (search row + grouped rows + per-provider brand icon + selected
//! check / badges).

use crate::theme::Theme;
use crate::widgets::brand_icons::{paint_brand_logo, paint_opencode_logo, BrandLogo};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_text_input::paint_text_input_view_value;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use jian_core::text_input::TextInputState;
pub use jian_widgets::components::select::SelectHit;
use jian_widgets::components::select::SelectState;
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
pub const MODEL_PICKER_MAX_H: f32 = 288.0;
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
        acp: bool,
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

fn is_acp(entry: &ModelEntry) -> bool {
    entry.acp_agent_id().is_some()
}

fn same_group(a: &ModelEntry, b: &ModelEntry) -> bool {
    if is_acp(a) || is_acp(b) {
        return is_acp(a) && is_acp(b) && a.value == b.value;
    }
    a.provider == b.provider && a.builtin_provider_id == b.builtin_provider_id
}

fn model_matches(entry: &ModelEntry, q: &str) -> bool {
    q.is_empty()
        || entry.display_name.to_lowercase().contains(q)
        || entry.value.to_lowercase().contains(q)
        || searchable_group_label(entry).is_some_and(|label| label.to_lowercase().contains(q))
}

fn searchable_group_label(entry: &ModelEntry) -> Option<String> {
    if is_acp(entry) {
        return Some(group_label_for_entry(entry));
    }
    if is_builtin(entry) {
        return entry
            .builtin_provider_display_name
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .or_else(|| {
                (entry.provider == AgentProvider::ClaudeCode)
                    .then(|| "Anthropic (API Key)".to_string())
            });
    }
    Some(provider_label(entry.provider).to_string())
}

pub fn visible_model_indices(models: &[ModelEntry], search: &str) -> Vec<usize> {
    let q = normalized_query(search);
    models
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| model_matches(entry, &q).then_some(idx))
        .collect()
}

fn grouped_visible_model_indices(models: &[ModelEntry], search: &str) -> Vec<Vec<usize>> {
    let visible = visible_model_indices(models, search);
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for idx in visible {
        if let Some(group) = groups
            .iter_mut()
            .find(|group| same_group(&models[group[0]], &models[idx]))
        {
            group.push(idx);
        } else {
            groups.push(vec![idx]);
        }
    }
    groups
}

/// Walk the dropdown row layout, invoking `f(row, y, height)` for
/// each row top-to-bottom starting at `top`. Paint and hit-test
/// both drive off this so they never drift apart.
fn walk_rows(models: &[ModelEntry], search: &str, top: f32, mut f: impl FnMut(&Row, f32, f32)) {
    let mut y = top + MODEL_PICKER_PAD_Y;
    for group in grouped_visible_model_indices(models, search) {
        let first_idx = group[0];
        let entry = &models[first_idx];
        f(
            &Row::Header {
                provider: entry.provider,
                builtin: is_builtin(entry),
                acp: is_acp(entry),
                label: group_label_for_entry(entry),
            },
            y,
            MODEL_GROUP_H,
        );
        y += MODEL_GROUP_H;
        for (group_row, idx) in group.into_iter().enumerate() {
            f(
                &Row::Model {
                    idx,
                    first_in_group: group_row == 0,
                },
                y,
                MODEL_ROW_H,
            );
            y += MODEL_ROW_H;
        }
    }
}

/// Total dropdown height for `models` (group headers + rows + the
/// top/bottom padding).
pub fn picker_content_height(models: &[ModelEntry], search: &str) -> f32 {
    MODEL_SEARCH_H + picker_list_height(models, search)
}

fn picker_list_height(models: &[ModelEntry], search: &str) -> f32 {
    let groups = grouped_visible_model_indices(models, search);
    if groups.is_empty() {
        return MODEL_EMPTY_H;
    }
    let model_count: usize = groups.iter().map(Vec::len).sum();
    groups.len() as f32 * MODEL_GROUP_H
        + model_count as f32 * MODEL_ROW_H
        + MODEL_PICKER_PAD_Y * 2.0
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
    let mut state = SelectState::default();
    state.open = true;
    state.scroll.offset = scroll;
    match model_picker_hit(&state, rect, point, models, search) {
        SelectHit::Row(index) => Some(index),
        SelectHit::Inside | SelectHit::Outside => None,
    }
}

/// Shared select-style hit protocol for the searchable model picker.
/// Search/header/empty/padding chrome returns `Inside`; model rows
/// return `Row(index)` where `index` addresses `available_models`.
pub fn model_picker_hit(
    state: &SelectState,
    rect: Rect,
    point: Point2D,
    models: &[ModelEntry],
    search: &str,
) -> SelectHit {
    if !state.open {
        return SelectHit::Outside;
    }
    if point.x < rect.origin.x
        || point.x > rect.origin.x + rect.size.x
        || point.y < rect.origin.y
        || point.y > rect.origin.y + rect.size.y
    {
        return SelectHit::Outside;
    }
    let list_rect = model_list_rect(rect);
    if point.y < list_rect.origin.y {
        return SelectHit::Inside;
    }
    let mut hit = SelectHit::Inside;
    // Walk from a scroll-shifted origin — the same offset paint
    // applies via `translate` — then keep only hits whose row band
    // actually falls inside the (unscrolled) card rect.
    walk_rows(
        models,
        search,
        list_rect.origin.y - state.scroll.offset,
        |row, y, h| {
            if point.y >= y
                && point.y < y + h
                && point.y >= list_rect.origin.y
                && point.y <= rect.origin.y + rect.size.y
            {
                hit = match row {
                    Row::Model { idx, .. } => SelectHit::Row(*idx),
                    Row::Header { .. } => SelectHit::Inside,
                };
            }
        },
    );
    hit
}

pub fn search_clear_hit(rect: Rect, point: Point2D, search: &str) -> bool {
    !search.is_empty() && (search_clear_rect(rect)).contains(point)
}

fn model_list_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + MODEL_SEARCH_H),
        size: Point2D::new(rect.size.x, (rect.size.y - MODEL_SEARCH_H).max(0.0)),
    }
}

fn search_clear_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - 40.0, rect.origin.y + 7.0),
        size: Point2D::new(32.0, 24.0),
    }
}

/// Paint the dropdown card + grouped rows. `selected` is the index
/// of the active model (gets a check mark), `hover` the index of the
/// row under the cursor (gets a hover wash). `rect` is the painted
/// dropdown bounds (already capped at [`MODEL_PICKER_MAX_H`]);
/// `scroll` shifts the content up when the catalog overflows.
#[allow(clippy::too_many_arguments)]
pub fn paint_model_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    models: &[ModelEntry],
    selected: usize,
    state: &SelectState,
    input: &TextInputState,
    now_ms: u64,
    locale: op_editor_core::Locale,
) {
    let search = input.text();
    let scroll = state.scroll.offset;
    let hover = state.hover;
    // Card background + border — painted unscrolled so the frame
    // stays put while the rows scroll inside it.
    cx.backend.fill_round_rect(rect, 10.0, theme.card);
    cx.backend.stroke_round_rect(rect, 10.0, theme.border, 1.0);
    let row_left = rect.origin.x + 12.0;
    paint_search_row(cx, theme, rect, input, now_ms, locale);
    let list_rect = model_list_rect(rect);
    if visible_model_indices(models, search).is_empty() {
        let empty = op_i18n::translate(locale, "ai.noModelsFound");
        let layout = TextLayout::single_run(
            empty,
            "system-ui",
            12.0,
            (theme.muted_foreground).to_jian(),
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
            acp,
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
            } else if *acp {
                paint_plug_glyph(
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
                (theme.muted_foreground).to_jian(),
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
                (color).to_jian(),
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
    let track_h = (view_h - 8.0).max(0.0);
    if let Some(thumb_geom) =
        (jian_core::scroll::ScrollState { offset: scroll }).thumb(track_h, content_h, view_h, 24.0)
    {
        let thumb_y = list_rect.origin.y + 4.0 + thumb_geom.offset;
        let thumb = Rect {
            origin: Point2D::new(rect.origin.x + rect.size.x - 6.0, thumb_y),
            size: Point2D::new(3.0, thumb_geom.len),
        };
        cx.backend
            .fill_round_rect(thumb, 1.5, theme.muted_foreground);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_search_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    input: &TextInputState,
    now_ms: u64,
    locale: op_editor_core::Locale,
) {
    let raw = input.text();
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
        .fill_round_rect(search_rect, 6.0, (theme.muted).with_alpha(0.5));
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(search_rect.origin.x + 8.0, search_rect.origin.y + 6.0),
        12.0,
        theme.muted_foreground,
        1.4,
    );
    let text_x = search_rect.origin.x + 28.0;
    let input_rect = Rect {
        origin: Point2D::new(text_x, search_rect.origin.y),
        size: Point2D::new((search_rect.size.x - 52.0).max(0.0), search_rect.size.y),
    };
    if raw.is_empty() {
        let placeholder = op_i18n::translate(locale, "ai.searchModels");
        let layout = TextLayout::single_run(
            placeholder,
            "system-ui",
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&layout, Point2D::new(text_x, search_rect.origin.y + 17.0));
    }
    paint_text_input_view_value(
        cx,
        theme,
        input,
        input_rect,
        12.0,
        0.0,
        search_rect.origin.y + 17.0,
        now_ms,
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
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(rect.origin.x + 4.0, rect.origin.y + 11.0),
    );
}

pub(crate) fn paint_key_glyph(cx: &mut PaintCx<'_>, top_left: Point2D, size: f32, color: Color) {
    // Use the real lucide `Key` glyph (TS renders `<Key/>`) rather than a
    // hand-rolled ring+shaft approximation.
    crate::widgets::icons::draw_icon(
        cx.backend,
        crate::widgets::icons::Icon::Key,
        top_left,
        size,
        color,
        1.4,
    );
}

fn paint_plug_glyph(cx: &mut PaintCx<'_>, top_left: Point2D, size: f32, color: Color) {
    let x = top_left.x;
    let y = top_left.y;
    let body = Rect {
        origin: Point2D::new(x + size * 0.28, y + size * 0.36),
        size: Point2D::new(size * 0.44, size * 0.34),
    };
    cx.backend.stroke_round_rect(body, size * 0.08, color, 1.3);
    cx.backend.stroke_line(
        Point2D::new(x + size * 0.38, y + size * 0.16),
        Point2D::new(x + size * 0.38, y + size * 0.36),
        color,
        1.3,
    );
    cx.backend.stroke_line(
        Point2D::new(x + size * 0.62, y + size * 0.16),
        Point2D::new(x + size * 0.62, y + size * 0.36),
        color,
        1.3,
    );
    cx.backend.stroke_line(
        Point2D::new(x + size * 0.50, y + size * 0.70),
        Point2D::new(x + size * 0.50, y + size * 0.94),
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
pub(super) fn provider_label(provider: AgentProvider) -> &'static str {
    match provider {
        AgentProvider::ClaudeCode => "ANTHROPIC",
        AgentProvider::CodexCli => "OPENAI",
        AgentProvider::GeminiCli => "GOOGLE GEMINI",
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

pub(super) fn group_label_for_entry(entry: &ModelEntry) -> String {
    if is_acp(entry) {
        let name = entry.display_name.trim();
        if name.is_empty() {
            return "ACP".to_string();
        }
        return format!("{name} (ACP)").to_uppercase();
    }
    if is_builtin(entry) {
        if let Some(label) = entry
            .builtin_provider_display_name
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
        {
            return label.to_uppercase();
        }
    }
    group_label(entry.provider, is_builtin(entry)).to_string()
}
