use op_editor_core::chat::ChatToolCall;
use serde_json::Value;

use super::ai_chat_panel::to_jian_color;
use super::ai_chat_transcript::wrap_units;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

const GROUP_HEADER_H: f32 = 22.0;
const CARD_HEADER_H: f32 = 24.0;
const CARD_GAP: f32 = 4.0;
const CARD_PAD_X: f32 = 8.0;
const CARD_PAD_Y: f32 = 6.0;
const CARD_LINE_H: f32 = 14.0;
const STATUS_SUCCESS_GREEN: crate::Color = crate::Color {
    r: 0x22 as f32 / 255.0,
    g: 0xc5 as f32 / 255.0,
    b: 0x5e as f32 / 255.0,
    a: 1.0,
};

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

    fn icon(self) -> Icon {
        match self {
            Self::Read => Icon::Search,
            Self::Create => Icon::Plus,
            Self::Modify => Icon::Pencil,
            Self::Delete => Icon::Trash,
            Self::Orchestrate => Icon::Wrench,
        }
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
    let mut bg = theme.card;
    bg.a *= 0.5;
    let mut border = theme.border;
    if card.level == ToolLevel::Delete {
        border = theme.destructive;
        border.a *= 0.35;
        bg = theme.destructive;
        bg.a *= 0.08;
    }
    cx.backend.fill_round_rect(card.rect, 6.0, bg);
    cx.backend.stroke_round_rect(card.rect, 6.0, border, 1.0);
    draw_icon(
        cx.backend,
        if card.expanded {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        Point2D::new(card.header.origin.x + 7.0, card.header.origin.y + 6.0),
        12.0,
        theme.muted_foreground,
        1.4,
    );
    let accent = match card.level {
        ToolLevel::Delete => theme.destructive,
        ToolLevel::Orchestrate => theme.primary,
        ToolLevel::Read => theme.muted_foreground,
        ToolLevel::Create | ToolLevel::Modify => theme.foreground,
    };
    draw_icon(
        cx.backend,
        card.level.icon(),
        Point2D::new(card.header.origin.x + 24.0, card.header.origin.y + 5.0),
        14.0,
        accent,
        1.5,
    );
    let text_color = if card.level == ToolLevel::Delete {
        theme.destructive
    } else {
        theme.foreground
    };
    draw_line(
        cx,
        &card.name,
        card.header.origin.x + 44.0,
        card.header.origin.y + 16.0,
        11.0,
        text_color,
    );
    if let Some(source) = card.source.as_deref() {
        paint_source_badge(cx, theme, card, source);
    }
    paint_status_icon(cx, theme, card);
    if card.body.size.y > 0.0 {
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

fn paint_status_icon(cx: &mut PaintCx<'_>, theme: &Theme, card: &ToolCallCard) {
    let (icon, color) = match card.status.as_str() {
        "running" | "pending" => (Icon::Loader, theme.muted_foreground),
        "error" => (Icon::Close, theme.destructive),
        _ if card.result_failed => (Icon::Close, theme.destructive),
        _ => (Icon::Check, STATUS_SUCCESS_GREEN),
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            card.header.origin.x + card.header.size.x - 20.0,
            card.header.origin.y + 5.0,
        ),
        14.0,
        color,
        1.5,
    );
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
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[test]
    fn orchestrate_tool_level_uses_ts_wrench_icon() {
        let wrench = Icon::from_name("wrench").expect("wrench icon should be available");

        assert_eq!(ToolLevel::Orchestrate.icon(), wrench);
    }

    #[test]
    fn done_tool_status_uses_ts_plain_check_icon() {
        let mut backend = StatusIconBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let card = test_card("done", false);

        paint_tool_card(&mut cx, &Theme::dark(), &card);

        assert_eq!(backend.status_paths, Icon::Check.paths());
        assert_eq!(backend.status_colors, vec![tailwind_green_500()]);
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

    #[derive(Default)]
    struct StatusIconBackend {
        status_paths: Vec<&'static str>,
        status_colors: Vec<Color>,
    }

    impl RenderBackend for StatusIconBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, d: &str, at: Point2D, _: f32, color: Color, _: f32) {
            if (at.x - 90.0).abs() < 0.01 && (at.y - 15.0).abs() < 0.01 {
                let path = Icon::Check
                    .paths()
                    .iter()
                    .chain(Icon::Close.paths().iter())
                    .chain(Icon::CheckCircle.paths().iter())
                    .chain(Icon::XCircle.paths().iter())
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

    fn tailwind_green_500() -> Color {
        Color {
            r: 0x22 as f32 / 255.0,
            g: 0xc5 as f32 / 255.0,
            b: 0x5e as f32 / 255.0,
            a: 1.0,
        }
    }
}
