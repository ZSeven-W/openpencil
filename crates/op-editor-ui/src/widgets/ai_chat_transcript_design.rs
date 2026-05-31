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
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesignBlock {
    pub rect: Rect,
    pub element_count: usize,
    pub label: String,
    pub streaming: bool,
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
) -> (Vec<DesignBlock>, f32) {
    let mut blocks = Vec::new();
    for pending in pending {
        let rect = Rect::xywh(x, y, width, DESIGN_BLOCK_H);
        blocks.push(DesignBlock {
            rect,
            element_count: pending.element_count,
            label: pending.label,
            streaming: pending.streaming,
        });
        y += DESIGN_BLOCK_H + gap;
    }
    (blocks, y)
}

pub(crate) fn paint_design_block(cx: &mut PaintCx<'_>, theme: &Theme, block: &DesignBlock) {
    cx.backend.fill_round_rect(block.rect, 6.0, theme.muted);
    cx.backend
        .stroke_round_rect(block.rect, 6.0, theme.border, 1.0);
    draw_icon(
        cx.backend,
        Icon::Sparkles,
        Point2D::new(block.rect.origin.x + 10.0, block.rect.origin.y + 9.0),
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
    let layout = TextLayout::single_run(
        &block.label,
        "system-ui",
        11.0,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(block.rect.origin.x + 30.0, block.rect.origin.y + 20.0),
    );
}
