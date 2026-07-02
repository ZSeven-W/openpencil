use op_editor_core::chat::ChatToolCall;
use serde_json::Value;

use super::ai_chat_transcript::wrap_units;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

const GROUP_HEADER_H: f32 = 22.0;
/// #27 restyle: comfortable 48px card header (was 24px).
const CARD_HEADER_H: f32 = 48.0;
const CARD_GAP: f32 = 4.0;
const CARD_PAD_X: f32 = 8.0;
const CARD_PAD_Y: f32 = 6.0;
const CARD_LINE_H: f32 = 14.0;
/// Radius for the #27 rounded-rect card style (~10px per spec).
const CARD_RADIUS: f32 = 10.0;
/// X-offset of the left-aligned card label.
const CARD_LABEL_X: f32 = 12.0;
/// X-offset from the card right edge to the status ring center.
const CARD_STATUS_RIGHT_OFFSET: f32 = 44.0;
/// X-offset from the card right edge to the chevron top-left.
const CARD_CHEVRON_RIGHT_OFFSET: f32 = 24.0;
/// Radius of the ✓-ring that surrounds the check icon on completed cards.
const CHECK_RING_R: f32 = 9.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolLevel {
    Read,
    Create,
    Modify,
    Delete,
    Orchestrate,
}

impl ToolLevel {
    fn from_name(name: &str) -> Self {
        match name {
            "delete_node" | "remove_page" => Self::Delete,
            "update_node" | "replace_node" | "move_node" | "set_variables" | "set_themes"
            | "load_theme_preset" | "rename_page" | "reorder_page" | "batch_design"
            | "set_design_md" | "export_design_md" => Self::Modify,
            "plan_layout" | "batch_insert" | "insert_node" | "add_page" | "duplicate_page"
            | "import_svg" | "copy_node" | "save_theme_preset" | "generate_design" => Self::Create,
            _ => Self::Read,
        }
    }

    fn from_value(value: Option<&Value>, name: &str) -> Self {
        match value.and_then(Value::as_str) {
            Some("create") => Self::Create,
            Some("modify") => Self::Modify,
            Some("delete") => Self::Delete,
            Some("orchestrate") => Self::Orchestrate,
            Some("read") => Self::Read,
            _ => Self::from_name(name),
        }
    }

    fn default_open(self) -> bool {
        matches!(self, Self::Modify | Self::Delete | Self::Orchestrate)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolCallCard {
    pub rect: Rect,
    pub header: Rect,
    pub body: Rect,
    pub expanded: bool,
    pub name: String,
    source: Option<String>,
    status: String,
    level: ToolLevel,
    result_failed: bool,
    arg_lines: Vec<String>,
    result_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolPanel {
    pub header: Rect,
    pub label: String,
    pub collapsed: bool,
    pub body: Rect,
    pub lines: Vec<String>,
    pub cards: Vec<ToolCallCard>,
}

pub(crate) struct ToolPanelLayout<'a> {
    pub collapsed: bool,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub budget: u32,
    pub default_status: &'a str,
    pub expanded_overrides: &'a [Option<bool>],
}

/// Format tool-call process cards with the same high-level semantics
/// surfaced by the TS chat: source, status, result, and call args.
pub(crate) fn tool_lines(calls: &[ChatToolCall], budget: u32, default_status: &str) -> Vec<String> {
    const MAX_ARG_LINES: usize = 8;
    let mut lines = Vec::new();
    for c in calls {
        lines.push(format!("→ {}", c.name));
        let card = ToolCardFields::from_call(&c.name, &c.args, default_status);
        if let Some(source) = card.source.as_deref() {
            lines.push(format!("  Source: {source}"));
        }
        lines.push(format!("  Status: {}", card.status));
        if let Some(result) = card.result.as_deref() {
            lines.push(format!("  Result: {result}"));
        }
        if !card.args.is_empty() {
            push_wrapped_tool_line(&mut lines, "Args", &card.args, budget, MAX_ARG_LINES);
        }
    }
    lines
}

pub(crate) fn build_tool_panel(
    calls: &[ChatToolCall],
    layout: ToolPanelLayout<'_>,
) -> (Option<ToolPanel>, f32) {
    let ToolPanelLayout {
        collapsed,
        label,
        x,
        mut y,
        width,
        budget,
        default_status,
        expanded_overrides,
    } = layout;
    if calls.is_empty() {
        return (None, y);
    }
    let header = Rect::xywh(x, y, width, GROUP_HEADER_H);
    y += GROUP_HEADER_H;
    if collapsed {
        y += CARD_GAP;
        return (
            Some(ToolPanel {
                header,
                label,
                collapsed,
                body: Rect::xywh(x, y, width, 0.0),
                lines: Vec::new(),
                cards: Vec::new(),
            }),
            y,
        );
    }

    let body_top = y;
    let mut cards = Vec::new();
    for (index, call) in calls.iter().enumerate() {
        let expanded_override = expanded_overrides.get(index).copied().flatten();
        let (card, next_y) =
            layout_tool_card(call, x, y, width, budget, default_status, expanded_override);
        cards.push(card);
        y = next_y + CARD_GAP;
    }
    let body_h = (y - CARD_GAP - body_top).max(0.0);
    y += CARD_GAP;
    (
        Some(ToolPanel {
            header,
            label,
            collapsed,
            body: Rect::xywh(x, body_top, width, body_h),
            lines: tool_lines(calls, budget, default_status),
            cards,
        }),
        y,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolCardFields {
    source: Option<String>,
    status: String,
    result: Option<String>,
    args: String,
    level: ToolLevel,
    result_failed: bool,
}

impl ToolCardFields {
    fn from_call(name: &str, raw_args: &str, default_status: &str) -> Self {
        let compact: String = raw_args.split_whitespace().collect::<Vec<_>>().join(" ");
        let Ok(value) = serde_json::from_str::<Value>(&compact) else {
            return Self {
                source: None,
                status: default_status.to_string(),
                result: None,
                args: compact,
                level: ToolLevel::from_name(name),
                result_failed: false,
            };
        };
        let Some(obj) = value.as_object() else {
            return Self {
                source: None,
                status: default_status.to_string(),
                result: None,
                args: compact_json(&value),
                level: ToolLevel::from_name(name),
                result_failed: false,
            };
        };

        let level = ToolLevel::from_value(obj.get("level"), name);
        let source = obj
            .get("source")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && *s != "lead")
            .map(str::to_string);
        let result_value = obj.get("result");
        let result_failed = result_value
            .and_then(|r| r.get("success"))
            .and_then(Value::as_bool)
            == Some(false);
        let result = result_value.and_then(result_summary);
        let status = obj
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| status_from_result(result_value))
            .unwrap_or_else(|| default_status.to_string());
        let args = obj.get("args").map(compact_json).unwrap_or(compact);

        Self {
            source,
            status,
            result,
            args,
            level,
            result_failed,
        }
    }
}

fn layout_tool_card(
    call: &ChatToolCall,
    x: f32,
    y: f32,
    width: f32,
    budget: u32,
    default_status: &str,
    expanded_override: Option<bool>,
) -> (ToolCallCard, f32) {
    const MAX_BODY_LINES: usize = 8;
    let fields = ToolCardFields::from_call(&call.name, &call.args, default_status);
    let expanded = expanded_override.unwrap_or_else(|| fields.level.default_open());
    let arg_lines = if expanded && !fields.args.is_empty() {
        prefixed_wrapped_lines("Args", &fields.args, budget, MAX_BODY_LINES)
    } else {
        Vec::new()
    };
    let result_failed = fields.status == "error" || fields.result_failed;
    let result_lines = if expanded {
        fields
            .result
            .as_deref()
            .map(|result| prefixed_wrapped_lines("Result", result, budget, MAX_BODY_LINES))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let body_lines = arg_lines.len() + result_lines.len();
    let body_h = if body_lines == 0 {
        0.0
    } else {
        body_lines as f32 * CARD_LINE_H + 2.0 * CARD_PAD_Y
    };
    let rect = Rect::xywh(x, y, width, CARD_HEADER_H + body_h);
    let header = Rect::xywh(x, y, width, CARD_HEADER_H);
    let body = Rect::xywh(x, y + CARD_HEADER_H, width, body_h);
    (
        ToolCallCard {
            rect,
            header,
            body,
            expanded,
            name: call.name.clone(),
            source: fields.source,
            status: fields.status,
            level: fields.level,
            result_failed,
            arg_lines,
            result_lines,
        },
        y + rect.size.y,
    )
}

fn prefixed_wrapped_lines(label: &str, text: &str, budget: u32, max_lines: usize) -> Vec<String> {
    let wrapped = wrap_units(text, budget.saturating_sub(8).max(1));
    let shown = wrapped.len().min(max_lines);
    let mut lines = Vec::new();
    for (i, line) in wrapped[..shown].iter().enumerate() {
        if i == 0 {
            lines.push(format!("{label}: {line}"));
        } else {
            lines.push(format!("  {line}"));
        }
    }
    if wrapped.len() > shown {
        lines.push("…".to_string());
    }
    lines
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

fn status_from_result(result: Option<&Value>) -> Option<String> {
    match result?.get("success").and_then(Value::as_bool) {
        Some(true) => Some("done".into()),
        Some(false) => Some("error".into()),
        None => None,
    }
}

fn result_summary(result: &Value) -> Option<String> {
    if result.get("success").and_then(Value::as_bool) == Some(false) {
        return result
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some(compact_json(result)));
    }
    result
        .get("data")
        .map(compact_json)
        .or_else(|| Some(compact_json(result)))
}

fn push_wrapped_tool_line(
    lines: &mut Vec<String>,
    label: &str,
    text: &str,
    budget: u32,
    max_lines: usize,
) {
    let wrapped = wrap_units(text, budget.saturating_sub(8).max(1));
    let shown = wrapped.len().min(max_lines);
    for (i, line) in wrapped[..shown].iter().enumerate() {
        if i == 0 {
            lines.push(format!("  {label}: {line}"));
        } else {
            lines.push(format!("  {line}"));
        }
    }
    if wrapped.len() > shown {
        lines.push("  …".to_string());
    }
}

pub(crate) fn paint_tool_panel(cx: &mut PaintCx<'_>, theme: &Theme, panel: &ToolPanel) {
    cx.backend.fill_round_rect(panel.header, 6.0, theme.muted);
    let icon = if panel.collapsed {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            panel.header.origin.x + 6.0,
            panel.header.origin.y + (GROUP_HEADER_H - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    draw_line(
        cx,
        &panel.label,
        panel.header.origin.x + 26.0,
        panel.header.origin.y + GROUP_HEADER_H / 2.0 + 4.0,
        11.0,
        theme.muted_foreground,
    );
    for card in &panel.cards {
        paint_tool_card(cx, theme, card);
    }
}

fn paint_tool_card(cx: &mut PaintCx<'_>, theme: &Theme, card: &ToolCallCard) {
    // #27 restyle: elevated-dark fill with subtle 1px border, ~10px radius.
    let mut bg = theme.card;
    let mut border = theme.border;
    if card.level == ToolLevel::Delete {
        border = theme.destructive;
        border.a *= 0.35;
        bg = theme.destructive;
        bg.a *= 0.08;
    }
    cx.backend.fill_round_rect(card.rect, CARD_RADIUS, bg);
    cx.backend
        .stroke_round_rect(card.rect, CARD_RADIUS, border, 1.0);

    // Label: left-aligned, muted-bright color, vertically centered.
    let text_color = if card.level == ToolLevel::Delete {
        theme.destructive
    } else {
        theme.muted_foreground
    };
    let label_x = card.header.origin.x + CARD_LABEL_X;
    let label_baseline = card.header.origin.y + CARD_HEADER_H / 2.0 + 4.0;
    draw_line(cx, &card.name, label_x, label_baseline, 11.0, text_color);

    // Source badge — sits just left of the status icon when present.
    if let Some(source) = card.source.as_deref() {
        paint_source_badge(cx, theme, card, source);
    }

    // Status ring + icon: positioned before the chevron.
    paint_status_icon(cx, theme, card);

    // Expand/collapse chevron at the far right, vertically centered.
    let chevron_icon = if card.expanded {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    draw_icon(
        cx.backend,
        chevron_icon,
        Point2D::new(
            card.header.origin.x + card.header.size.x - CARD_CHEVRON_RIGHT_OFFSET,
            card.header.origin.y + (CARD_HEADER_H - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );

    if card.body.size.y > 0.0 {
        cx.backend.stroke_line(
            Point2D::new(card.body.origin.x, card.body.origin.y),
            Point2D::new(card.body.origin.x + card.body.size.x, card.body.origin.y),
            theme.border,
            1.0,
        );
        cx.backend.save();
        cx.backend.clip_rect(card.body);
        let mut baseline = card.body.origin.y + CARD_PAD_Y + 9.0;
        for line in card.arg_lines.iter().chain(card.result_lines.iter()) {
            let color = if card.result_failed && line.starts_with("Result:") {
                theme.destructive
            } else {
                theme.muted_foreground
            };
            draw_line(
                cx,
                line,
                card.body.origin.x + CARD_PAD_X,
                baseline,
                10.0,
                color,
            );
            baseline += CARD_LINE_H;
        }
        cx.backend.restore();
    }
}

fn paint_source_badge(cx: &mut PaintCx<'_>, theme: &Theme, card: &ToolCallCard, source: &str) {
    let badge_w = (source.chars().count() as f32 * 5.5 + 10.0).min(96.0);
    let badge = Rect::xywh(
        card.header.origin.x + card.header.size.x - badge_w - 34.0,
        card.header.origin.y + 6.0,
        badge_w,
        12.0,
    );
    let mut bg = theme.primary;
    bg.a *= 0.12;
    cx.backend.fill_round_rect(badge, 3.0, bg);
    cx.backend.save();
    cx.backend.clip_rect(badge);
    draw_line(
        cx,
        source,
        badge.origin.x + 5.0,
        badge.origin.y + 9.0,
        9.0,
        theme.primary,
    );
    cx.backend.restore();
}

/// Paint the per-card status indicator.
///
/// For completed (done) cards: a thin-ring circle in success-green with
/// the `check` glyph inside — matching the #27 reference look. For
/// running/pending: a muted loader spinner. For error: a red ✕ icon.
/// The icon top-left is placed at `(x + w - CARD_STATUS_RIGHT_OFFSET,
/// y + (CARD_HEADER_H - 14) / 2)`.
fn paint_status_icon(cx: &mut PaintCx<'_>, theme: &Theme, card: &ToolCallCard) {
    let icon_top_left = Point2D::new(
        card.header.origin.x + card.header.size.x - CARD_STATUS_RIGHT_OFFSET,
        card.header.origin.y + (CARD_HEADER_H - 14.0) / 2.0,
    );

    let is_error = card.status == "error" || card.result_failed;
    let is_pending = matches!(card.status.as_str(), "running" | "pending");

    if is_pending {
        draw_icon(
            cx.backend,
            Icon::Loader,
            icon_top_left,
            14.0,
            theme.muted_foreground,
            1.5,
        );
    } else if is_error {
        draw_icon(
            cx.backend,
            Icon::Close,
            icon_top_left,
            14.0,
            theme.destructive,
            1.5,
        );
    } else {
        // Done: thin success-green ring + check glyph inside (#27 reference).
        let success_color = theme.status_success;
        let ring_cx = icon_top_left.x + 7.0;
        let ring_cy = icon_top_left.y + 7.0;
        cx.backend.stroke_oval(
            Rect::xywh(
                ring_cx - CHECK_RING_R,
                ring_cy - CHECK_RING_R,
                CHECK_RING_R * 2.0,
                CHECK_RING_R * 2.0,
            ),
            success_color,
            1.0,
        );
        draw_icon(
            cx.backend,
            Icon::Check,
            icon_top_left,
            14.0,
            success_color,
            1.5,
        );
    }
}

fn draw_line(
    cx: &mut PaintCx<'_>,
    text: &str,
    x: f32,
    baseline_y: f32,
    size: f32,
    color: crate::Color,
) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[test]
    fn done_tool_status_uses_check_icon_with_success_green_ring() {
        // #27 restyle: done status uses the theme's success-green (0x3fb950)
        // with a ✓ check icon + thin ring. Old constant tailwind-green-500
        // (0x22c55e) is replaced by Theme::dark().status_success.
        let theme = Theme::dark();
        let mut backend = StatusIconBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let card = test_card("done", false);

        paint_tool_card(&mut cx, &theme, &card);

        assert_eq!(backend.status_paths, Icon::Check.paths());
        assert_eq!(backend.status_colors, vec![theme.status_success]);
    }

    #[test]
    fn failed_tool_status_uses_ts_plain_x_icon() {
        let mut backend = StatusIconBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let card = test_card("error", true);

        paint_tool_card(&mut cx, &Theme::dark(), &card);

        assert_eq!(backend.status_paths, Icon::Close.paths());
        assert_eq!(backend.status_colors, vec![Theme::dark().destructive]);
    }

    #[test]
    fn expanded_tool_card_paints_ts_body_separator() {
        let mut backend = StatusIconBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let card = expanded_test_card();
        let theme = Theme::dark();

        paint_tool_card(&mut cx, &theme, &card);

        assert_eq!(
            backend.separator_lines,
            vec![(
                Point2D::new(card.body.origin.x, card.body.origin.y),
                Point2D::new(card.body.origin.x + card.body.size.x, card.body.origin.y),
                theme.border,
                1.0,
            )]
        );
    }

    fn test_card(status: &str, result_failed: bool) -> ToolCallCard {
        ToolCallCard {
            rect: Rect::xywh(10.0, 10.0, 100.0, 24.0),
            header: Rect::xywh(10.0, 10.0, 100.0, 24.0),
            body: Rect::xywh(10.0, 34.0, 100.0, 0.0),
            expanded: false,
            name: "update_node".into(),
            source: None,
            status: status.into(),
            level: ToolLevel::Modify,
            result_failed,
            arg_lines: Vec::new(),
            result_lines: Vec::new(),
        }
    }

    fn expanded_test_card() -> ToolCallCard {
        ToolCallCard {
            rect: Rect::xywh(10.0, 10.0, 100.0, 50.0),
            header: Rect::xywh(10.0, 10.0, 100.0, 24.0),
            body: Rect::xywh(10.0, 34.0, 100.0, 26.0),
            expanded: true,
            name: "update_node".into(),
            source: None,
            status: "done".into(),
            level: ToolLevel::Modify,
            result_failed: false,
            arg_lines: vec!["Args: {}".into()],
            result_lines: Vec::new(),
        }
    }

    #[derive(Default)]
    struct StatusIconBackend {
        status_paths: Vec<&'static str>,
        status_colors: Vec<Color>,
        separator_lines: Vec<(Point2D, Point2D, Color, f32)>,
    }

    impl RenderBackend for StatusIconBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
            self.separator_lines.push((from, to, color, width));
        }
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, d: &str, at: Point2D, _: f32, color: Color, _: f32) {
            // Status icon position after #27 restyle:
            // X = header.origin.x + header.size.x - CARD_STATUS_RIGHT_OFFSET (44)
            //   = 10 + 100 - 44 = 66
            // Y = header.origin.y + (CARD_HEADER_H - 14) / 2
            //   = 10 + (48 - 14) / 2 = 27
            if (at.x - 66.0).abs() < 0.01 && (at.y - 27.0).abs() < 0.01 {
                let path = Icon::Check
                    .paths()
                    .iter()
                    .chain(Icon::Close.paths().iter())
                    .chain(Icon::CheckCircle.paths().iter())
                    .chain(Icon::XCircle.paths().iter())
                    .chain(Icon::Loader.paths().iter())
                    .copied()
                    .find(|path| *path == d)
                    .expect("status icon path should be static icon data");
                self.status_paths.push(path);
                if self.status_colors.last().copied() != Some(color) {
                    self.status_colors.push(color);
                }
            }
        }
        fn fill_oval(&mut self, _: Rect, _: Color) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }
}
