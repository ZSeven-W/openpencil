use serde_json::{json, Value};

/// Read a width/height as a pixel number (port of TS `toSizeNumber`).
pub(crate) fn size_number(node: &Value, key: &str) -> f64 {
    match node.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn gap_number(node: &Value) -> f64 {
    match node.get("gap") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn padding_lr(node: &Value) -> (f64, f64) {
    match node.get("padding") {
        Some(Value::Number(n)) => {
            let v = n.as_f64().unwrap_or(0.0);
            (v, v)
        }
        Some(Value::String(s)) => {
            let v = s.parse::<f64>().unwrap_or(0.0);
            (v, v)
        }
        Some(Value::Array(a)) if a.len() == 2 => (
            a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
            a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
        ),
        Some(Value::Array(a)) if a.len() >= 4 => (
            a.get(3).and_then(Value::as_f64).unwrap_or(0.0),
            a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
        ),
        _ => (0.0, 0.0),
    }
}

pub(crate) fn fix_horizontal_overflow(node: &mut Value, canvas_width: f64) {
    let parent_w = size_number(node, "width");
    if parent_w <= 0.0 {
        return;
    }
    let (pad_l, pad_r) = padding_lr(node);
    let avail_w = parent_w - pad_l - pad_r;
    let gap = gap_number(node);
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if children.len() < 2 {
        return;
    }
    let gap_total = gap * (children.len().saturating_sub(1) as f64);
    let mut total_w = gap_total
        + children
            .iter()
            .map(|c| {
                let w = size_number(c, "width");
                if c.get("width").and_then(Value::as_f64).is_some() && w > 0.0 {
                    w
                } else {
                    80.0
                }
            })
            .sum::<f64>();
    if total_w <= avail_w {
        return;
    }
    for try_gap in [8.0, 4.0] {
        if gap > try_gap {
            let reduced = total_w - gap_total + try_gap * (children.len() - 1) as f64;
            if reduced <= avail_w {
                node["gap"] = json!(try_gap);
                total_w = reduced;
                break;
            }
        }
    }
    if total_w > avail_w {
        let needed_w = (total_w + pad_l + pad_r).round();
        if needed_w > parent_w && needed_w <= canvas_width {
            node["width"] = json!(needed_w);
        } else if needed_w > canvas_width * 0.8 {
            // Content exceeds the viewport — widening can't make the children fit
            // (their sum already overflows the canvas). Span the viewport and clip
            // the overflow at the row edge so it reads as a scroll row cut at the
            // screen, instead of chips spilling off-canvas into the void.
            // overflow.md mandates a `clipContent` wrapper for scroll rows; weak
            // models (e.g. glm-5.2) routinely emit a bare horizontal frame without
            // it, so this is the deterministic floor that keeps off-screen children
            // from rendering outside the device frame.
            node["width"] = json!("fill_container");
            node["clipContent"] = json!(true);
        }
    }
}

pub(crate) fn fix_text_heights(node: &mut Value) {
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if child.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        let explicit_height = child.get("height").and_then(Value::as_f64).is_some();
        let fixed_height =
            child.get("textGrowth").and_then(Value::as_str) == Some("fixed-width-height");
        if explicit_height && !fixed_height {
            if let Some(obj) = child.as_object_mut() {
                obj.remove("height");
            }
        }
    }
}
