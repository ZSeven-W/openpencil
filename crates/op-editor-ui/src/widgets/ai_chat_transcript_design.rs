use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use std::collections::BTreeMap;

pub(crate) const DESIGN_BLOCK_H: f32 = 32.0;
const DESIGN_ICON_LEFT: f32 = 12.0;
const DESIGN_ICON_BG: f32 = 16.0;
const DESIGN_ICON_SIZE: f32 = 10.0;
const DESIGN_ICON_GAP: f32 = 10.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PendingDesignBlock {
    pub element_count: usize,
    pub label: String,
    pub streaming: bool,
    pub applied: bool,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesignBlock {
    pub rect: Rect,
    pub header: Rect,
    pub copy: Rect,
    pub body: Rect,
    pub apply: Option<Rect>,
    pub expanded: bool,
    pub element_count: usize,
    pub label: String,
    pub streaming: bool,
    pub code: String,
    pub copy_visible: bool,
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
            applied: false,
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

pub fn parse_design_json_nodes(code: &str) -> Result<Vec<PenNode>, String> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Err("empty design JSON".into());
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return parse_design_value(value);
    }
    parse_jsonl_nodes(trimmed)
}

fn parse_design_value(mut value: serde_json::Value) -> Result<Vec<PenNode>, String> {
    normalize_design_json(&mut value);
    let nodes =
        match value {
            serde_json::Value::Array(_) => serde_json::from_value::<Vec<PenNode>>(value)
                .map_err(|e| format!("deserialize: {e}"))?,
            serde_json::Value::Object(_) => vec![serde_json::from_value::<PenNode>(value)
                .map_err(|e| format!("deserialize: {e}"))?],
            _ => return Err("design JSON must be an object or array".into()),
        };
    if nodes.is_empty() {
        Err("empty design node array".into())
    } else {
        Ok(nodes)
    }
}

fn normalize_design_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_design_json(item);
            }
        }
        serde_json::Value::Object(object) => {
            object.remove("_parent");
            if object
                .get("type")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|ty| ty == "image")
                && matches!(object.get("src"), None | Some(serde_json::Value::Null))
            {
                object.insert("src".into(), serde_json::Value::String(String::new()));
            }
            for child in object.values_mut() {
                normalize_design_json(child);
            }
        }
        _ => {}
    }
}

fn parse_jsonl_nodes(text: &str) -> Result<Vec<PenNode>, String> {
    let objects = extract_json_objects(text)?;
    if objects.is_empty() {
        return Err("no JSON objects found".into());
    }

    let mut nodes_by_id = BTreeMap::<String, PenNode>::new();
    let mut children_by_parent = BTreeMap::<String, Vec<String>>::new();
    let mut parents = Vec::<(String, Option<String>)>::new();

    for mut value in objects {
        let parent = value
            .as_object_mut()
            .and_then(|object| object.remove("_parent"))
            .and_then(|value| value.as_str().map(str::to_string));
        normalize_design_json(&mut value);
        let node =
            serde_json::from_value::<PenNode>(value).map_err(|e| format!("deserialize: {e}"))?;
        let id = node.id_str().to_string();
        parents.push((id.clone(), parent));
        nodes_by_id.insert(id, node);
    }

    let mut root_ids = Vec::new();
    for (id, parent) in parents {
        if let Some(parent_id) = parent.filter(|parent_id| nodes_by_id.contains_key(parent_id)) {
            children_by_parent.entry(parent_id).or_default().push(id);
        } else {
            root_ids.push(id);
        }
    }

    let mut roots = Vec::new();
    for id in root_ids {
        if let Some(node) = build_jsonl_tree(&id, &mut nodes_by_id, &mut children_by_parent) {
            roots.push(node);
        }
    }
    if roots.is_empty() {
        Err("empty design node tree".into())
    } else {
        Ok(roots)
    }
}

fn extract_json_objects(text: &str) -> Result<Vec<serde_json::Value>, String> {
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i] != b'{' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        let mut depth = 0i32;
        let mut in_str = false;
        let mut esc = false;
        while i < bytes.len() {
            let c = bytes[i];
            if in_str {
                if esc {
                    esc = false;
                } else if c == b'\\' {
                    esc = true;
                } else if c == b'"' {
                    in_str = false;
                }
                i += 1;
                continue;
            }
            match c {
                b'"' => in_str = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        let slice = &text[start..=i];
                        let value = serde_json::from_str::<serde_json::Value>(slice)
                            .map_err(|e| format!("json: {e}"))?;
                        values.push(value);
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if depth > 0 {
            return Err("incomplete JSON object".into());
        }
    }
    Ok(values)
}

fn build_jsonl_tree(
    id: &str,
    nodes_by_id: &mut BTreeMap<String, PenNode>,
    children_by_parent: &mut BTreeMap<String, Vec<String>>,
) -> Option<PenNode> {
    let mut node = nodes_by_id.remove(id)?;
    if let Some(child_ids) = children_by_parent.remove(id) {
        if let Some(children) = node.children_mut() {
            for child_id in child_ids {
                if let Some(child) = build_jsonl_tree(&child_id, nodes_by_id, children_by_parent) {
                    children.push(child);
                }
            }
        }
    }
    Some(node)
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
    hovered_index: Option<usize>,
) -> (Vec<DesignBlock>, f32) {
    const BODY_TOP_GAP: f32 = 4.0;
    const BODY_PAD_Y: f32 = 8.0;
    const BODY_LINE_H: f32 = 13.0;
    const APPLY_H: f32 = 32.0;
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
        let body_content_h = if code_lines.is_empty() {
            0.0
        } else {
            BODY_PAD_Y * 2.0 + BODY_LINE_H * code_lines.len() as f32
        };
        let body_h = if body_content_h > 0.0 {
            BODY_TOP_GAP + body_content_h
        } else {
            0.0
        };
        let apply = if expanded && !pending.streaming && !pending.applied && body_content_h > 0.0 {
            Some(Rect::xywh(
                x,
                y + DESIGN_BLOCK_H + BODY_TOP_GAP + body_content_h,
                width,
                APPLY_H,
            ))
        } else {
            None
        };
        let apply_h = apply.map(|rect| rect.size.y).unwrap_or(0.0);
        let rect = Rect::xywh(x, y, width, DESIGN_BLOCK_H + body_h + apply_h);
        let header = Rect::xywh(x, y, width, DESIGN_BLOCK_H);
        let copy = Rect::xywh(x + width - 48.0, y + 6.0, 20.0, 20.0);
        let body = Rect::xywh(x, y + DESIGN_BLOCK_H + BODY_TOP_GAP, width, body_content_h);
        blocks.push(DesignBlock {
            rect,
            header,
            copy,
            body,
            apply,
            expanded,
            element_count: pending.element_count,
            label: pending.label,
            streaming: pending.streaming,
            code: pending.code,
            copy_visible: hovered_index == Some(index),
            code_lines,
        });
        y += DESIGN_BLOCK_H + body_h + apply_h + gap;
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
    let icon_bg_rect = Rect::xywh(
        block.rect.origin.x + DESIGN_ICON_LEFT,
        block.rect.origin.y + (DESIGN_BLOCK_H - DESIGN_ICON_BG) / 2.0,
        DESIGN_ICON_BG,
        DESIGN_ICON_BG,
    );
    cx.backend.fill_round_rect(icon_bg_rect, 8.0, icon_bg);
    draw_icon(
        cx.backend,
        Icon::Wand2,
        Point2D::new(
            icon_bg_rect.origin.x + (DESIGN_ICON_BG - DESIGN_ICON_SIZE) / 2.0,
            icon_bg_rect.origin.y + (DESIGN_ICON_BG - DESIGN_ICON_SIZE) / 2.0,
        ),
        DESIGN_ICON_SIZE,
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
    let label_x = icon_bg_rect.origin.x + DESIGN_ICON_BG + DESIGN_ICON_GAP;
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        label_x,
        block.header.origin.y,
        (block.header.origin.x + block.header.size.x - label_x - 56.0).max(1.0),
        block.header.size.y,
    ));
    let layout = TextLayout::single_run(
        &block.label,
        "system-ui",
        11.0,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(label_x, block.header.origin.y + 20.0));
    cx.backend.restore();

    if block.copy_visible {
        let mut copy_color = theme.muted_foreground;
        copy_color.a *= 0.5;
        draw_icon(
            cx.backend,
            Icon::Copy,
            Point2D::new(block.copy.origin.x + 4.0, block.copy.origin.y + 4.0),
            12.0,
            copy_color,
            1.5,
        );
    }

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
        let mut body_fill = theme.card;
        body_fill.a *= 0.5;
        let mut body_border = theme.border;
        body_border.a *= 0.3;
        cx.backend.fill_round_rect(block.body, 6.0, body_fill);
        cx.backend
            .stroke_round_rect(block.body, 6.0, body_border, 1.0);
        cx.backend.save();
        cx.backend.clip_rect(block.body);
        let mut baseline = block.body.origin.y + 18.0;
        for line in &block.code_lines {
            let layout = TextLayout::single_run(
                line,
                "monospace",
                9.0,
                (theme.muted_foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&layout, Point2D::new(block.body.origin.x + 10.0, baseline));
            baseline += 13.0;
        }
        cx.backend.restore();
    }

    if let Some(apply) = block.apply {
        let mut apply_fill = theme.muted;
        apply_fill.a *= 0.35;
        let mut apply_border = theme.border;
        apply_border.a *= 0.3;
        cx.backend.fill_round_rect(apply, 6.0, apply_fill);
        cx.backend.stroke_round_rect(apply, 6.0, apply_border, 1.0);
        let layout = TextLayout::single_run(
            "Apply to Canvas",
            "system-ui",
            10.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        let text_x = apply.origin.x + (apply.size.x - 84.0).max(0.0) / 2.0;
        cx.backend
            .draw_text(&layout, Point2D::new(text_x, apply.origin.y + 20.0));
    }
}
