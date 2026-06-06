//! Shared TS-shaped node defaults applied before schema deserialization.

use serde_json::{Map, Number, Value};

const DEFAULT_TEXT_FONT_SIZE: f64 = 13.0;
const TEXT_WIDTH_PADDING: f64 = 4.0;
const MAX_DEFAULT_TEXT_WIDTH: f64 = 4096.0;

pub(crate) fn normalize_text_default_bounds(obj: &mut Map<String, Value>) {
    let kind = obj
        .get("type")
        .or_else(|| obj.get("kind"))
        .and_then(Value::as_str);
    if kind != Some("text") {
        return;
    }

    let (w, h) = default_text_bounds(obj);
    obj.entry("width")
        .or_insert_with(|| Value::Number(Number::from(w)));
    obj.entry("height")
        .or_insert_with(|| Value::Number(Number::from(h)));
}

fn default_text_bounds(obj: &Map<String, Value>) -> (i64, i64) {
    let (min_w, min_h) = op_editor_core::default_leaf_node_size("text");
    let content = obj.get("content").and_then(Value::as_str).unwrap_or("");
    if content.is_empty() {
        return (i64::from(min_w), i64::from(min_h));
    }

    let font_size = obj
        .get("fontSize")
        .and_then(json_number)
        .filter(|v| *v > 0.0)
        .unwrap_or(DEFAULT_TEXT_FONT_SIZE);
    let line_height = (font_size * 1.2).ceil();
    let max_line_width = content
        .split('\n')
        .map(|line| estimate_line_width(line, font_size))
        .fold(0.0, f64::max);
    let line_count = content.split('\n').count().max(1) as f64;
    let width = (max_line_width + TEXT_WIDTH_PADDING)
        .ceil()
        .clamp(f64::from(min_w), MAX_DEFAULT_TEXT_WIDTH);
    let height = (line_height * line_count).ceil().max(f64::from(min_h));

    (width as i64, height as i64)
}

fn json_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn estimate_line_width(line: &str, font_size: f64) -> f64 {
    line.chars()
        .map(|ch| font_size * estimated_glyph_em(ch))
        .sum()
}

fn estimated_glyph_em(ch: char) -> f64 {
    if ch.is_ascii_whitespace() {
        0.32
    } else if matches!(
        ch,
        '-' | '_' | '.' | ',' | ':' | ';' | '/' | '\\' | '|' | '!' | 'i' | 'l'
    ) {
        0.35
    } else if ch.is_ascii() {
        0.58
    } else {
        1.0
    }
}
