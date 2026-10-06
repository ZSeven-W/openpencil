//! Studio-language painting for the chat model picker. The geometry,
//! scroll and hit-test contract live in `ai_chat_model_picker.rs`;
//! this sibling (declared from there via `#[path]`) keeps the paint
//! pass under the 800-line cap.
//!
//! Light-mode values restate the founder-approved Studio Home
//! palette — origin: `home_surface_palette.rs`. Several are verbatim
//! `tab_*` tokens (hover fill `#F0F5FD`, selected fill `#EAF2FF`,
//! border `#E1EAF7`, focus line `#C8DBFF`). The picker is painted
//! from the generic editor `Theme`, which carries no Studio tokens,
//! so dark mode derives each value from the closest `Theme` field.

use super::{
    is_acp, is_builtin, model_list_rect, model_row_pill_rect, normalized_query,
    paint_provider_logo, search_field_rect, visible_model_indices, walk_rows, Row, MODEL_EMPTY_H,
    MODEL_FOOTER_H, MODEL_GROUP_H, MODEL_PICKER_PAD_Y, MODEL_ROW_H,
};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_text_input::paint_text_input_view_value;
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_core::text_input::TextInputState;
use jian_widgets::components::select::SelectState;
use op_editor_core::chat::ModelEntry;
use op_editor_core::Locale;

// Studio light tokens (see the module doc for their origin).
const STUDIO_CARD: Color = Color::rgb_u8(0xFF, 0xFF, 0xFF);
const STUDIO_BORDER: Color = Color::rgb_u8(0xE1, 0xEA, 0xF7);
/// Popover shadow ink `#283548` at 10 % — `0 12px 32px`.
const STUDIO_SHADOW: Color = Color::rgba_u8(0x28, 0x35, 0x48, 0.10);
const STUDIO_SEARCH_WELL: Color = Color::rgb_u8(0xF5, 0xF8, 0xFD);
const STUDIO_HOVER_FILL: Color = Color::rgb_u8(0xF0, 0xF5, 0xFD);
/// One step darker than the hover fill (no Studio token exists for
/// the pressed step).
const STUDIO_PRESSED_FILL: Color = Color::rgb_u8(0xE7, 0xEF, 0xFB);
const STUDIO_SELECTED_FILL: Color = Color::rgb_u8(0xEA, 0xF2, 0xFF);
const STUDIO_SELECTED_INK: Color = Color::rgb_u8(0x25, 0x63, 0xEB);
const STUDIO_INK: Color = Color::rgb_u8(0x11, 0x1A, 0x32);
const STUDIO_MUTED: Color = Color::rgb_u8(0x78, 0x85, 0x9C);
const STUDIO_PLACEHOLDER: Color = Color::rgb_u8(0x92, 0x9D, 0xAF);
const STUDIO_HAIRLINE: Color = Color::rgb_u8(0xE8, 0xEE, 0xF7);
const STUDIO_ACTION_BLUE: Color = Color::rgb_u8(0x07, 0x5B, 0xFF);

/// Popover corner radius.
const POPOVER_RADIUS: f32 = 12.0;
/// Selected / hover pill corner radius.
const PILL_RADIUS: f32 = 8.0;
/// Left inset shared by every row's content (mark, labels, glyphs).
const ROW_INSET_X: f32 = 12.0;
/// Provider mark size inside a row and its gap to the name.
const ROW_LOGO: f32 = 18.0;
const ROW_TEXT_GAP: f32 = 10.0;
/// Right inset of the qualifier line on a plain row.
const ROW_RIGHT_INSET: f32 = 12.0;
/// Qualifier right inset on the selected row — clears the 14 px
/// check plus its gutters.
const CHECK_CLEARANCE: f32 = 36.0;
/// Minimum gutter kept between the name and the qualifier line.
const NAME_DETAIL_GAP: f32 = 10.0;
const NAME_SIZE: f32 = 13.0;
const DETAIL_SIZE: f32 = 11.0;
const GROUP_LABEL_SIZE: f32 = 10.0;
/// Letter-spacing of the group label.
const GROUP_TRACKING: f32 = 0.6;
const GROUP_LABEL_WEIGHT: u16 = 600;
const SEARCH_GLYPH: f32 = 14.0;
const FOOTER_GLYPH: f32 = 14.0;
const CHECK_SIZE: f32 = 14.0;

/// Resolved picker colours for one paint pass.
struct PickerTokens {
    card: Color,
    border: Color,
    shadow: Color,
    search_well: Color,
    search_border: Color,
    search_focus_border: Color,
    hover_fill: Color,
    pressed_fill: Color,
    selected_fill: Color,
    selected_ink: Color,
    ink: Color,
    muted: Color,
    placeholder: Color,
    hairline: Color,
    action_blue: Color,
}

impl PickerTokens {
    fn for_theme(theme: &Theme) -> Self {
        if theme_is_dark(theme) {
            // Dark mode: derive from the editor Theme so the popover
            // harmonises with the dark shell it floats over.
            Self {
                card: theme.popover,
                border: theme.border,
                shadow: Color::rgb_u8(0x00, 0x00, 0x00).with_alpha(0.32),
                search_well: theme.muted,
                search_border: theme.border,
                search_focus_border: theme.ring,
                hover_fill: theme.accent,
                pressed_fill: theme.muted,
                selected_fill: theme.row_selected_primary,
                selected_ink: theme.primary,
                ink: theme.foreground,
                muted: theme.muted_foreground,
                placeholder: theme.muted_foreground,
                hairline: theme.border,
                action_blue: theme.primary,
            }
        } else {
            Self {
                card: STUDIO_CARD,
                border: STUDIO_BORDER,
                shadow: STUDIO_SHADOW,
                search_well: STUDIO_SEARCH_WELL,
                search_border: STUDIO_BORDER,
                search_focus_border: Color::rgb_u8(0xC8, 0xDB, 0xFF),
                hover_fill: STUDIO_HOVER_FILL,
                pressed_fill: STUDIO_PRESSED_FILL,
                selected_fill: STUDIO_SELECTED_FILL,
                selected_ink: STUDIO_SELECTED_INK,
                ink: STUDIO_INK,
                muted: STUDIO_MUTED,
                placeholder: STUDIO_PLACEHOLDER,
                hairline: STUDIO_HAIRLINE,
                action_blue: STUDIO_ACTION_BLUE,
            }
        }
    }
}

/// The editor `Theme` carries no mode flag; its card fill is the
/// reliable proxy (light `#F7F7F7` vs dark `#1E1E1E`).
fn theme_is_dark(theme: &Theme) -> bool {
    let c = theme.card;
    0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b < 0.5
}

/// The laid-out text of one model row: the ellipsized name plus the
/// right-aligned qualifier, positioned so they can never collide.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct RowTextLabel {
    pub name: String,
    pub name_x: f32,
    pub detail: String,
    pub detail_x: f32,
    pub detail_w: f32,
}

/// Lay out one row's name + qualifier: the qualifier is measured and
/// right-aligned first, then the name is ellipsized (in the weight
/// it paints in) into whatever width remains.
pub(crate) fn fit_row_text(
    backend: &mut dyn RenderBackend,
    rect: Rect,
    selected: bool,
    name: &str,
    detail: Option<&str>,
) -> RowTextLabel {
    let name_x = rect.origin.x + ROW_INSET_X + ROW_LOGO + ROW_TEXT_GAP;
    let detail_w = detail
        .map(|d| text_metrics::measure_chrome(backend, d, DETAIL_SIZE))
        .unwrap_or(0.0);
    let detail_right = rect.origin.x + rect.size.x
        - if selected {
            CHECK_CLEARANCE
        } else {
            ROW_RIGHT_INSET
        };
    let detail_x = detail_right - detail_w;
    let weight = if selected { 600 } else { 500 };
    let name_limit = (detail_x - NAME_DETAIL_GAP - name_x).max(0.0);
    let name = crate::util::ellipsize_to_width(name, name_limit, |s| {
        text_metrics::measure_chrome_weighted(backend, s, NAME_SIZE, weight)
    });
    RowTextLabel {
        name,
        name_x,
        detail: detail.unwrap_or("").to_string(),
        detail_x,
        detail_w,
    }
}

/// The qualifier a model row carries as its right-aligned detail
/// line: the key / plug state for built-in / ACP rows, else the
/// group's recommended flag on an unfiltered list. (The catalogue
/// has no context-size field to surface.)
fn row_qualifier(
    entry: Option<&ModelEntry>,
    first_in_group: bool,
    query_empty: bool,
    locale: Locale,
) -> Option<String> {
    let entry = entry?;
    if is_builtin(entry) {
        return Some(op_i18n::translate(locale, "builtin.apiKeyBadge").to_string());
    }
    if is_acp(entry) {
        return Some("ACP".to_string());
    }
    (first_in_group && query_empty).then(|| op_i18n::translate(locale, "common.best").to_string())
}

/// Paint the dropdown card + grouped rows + footer. `selected` is
/// the index of the active model (gets the filled pill + check),
/// `hover` / `pressed` the row under the cursor. `rect` is the
/// painted dropdown bounds (already capped at
/// [`super::MODEL_PICKER_MAX_H`]); `scroll` shifts the content up
/// when the catalog overflows. The footer action row is fixed
/// chrome under the scrolling list — presses there resolve as
/// `Inside` in the shared hit protocol.
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
    locale: Locale,
) {
    let tokens = PickerTokens::for_theme(theme);
    let search = input.text();
    let scroll = state.scroll.offset;
    // Card shadow + surface — painted unscrolled so the frame stays
    // put while the rows scroll inside it. The shadow rect is offset
    // +12 and inset so the no-blur fallback stays mostly hidden
    // under the card.
    cx.backend.fill_drop_shadow(
        Rect {
            origin: Point2D::new(rect.origin.x + 8.0, rect.origin.y + 12.0),
            size: Point2D::new((rect.size.x - 16.0).max(0.0), (rect.size.y - 12.0).max(0.0)),
        },
        POPOVER_RADIUS,
        32.0,
        tokens.shadow,
    );
    cx.backend
        .fill_round_rect(rect, POPOVER_RADIUS, tokens.card);
    cx.backend
        .stroke_round_rect(rect, POPOVER_RADIUS, tokens.border, 1.0);
    paint_search_row(cx, theme, &tokens, rect, input, now_ms, locale);
    let list_rect = model_list_rect(rect);
    if visible_model_indices(models, search).is_empty() {
        let empty = op_i18n::translate(locale, "ai.noModelsFound");
        let layout = TextLayout::single_run(
            empty,
            "system-ui",
            12.0,
            (tokens.muted).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        let band = Rect {
            origin: Point2D::new(rect.origin.x, list_rect.origin.y + MODEL_PICKER_PAD_Y),
            size: Point2D::new(rect.size.x, MODEL_EMPTY_H),
        };
        let empty_x = text_metrics::centered_text_x(cx.backend, empty, 12.0, band);
        cx.backend.draw_text(
            &layout,
            Point2D::new(empty_x, jian_widgets::centered_text_baseline_y(band, 12.0)),
        );
    } else {
        // Clip to the list band and shift by `-scroll` so off-card
        // rows are trimmed and the visible band tracks the scroll
        // offset.
        cx.backend.save();
        cx.backend.clip_rect(list_rect);
        cx.backend.translate(Point2D::new(0.0, -scroll));
        let query_empty = normalized_query(search).is_empty();
        walk_rows(models, search, list_rect.origin.y, |row, y, h| match row {
            Row::Header { first_group, label } => {
                paint_group_header(cx, &tokens, rect, y, *first_group, label);
            }
            Row::Model {
                idx,
                first_in_group,
            } => {
                paint_model_row(
                    cx,
                    &tokens,
                    rect,
                    models,
                    *idx,
                    selected,
                    state.hover,
                    state.pressed,
                    *first_in_group,
                    query_empty,
                    locale,
                    y,
                    h,
                );
            }
        });
        cx.backend.restore();

        // Scrollbar thumb — drawn after `restore()` so it sits in
        // unscrolled card space. Shown only when the content
        // overflows; its length stays proportional to the visible
        // fraction of the list.
        let content_h = super::picker_list_height(models, search);
        let view_h = list_rect.size.y;
        let track_h = (view_h - 8.0).max(0.0);
        if let Some(thumb_geom) = (jian_core::scroll::ScrollState { offset: scroll })
            .thumb(track_h, content_h, view_h, 24.0)
        {
            let thumb_y = list_rect.origin.y + 4.0 + thumb_geom.offset;
            let thumb = Rect {
                origin: Point2D::new(rect.origin.x + rect.size.x - 6.0, thumb_y),
                size: Point2D::new(3.0, thumb_geom.len),
            };
            cx.backend.fill_round_rect(thumb, 1.5, tokens.muted);
        }
    }
    paint_footer_row(cx, &tokens, rect, locale);
}

/// Group header: hairline above every group but the first, then the
/// 10 px w600 uppercase label with 0.6 px letter-spacing at the
/// shared 12 px inset. Tracked text has no backend primitive, so the
/// label paints per glyph (the Home eyebrow precedent).
fn paint_group_header(
    cx: &mut PaintCx<'_>,
    tokens: &PickerTokens,
    rect: Rect,
    y: f32,
    first_group: bool,
    label: &str,
) {
    if !first_group {
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x + ROW_INSET_X, y),
                size: Point2D::new((rect.size.x - ROW_INSET_X * 2.0).max(0.0), 1.0),
            },
            tokens.hairline,
        );
    }
    let baseline = y + MODEL_GROUP_H / 2.0 + GROUP_LABEL_SIZE * 0.35;
    let mut x = rect.origin.x + ROW_INSET_X;
    for character in label.chars() {
        let glyph = character.to_string();
        let layout = TextLayout::single_run(
            &glyph,
            "system-ui",
            GROUP_LABEL_SIZE,
            (tokens.muted).to_jian(),
            Point2D::new(0.0, 0.0),
        )
        .with_font_weight(GROUP_LABEL_WEIGHT);
        cx.backend.draw_text(&layout, Point2D::new(x, baseline));
        x += text_metrics::measure_chrome_weighted(
            cx.backend,
            &glyph,
            GROUP_LABEL_SIZE,
            GROUP_LABEL_WEIGHT,
        ) + GROUP_TRACKING;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_model_row(
    cx: &mut PaintCx<'_>,
    tokens: &PickerTokens,
    rect: Rect,
    models: &[ModelEntry],
    idx: usize,
    selected: usize,
    hover: Option<usize>,
    pressed: Option<usize>,
    first_in_group: bool,
    query_empty: bool,
    locale: Locale,
    y: f32,
    _h: f32,
) {
    let is_selected = idx == selected;
    let pill = model_row_pill_rect(rect, y);
    if is_selected {
        cx.backend
            .fill_round_rect(pill, PILL_RADIUS, tokens.selected_fill);
    } else if pressed == Some(idx) {
        cx.backend
            .fill_round_rect(pill, PILL_RADIUS, tokens.pressed_fill);
    } else if hover == Some(idx) {
        cx.backend
            .fill_round_rect(pill, PILL_RADIUS, tokens.hover_fill);
    }
    let entry = models.get(idx);
    // Provider mark — the builtin / ACP key/plug state moved from
    // the header glyph to the qualifier line, so every row shows its
    // provider's mark.
    let logo_y = y + (MODEL_ROW_H - ROW_LOGO) / 2.0;
    if let Some(entry) = entry {
        paint_provider_logo(
            cx,
            entry.provider,
            Point2D::new(rect.origin.x + ROW_INSET_X, logo_y),
            ROW_LOGO,
            tokens.muted,
        );
    }
    let detail = row_qualifier(entry, first_in_group, query_empty, locale);
    let label = fit_row_text(
        cx.backend,
        rect,
        is_selected,
        entry.map(|m| m.display_name.as_str()).unwrap_or(""),
        detail.as_deref(),
    );
    let baseline = y + MODEL_ROW_H / 2.0 + NAME_SIZE * 0.35;
    let name_color = if is_selected {
        tokens.selected_ink
    } else {
        tokens.ink
    };
    let name_weight = if is_selected { 600 } else { 500 };
    let name = TextLayout::single_run(
        &label.name,
        "system-ui",
        NAME_SIZE,
        name_color.to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(name_weight);
    cx.backend
        .draw_text(&name, Point2D::new(label.name_x, baseline));
    if label.detail_w > 0.0 {
        let detail = TextLayout::single_run(
            &label.detail,
            "system-ui",
            DETAIL_SIZE,
            (tokens.muted).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&detail, Point2D::new(label.detail_x, baseline));
    }
    if is_selected {
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(
                rect.origin.x + rect.size.x - ROW_INSET_X - CHECK_SIZE - 8.0,
                y + (MODEL_ROW_H - CHECK_SIZE) / 2.0,
            ),
            CHECK_SIZE,
            tokens.selected_ink,
            1.7,
        );
    }
}

/// Search strip: the 32 px rounded-8 well with its 14 px glyph and
/// 12 px placeholder. The well shows its focus border once the field
/// is engaged (text present or caret moved) — the popover's search
/// owns editor focus whenever it is open, so a fresh untouched field
/// is the only state that shows the rest border.
fn paint_search_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    tokens: &PickerTokens,
    rect: Rect,
    input: &TextInputState,
    now_ms: u64,
    locale: Locale,
) {
    let raw = input.text();
    let well = search_field_rect(rect);
    cx.backend.fill_round_rect(well, 8.0, tokens.search_well);
    let engaged = !raw.is_empty() || input.caret() > 0;
    let border = if engaged {
        tokens.search_focus_border
    } else {
        tokens.search_border
    };
    cx.backend.stroke_round_rect(well, 8.0, border, 1.0);
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(
            well.origin.x + 10.0,
            well.origin.y + (well.size.y - SEARCH_GLYPH) / 2.0,
        ),
        SEARCH_GLYPH,
        tokens.muted,
        1.5,
    );
    let text_x = well.origin.x + 10.0 + SEARCH_GLYPH + 8.0;
    let baseline = jian_widgets::centered_text_baseline_y(well, 12.0);
    let input_rect = Rect {
        origin: Point2D::new(text_x, well.origin.y),
        size: Point2D::new(
            (well.origin.x + well.size.x - 30.0 - text_x).max(0.0),
            well.size.y,
        ),
    };
    if raw.is_empty() {
        let placeholder = op_i18n::translate(locale, "ai.searchModels");
        let layout = TextLayout::single_run(
            placeholder,
            "system-ui",
            12.0,
            (tokens.placeholder).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&layout, Point2D::new(text_x, baseline));
    }
    paint_text_input_view_value(cx, theme, input, input_rect, 12.0, 0.0, baseline, now_ms);
    if !raw.is_empty() {
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(
                well.origin.x + well.size.x - 18.0,
                well.origin.y + (well.size.y - 10.0) / 2.0,
            ),
            10.0,
            tokens.muted,
            1.4,
        );
    }
}

/// Footer action row: a full-width hairline, then the 14 px plus
/// glyph and the blue label so the row reads as an action rather
/// than another model.
fn paint_footer_row(cx: &mut PaintCx<'_>, tokens: &PickerTokens, rect: Rect, locale: Locale) {
    let top = rect.origin.y + rect.size.y - MODEL_FOOTER_H;
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(rect.origin.x, top),
            size: Point2D::new(rect.size.x, 1.0),
        },
        tokens.hairline,
    );
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(
            rect.origin.x + ROW_INSET_X,
            top + (MODEL_FOOTER_H - FOOTER_GLYPH) / 2.0,
        ),
        FOOTER_GLYPH,
        tokens.action_blue,
        1.6,
    );
    let label = op_i18n::translate(locale, "home.connectMoreModels");
    let row = Rect {
        origin: Point2D::new(rect.origin.x, top),
        size: Point2D::new(rect.size.x, MODEL_FOOTER_H),
    };
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        (tokens.action_blue).to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(500);
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            rect.origin.x + ROW_INSET_X + FOOTER_GLYPH + ROW_TEXT_GAP,
            jian_widgets::centered_text_baseline_y(row, 12.0),
        ),
    );
}
