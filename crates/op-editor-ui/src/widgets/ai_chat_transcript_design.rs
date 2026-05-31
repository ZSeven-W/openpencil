use super::ai_chat_panel::to_jian_color;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

pub(crate) const DESIGN_BLOCK_H: f32 = 32.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PendingDesignBlock {
    pub element_count: usize,
    pub label: String,
    pub streaming: bool,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesignBlock {
    pub rect: Rect,
    pub header: Rect,
    pub body: Rect,
    pub expanded: bool,
    pub element_count: usize,
    pub label: String,
    pub streaming: bool,
    pub code_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesignExtraction {
    pub visible_text: String,
    pub blocks: Vec<PendingDesignBlock>,
}

pub(crate) fn extract_design_json_blocks(text: &str, is_streaming: bool) -> DesignExtraction {
    let mut visible = Vec::new();
    let mut blocks = Vec::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code = String::new();

    for line in text.lines() {
        if line.starts_with("```") && !in_code {
            in_code = true;
            code_lang = line[3..].trim().to_string();
            code.clear();
            continue;
        }
        if line.starts_with("```") && in_code {
            finish_code_block(&mut visible, &mut blocks, &code_lang, &code, false);
            in_code = false;
            continue;
        }
        if in_code {
            if !code.is_empty() {
                code.push('\n');
            }
            code.push_str(line);
        } else {
            visible.push(line.to_string());
        }
    }

    if in_code {
        finish_code_block(&mut visible, &mut blocks, &code_lang, &code, is_streaming);
    }

    DesignExtraction {
        visible_text: collapse_blank_lines(&visible.join("\n")).trim().to_string(),
        blocks,
    }
}

fn finish_code_block(
    visible: &mut Vec<String>,
    blocks: &mut Vec<PendingDesignBlock>,
    code_lang: &str,
    code: &str,
    streaming: bool,
) {
    if code_lang == "json" && is_design_json(code) {
        let element_count = design_element_count(code);
        let label = if streaming {
            "Generating design...".to_string()
        } else {
            format!(
                "{element_count} design element{}",
                if element_count == 1 { "" } else { "s" }
            )
        };
        blocks.push(PendingDesignBlock {
            element_count,
            label,
            streaming,
            code: code.trim_end().to_string(),
        });
    } else {
        visible.push(format!("```{code_lang}"));
        visible.extend(code.lines().map(str::to_string));
        visible.push("```".to_string());
    }
}

fn is_design_json(code: &str) -> bool {
    let trimmed = code.trim_start();
    (trimmed.starts_with('[') || trimmed.starts_with('{'))
        && code.contains("\"type\"")
        && code.contains("\"id\"")
}

fn design_element_count(code: &str) -> usize {
    match serde_json::from_str::<serde_json::Value>(code) {
        Ok(serde_json::Value::Array(items)) => items.len(),
        Ok(serde_json::Value::Object(_)) => 1,
        _ if code.contains("\"_parent\"") => code
            .lines()
            .filter(|line| line.trim_start().starts_with('{'))
            .count(),
        _ => 0,
    }
}

fn collapse_blank_lines(input: &str) -> String {
    let mut out = Vec::new();
    let mut blank = false;
    for line in input.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            if !blank && !out.is_empty() {
                out.push(String::new());
            }
            blank = true;
        } else {
            out.push(line.to_string());
            blank = false;
        }
    }
    out.join("\n")
}

pub(crate) fn place_design_blocks(
    pending: Vec<PendingDesignBlock>,
    x: f32,
    mut y: f32,
    width: f32,
    gap: f32,
    expanded_overrides: &[Option<bool>],
) -> (Vec<DesignBlock>, f32) {
    const BODY_PAD_Y: f32 = 8.0;
    const BODY_LINE_H: f32 = 13.0;
    const MAX_BODY_LINES: usize = 12;
    let mut blocks = Vec::new();
    for (index, pending) in pending.into_iter().enumerate() {
        let expanded = expanded_overrides
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(false);
        let mut code_lines = Vec::new();
        if expanded {
            code_lines.extend(
                pending
                    .code
                    .lines()
                    .take(MAX_BODY_LINES)
                    .map(str::to_string),
            );
            if pending.code.lines().count() > MAX_BODY_LINES {
                code_lines.push("…".to_string());
            }
        }
        let body_h = if code_lines.is_empty() {
            0.0
        } else {
            BODY_PAD_Y * 2.0 + BODY_LINE_H * code_lines.len() as f32
        };
        let rect = Rect::xywh(x, y, width, DESIGN_BLOCK_H + body_h);
        let header = Rect::xywh(x, y, width, DESIGN_BLOCK_H);
        let body = Rect::xywh(x, y + DESIGN_BLOCK_H, width, body_h);
        blocks.push(DesignBlock {
            rect,
            header,
            body,
            expanded,
            element_count: pending.element_count,
            label: pending.label,
            streaming: pending.streaming,
            code_lines,
        });
        y += DESIGN_BLOCK_H + body_h + gap;
    }
    (blocks, y)
}

pub(crate) fn paint_design_block(cx: &mut PaintCx<'_>, theme: &Theme, block: &DesignBlock) {
    let mut fill = theme.background;
    fill.a *= 0.4;
    let mut border = theme.border;
    border.a *= 0.35;
    cx.backend.fill_round_rect(block.rect, 6.0, fill);
    cx.backend.stroke_round_rect(block.rect, 6.0, border, 1.0);

    let mut icon_bg = theme.primary;
    icon_bg.a *= 0.12;
    cx.backend.fill_round_rect(
        Rect::xywh(
            block.rect.origin.x + 8.0,
            block.rect.origin.y + 8.0,
            16.0,
            16.0,
        ),
        8.0,
        icon_bg,
    );
    draw_icon(
        cx.backend,
        Icon::Sparkles,
        Point2D::new(block.rect.origin.x + 10.0, block.rect.origin.y + 10.0),
        12.0,
        theme.primary,
        1.5,
    );
    let mut color = if block.streaming {
        theme.muted_foreground
    } else {
        theme.foreground
    };
    if block.streaming {
        color.a *= 0.85;
    }
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        block.header.origin.x + 30.0,
        block.header.origin.y,
        (block.header.size.x - 58.0).max(1.0),
        block.header.size.y,
    ));
    let layout = TextLayout::single_run(
        &block.label,
        "system-ui",
        11.0,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(block.header.origin.x + 30.0, block.header.origin.y + 20.0),
    );
    cx.backend.restore();

    let mut chevron_color = theme.muted_foreground;
    chevron_color.a *= 0.45;
    draw_icon(
        cx.backend,
        if block.expanded {
            Icon::ChevronUp
        } else {
            Icon::ChevronDown
        },
        Point2D::new(
            block.header.origin.x + block.header.size.x - 20.0,
            block.header.origin.y + 10.0,
        ),
        12.0,
        chevron_color,
        1.5,
    );

    if block.body.size.y > 0.0 {
        let mut divider = theme.border;
        divider.a *= 0.3;
        cx.backend.stroke_line(
            Point2D::new(block.body.origin.x, block.body.origin.y),
            Point2D::new(block.body.origin.x + block.body.size.x, block.body.origin.y),
            divider,
            1.0,
        );
        cx.backend.save();
        cx.backend.clip_rect(block.body);
        let mut baseline = block.body.origin.y + 18.0;
        for line in &block.code_lines {
            let layout = TextLayout::single_run(
                line,
                "monospace",
                9.0,
                to_jian_color(theme.muted_foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&layout, Point2D::new(block.body.origin.x + 10.0, baseline));
            baseline += 13.0;
        }
        cx.backend.restore();
    }
}
