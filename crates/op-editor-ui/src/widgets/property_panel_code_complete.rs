use super::code_i18n::CodePanelStrings;
use super::{action_hovered, draw_line, origin};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_inputs::{to_jian_color, INPUT_RADIUS, PAD_X};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::codegen::{CodeSelection, CodegenHover, CodegenState};

const CODE_AREA_MIN_H: f32 = 220.0;
const CODE_LINE_H: f32 = 16.0;
const CODE_FONT_SIZE: f32 = 11.0;
const CODE_PAD_X: f32 = 12.0;
const CODE_PAD_Y: f32 = 10.0;
const CODE_GUTTER_W: f32 = 28.0;
const CODE_TEXT_GAP: f32 = 10.0;
const CODE_CHAR_W: f32 = CODE_FONT_SIZE * 0.55;
const ACTION_ROW_GAP: f32 = 12.0;
const ACTION_CHIP_H: f32 = 30.0;
const ACTION_CHIP_GAP: f32 = 8.0;
const ACTION_CHIP_ICON_SIZE: f32 = 14.0;
const ACTION_CHIP_PAD_X: f32 = 12.0;
const ACTION_CHIP_TEXT_GAP: f32 = 8.0;
const BODY_BOTTOM_PAD: f32 = 12.0;
const ACTION_LABEL_FONT_SIZE: f32 = 13.0;

#[derive(Clone, Copy)]
pub(super) struct CompleteLayout {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub progress_row_h: f32,
    pub panel_bottom: Option<f32>,
}

pub(super) fn complete_action_row_y_in_panel(state: &CodegenState, layout: CompleteLayout) -> f32 {
    let code_y = code_area_y(state, layout.y, layout.progress_row_h);
    code_y + code_area_height(state, layout) + ACTION_ROW_GAP
}

pub(super) fn code_area_y(state: &CodegenState, y: f32, progress_row_h: f32) -> f32 {
    let mut y = y;
    if state.degraded {
        y += progress_row_h;
    }
    if !state.assets.is_empty() {
        y += progress_row_h;
    }
    y
}

pub(super) fn action_chip_rects(x: f32, y: f32, w: f32, strings: CodePanelStrings) -> [Rect; 4] {
    let usable = w - PAD_X * 2.0;
    let preferred = [
        action_chip_width(strings.copy()),
        action_chip_width(strings.save()),
        action_chip_width(strings.bundle()),
        action_chip_width(strings.regen()),
    ];
    let preferred_total =
        preferred.iter().sum::<f32>() + ACTION_CHIP_GAP * (preferred.len() - 1) as f32;
    let fits_preferred = preferred_total <= usable;
    let widths = if fits_preferred {
        preferred
    } else {
        let fallback = ((usable - ACTION_CHIP_GAP * 3.0) / 4.0).max(ACTION_CHIP_H);
        [fallback; 4]
    };
    let total = widths.iter().sum::<f32>() + ACTION_CHIP_GAP * (widths.len() - 1) as f32;
    let mut cursor = if fits_preferred {
        x + w - PAD_X - total
    } else {
        x + PAD_X
    };
    std::array::from_fn(|i| {
        let rect = Rect {
            origin: Point2D::new(cursor, y),
            size: Point2D::new(widths[i], ACTION_CHIP_H),
        };
        cursor += widths[i] + ACTION_CHIP_GAP;
        rect
    })
}

pub(super) fn paint_complete_body_in_panel(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    strings: CodePanelStrings,
    layout: CompleteLayout,
) -> f32 {
    let mut y = layout.y;
    if state.degraded {
        draw_icon(
            cx.backend,
            Icon::AlertTriangle,
            Point2D::new(layout.x + PAD_X, y + 4.0),
            16.0,
            theme.muted_foreground,
            1.6,
        );
        draw_line(
            cx,
            strings.generated_degraded(),
            theme.muted_foreground,
            layout.x + PAD_X + 22.0,
            y + 16.0,
        );
        y += layout.progress_row_h;
    }
    if !state.assets.is_empty() {
        let notice = strings.includes_assets(state.assets.len());
        draw_line(
            cx,
            &notice,
            theme.muted_foreground,
            layout.x + PAD_X,
            y + 16.0,
        );
        y += layout.progress_row_h;
    }
    let code_h = code_area_height_from_y(y, layout.panel_bottom);
    let code_rect = code_area_rect(layout.x, y, layout.w, code_h);
    y = paint_code_area(
        cx,
        theme,
        &state.code,
        state.code_selection,
        state.code_scroll,
        code_rect,
    );
    let actions = [
        (Icon::Copy, strings.copy(), CodegenHover::Copy),
        (Icon::Download, strings.save(), CodegenHover::Download),
        (Icon::Sparkles, strings.bundle(), CodegenHover::ExportBundle),
        (Icon::RefreshCw, strings.regen(), CodegenHover::Regenerate),
    ];
    let rects = action_chip_rects(layout.x, y, layout.w, strings);
    for ((icon, label, hover), rect) in actions.iter().zip(rects) {
        paint_action_chip(cx, theme, *icon, label, rect, action_hovered(state, *hover));
    }
    y + ACTION_CHIP_H + 12.0
}

pub(super) fn code_text_offset_at_in_panel(
    state: &CodegenState,
    point: Point2D,
    layout: CompleteLayout,
) -> Option<usize> {
    let rect = code_area_rect_for_panel(state, layout);
    if !(rect).contains(point) {
        return None;
    }
    let lines = line_spans(&state.code);
    let last_end = state.code.len();
    if lines.is_empty() {
        return Some(0);
    }
    let scroll = state
        .code_scroll
        .clamp(0.0, max_scroll_for_code(&state.code, rect));
    let line_index = ((point.y - code_text_top(rect) + scroll) / CODE_LINE_H)
        .floor()
        .max(0.0) as usize;
    let line_index = line_index.min(lines.len() - 1);
    let span = lines[line_index];
    let rel_x = (point.x - code_text_x(rect)).max(0.0);
    let col = (rel_x / CODE_CHAR_W).round().max(0.0) as usize;
    Some((span.start + byte_offset_for_col(span.text, col)).min(last_end))
}

pub(super) fn code_area_rect_for_panel(state: &CodegenState, layout: CompleteLayout) -> Rect {
    let y = code_area_y(state, layout.y, layout.progress_row_h);
    code_area_rect(layout.x, y, layout.w, code_area_height(state, layout))
}

pub(super) fn code_preview_max_scroll(state: &CodegenState, layout: CompleteLayout) -> f32 {
    let rect = code_area_rect_for_panel(state, layout);
    max_scroll_for_code(&state.code, rect)
}

fn paint_code_area(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    code: &str,
    selection: Option<CodeSelection>,
    scroll: f32,
    rect: Rect,
) -> f32 {
    let scroll = scroll.clamp(0.0, max_scroll_for_code(code, rect));
    let lines = visible_line_spans(code, rect, scroll);
    cx.backend
        .fill_round_rect(rect, INPUT_RADIUS, code_surface(theme));
    cx.backend
        .stroke_round_rect(rect, INPUT_RADIUS, theme.border, 1.0);
    cx.backend.save();
    cx.backend.clip_rect(rect);
    let gutter_right = code_text_x(rect) - CODE_TEXT_GAP;
    cx.backend.fill_rect(
        Rect::xywh(
            gutter_right,
            rect.origin.y + CODE_PAD_Y,
            1.0,
            rect.size.y - CODE_PAD_Y * 2.0,
        ),
        code_gutter_rule(theme),
    );
    if let Some(selection) = selection {
        paint_code_selection(cx, theme, &lines, selection, code.len(), rect);
    }
    for line in lines.iter() {
        let n = (line.index + 1).to_string();
        let line_no_w = cx.backend.measure_text(&n, CODE_FONT_SIZE);
        let layout = TextLayout::single_run(
            &n,
            "monospace",
            CODE_FONT_SIZE,
            to_jian_color(code_line_number_color(theme)),
            origin(),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(gutter_right - 8.0 - line_no_w, line.baseline_y),
        );
        paint_highlighted_code_line(
            cx,
            theme,
            line.span.text,
            code_text_x(rect),
            line.baseline_y,
        );
    }
    cx.backend.restore();
    rect.origin.y + rect.size.y + ACTION_ROW_GAP
}

fn paint_code_selection(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    lines: &[VisibleCodeLine<'_>],
    selection: CodeSelection,
    code_len: usize,
    rect: Rect,
) {
    if selection.is_collapsed() {
        return;
    }
    let (mut start, mut end) = selection.ordered();
    start = start.min(code_len);
    end = end.min(code_len);
    if start >= end {
        return;
    }
    let text_x = code_text_x(rect);
    for line in lines.iter() {
        let span = line.span;
        if start > span.end || end < span.start {
            continue;
        }
        let sel_start = start.max(span.start).min(span.end);
        let sel_end = end.max(span.start).min(span.end);
        let includes_line_break = end > span.end && start <= span.end;
        if sel_start >= sel_end && !includes_line_break {
            continue;
        }
        let before = &span.text[..sel_start - span.start];
        let selected = &span.text[sel_start - span.start..sel_end - span.start];
        let x0 = text_x + cx.backend.measure_text(before, CODE_FONT_SIZE);
        let mut width = cx.backend.measure_text(selected, CODE_FONT_SIZE);
        if includes_line_break {
            width += CODE_CHAR_W;
        }
        if width <= 0.0 {
            width = CODE_CHAR_W;
        }
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(x0, line.baseline_y - 10.0),
                size: Point2D::new(width, CODE_LINE_H - 2.0),
            },
            crate::widgets::text_selection::selection_color(theme),
        );
    }
}

#[derive(Clone, Copy)]
struct CodeLineSpan<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct VisibleCodeLine<'a> {
    index: usize,
    span: CodeLineSpan<'a>,
    baseline_y: f32,
}

fn line_spans(code: &str) -> Vec<CodeLineSpan<'_>> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    loop {
        if start > code.len() {
            break;
        }
        let rest = &code[start..];
        let newline = rest.find('\n');
        let end = start + newline.unwrap_or(rest.len());
        spans.push(CodeLineSpan {
            text: &code[start..end],
            start,
            end,
        });
        let Some(newline) = newline else {
            break;
        };
        start += newline + 1;
    }
    spans
}

fn visible_line_spans(code: &str, rect: Rect, scroll: f32) -> Vec<VisibleCodeLine<'_>> {
    let lines = line_spans(code);
    let first = (scroll / CODE_LINE_H).floor().max(0.0) as usize;
    if first >= lines.len() {
        return Vec::new();
    }
    let viewport_h = (rect.size.y - CODE_PAD_Y * 2.0).max(0.0);
    let visible_count = (viewport_h / CODE_LINE_H).ceil().max(0.0) as usize + 2;
    let y_offset = -(scroll % CODE_LINE_H);
    let first_baseline = rect.origin.y + CODE_PAD_Y + 11.0 + y_offset;
    lines
        .into_iter()
        .enumerate()
        .skip(first)
        .take(visible_count)
        .map(|(index, span)| VisibleCodeLine {
            index,
            span,
            baseline_y: first_baseline + (index - first) as f32 * CODE_LINE_H,
        })
        .collect()
}

fn max_scroll_for_code(code: &str, rect: Rect) -> f32 {
    let line_count = line_spans(code).len().max(1) as f32;
    let content_h = line_count * CODE_LINE_H;
    let viewport_h = (rect.size.y - CODE_PAD_Y * 2.0).max(0.0);
    (content_h - viewport_h).max(0.0)
}

fn byte_offset_for_col(line: &str, col: usize) -> usize {
    if col == 0 {
        return 0;
    }
    for (current, (idx, _)) in line.char_indices().enumerate() {
        if current == col {
            return idx;
        }
    }
    line.len()
}

fn code_area_height(state: &CodegenState, layout: CompleteLayout) -> f32 {
    code_area_height_from_y(
        code_area_y(state, layout.y, layout.progress_row_h),
        layout.panel_bottom,
    )
}

fn code_area_height_from_y(code_y: f32, panel_bottom: Option<f32>) -> f32 {
    let Some(panel_bottom) = panel_bottom else {
        return CODE_AREA_MIN_H;
    };
    let available = panel_bottom - code_y - ACTION_ROW_GAP - ACTION_CHIP_H - BODY_BOTTOM_PAD;
    available.max(CODE_AREA_MIN_H)
}

fn code_area_rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(w - PAD_X * 2.0, h),
    }
}

fn code_text_x(rect: Rect) -> f32 {
    rect.origin.x + CODE_GUTTER_W + CODE_PAD_X + CODE_TEXT_GAP
}

fn code_text_top(rect: Rect) -> f32 {
    rect.origin.y + CODE_PAD_Y
}

fn paint_highlighted_code_line(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    line: &str,
    mut x: f32,
    baseline_y: f32,
) {
    for token in syntax_tokens(line) {
        if !token.text.is_empty() {
            let layout = TextLayout::single_run(
                token.text,
                "monospace",
                CODE_FONT_SIZE,
                to_jian_color(syntax_color(theme, token.kind)),
                origin(),
            );
            cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
            x += cx.backend.measure_text(token.text, CODE_FONT_SIZE);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntaxKind {
    Plain,
    Keyword,
    String,
    Number,
    Comment,
    Punctuation,
    Tag,
}

#[derive(Debug, Clone, Copy)]
struct SyntaxToken<'a> {
    text: &'a str,
    kind: SyntaxKind,
}

fn syntax_tokens(line: &str) -> Vec<SyntaxToken<'_>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let bytes = line.as_bytes();
    while i < line.len() {
        let ch = line[i..].chars().next().unwrap_or_default();
        if ch.is_whitespace() {
            let start = i;
            i += ch.len_utf8();
            while i < line.len() {
                let next = line[i..].chars().next().unwrap_or_default();
                if !next.is_whitespace() {
                    break;
                }
                i += next.len_utf8();
            }
            out.push(SyntaxToken {
                text: &line[start..i],
                kind: SyntaxKind::Plain,
            });
        } else if bytes.get(i) == Some(&b'/') && bytes.get(i + 1) == Some(&b'/') {
            out.push(SyntaxToken {
                text: &line[i..],
                kind: SyntaxKind::Comment,
            });
            break;
        } else if matches!(ch, '"' | '\'' | '`') {
            let start = i;
            let quote = ch;
            i += ch.len_utf8();
            let mut escaped = false;
            while i < line.len() {
                let next = line[i..].chars().next().unwrap_or_default();
                i += next.len_utf8();
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    break;
                }
            }
            out.push(SyntaxToken {
                text: &line[start..i],
                kind: SyntaxKind::String,
            });
        } else if ch.is_ascii_digit() {
            let start = i;
            i += ch.len_utf8();
            while i < line.len() {
                let next = line[i..].chars().next().unwrap_or_default();
                if !(next.is_ascii_digit() || next == '.') {
                    break;
                }
                i += next.len_utf8();
            }
            out.push(SyntaxToken {
                text: &line[start..i],
                kind: SyntaxKind::Number,
            });
        } else if is_ident_start(ch) {
            let start = i;
            i += ch.len_utf8();
            while i < line.len() {
                let next = line[i..].chars().next().unwrap_or_default();
                if !is_ident_continue(next) {
                    break;
                }
                i += next.len_utf8();
            }
            let text = &line[start..i];
            let kind = if is_keyword(text) {
                SyntaxKind::Keyword
            } else if follows_jsx_open(line, start) {
                SyntaxKind::Tag
            } else {
                SyntaxKind::Plain
            };
            out.push(SyntaxToken { text, kind });
        } else {
            let start = i;
            i += ch.len_utf8();
            out.push(SyntaxToken {
                text: &line[start..i],
                kind: SyntaxKind::Punctuation,
            });
        }
    }
    out
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

fn is_keyword(text: &str) -> bool {
    matches!(
        text,
        "as" | "async"
            | "await"
            | "break"
            | "case"
            | "class"
            | "const"
            | "continue"
            | "default"
            | "else"
            | "export"
            | "extends"
            | "false"
            | "for"
            | "from"
            | "function"
            | "if"
            | "import"
            | "interface"
            | "let"
            | "new"
            | "null"
            | "return"
            | "switch"
            | "true"
            | "type"
            | "undefined"
            | "var"
            | "while"
    )
}

fn follows_jsx_open(line: &str, start: usize) -> bool {
    let before = line[..start].trim_end();
    before.ends_with('<') || before.ends_with("</")
}

fn code_surface(theme: &Theme) -> Color {
    mix(theme.card, theme.muted, 0.64)
}

fn code_gutter_rule(theme: &Theme) -> Color {
    Color {
        a: 0.72,
        ..theme.border
    }
}

fn code_line_number_color(theme: &Theme) -> Color {
    Color {
        a: 0.58,
        ..theme.muted_foreground
    }
}

fn syntax_color(theme: &Theme, kind: SyntaxKind) -> Color {
    let light = theme.background.r > 0.5;
    match kind {
        SyntaxKind::Plain => theme.foreground,
        SyntaxKind::Keyword => theme.primary,
        SyntaxKind::String => {
            if light {
                rgb(0x0f, 0x7b, 0x4f)
            } else {
                rgb(0x8c, 0xd8, 0x9c)
            }
        }
        SyntaxKind::Number => {
            if light {
                rgb(0xb4, 0x53, 0x09)
            } else {
                rgb(0xf6, 0xb7, 0x3c)
            }
        }
        SyntaxKind::Comment => Color {
            a: 0.78,
            ..theme.muted_foreground
        },
        SyntaxKind::Punctuation => theme.muted_foreground,
        SyntaxKind::Tag => {
            if light {
                rgb(0x7c, 0x3a, 0xed)
            } else {
                rgb(0xc4, 0xb5, 0xfd)
            }
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

fn mix(a: Color, b: Color, b_amount: f32) -> Color {
    let b_amount = b_amount.clamp(0.0, 1.0);
    let a_amount = 1.0 - b_amount;
    Color {
        r: a.r * a_amount + b.r * b_amount,
        g: a.g * a_amount + b.g * b_amount,
        b: a.b * a_amount + b.b * b_amount,
        a: a.a * a_amount + b.a * b_amount,
    }
}

fn paint_action_chip(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    icon: Icon,
    label: &str,
    rect: Rect,
    hovered: bool,
) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.card);
    if hovered {
        cx.backend
            .fill_round_rect(rect, INPUT_RADIUS, theme.button_hover);
    }
    cx.backend
        .stroke_round_rect(rect, INPUT_RADIUS, theme.border, 1.0);
    let cy = rect.origin.y + rect.size.y / 2.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            rect.origin.x + ACTION_CHIP_PAD_X,
            cy - ACTION_CHIP_ICON_SIZE / 2.0,
        ),
        ACTION_CHIP_ICON_SIZE,
        theme.foreground,
        1.4,
    );
    draw_line(
        cx,
        label,
        theme.foreground,
        rect.origin.x + ACTION_CHIP_PAD_X + ACTION_CHIP_ICON_SIZE + ACTION_CHIP_TEXT_GAP,
        cy + 4.5,
    );
}

fn action_chip_width(label: &str) -> f32 {
    let label_w = label.chars().count() as f32 * ACTION_LABEL_FONT_SIZE * 0.55;
    ACTION_CHIP_PAD_X * 2.0 + ACTION_CHIP_ICON_SIZE + ACTION_CHIP_TEXT_GAP + label_w
}
