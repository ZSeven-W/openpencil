//! The generation recorder's foreground check, after structural repair.
//! Keep authored ink until the label's actual parent is known; unresolved
//! tokens, photos and translucent paints remain for document-aware cleanup.

use op_design_lint::color::{color_contrast, parse_hex_color};
use serde_json::Value;

fn opaque_color(node: &Value) -> Option<&str> {
    if node.get("opacity").and_then(Value::as_f64).unwrap_or(1.0) != 1.0 {
        return None;
    }
    let fill = node.get("fill")?;
    let paint = match fill.as_array() {
        Some(paints) if paints.len() == 1 => &paints[0],
        Some(_) => return None,
        None => fill,
    };
    let color = if let Some(color) = paint.as_str() {
        color
    } else {
        if paint.get("type")?.as_str()?.to_lowercase() != "solid"
            || paint.get("visible").and_then(Value::as_bool) == Some(false)
            || paint.get("enabled").and_then(Value::as_bool) == Some(false)
            || paint.get("opacity").and_then(Value::as_f64).unwrap_or(1.0) != 1.0
        {
            return None;
        }
        paint.get("color")?.as_str()?
    };
    let color = color.trim();
    parse_hex_color(color)?;
    if color.len() == 9 && !color[7..].eq_ignore_ascii_case("ff") {
        return None;
    }
    Some(color)
}

fn replace_color(node: &mut Value, color: &str) {
    let fill = &mut node["fill"];
    if fill.is_string() {
        *fill = Value::String(color.into());
    } else if let Some(paints) = fill.as_array_mut() {
        paints[0]["color"] = Value::String(color.into());
    } else {
        fill["color"] = Value::String(color.into());
    }
}

/// Whose colours the forest carries, which decides how much contrast repair
/// may claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForegroundContrast {
    /// Model-generated sections: hold text to WCAG AA (4.5:1, icons 3:1).
    Generated,
    /// A document whose colours an author set explicitly (MCP edits, the
    /// agentic loop's finalize): repair only ink no author could have meant —
    /// near-invisible text or icons. A brand-blue link (~3.1:1) or white on a
    /// primary button is a design decision and is kept (issue #273).
    Authored,
}

/// Below this ratio foreground ink is effectively invisible on its surface.
pub const INVISIBLE_INK_RATIO: f64 = 1.5;

pub(super) fn repair(node: &mut Value, inherited: Option<&str>, mode: ForegroundContrast) {
    let threshold = match (node.get("type").and_then(Value::as_str), mode) {
        (Some("text"), ForegroundContrast::Generated) => Some(4.5),
        (Some("icon_font"), ForegroundContrast::Generated) => Some(3.0),
        (Some("text" | "icon_font"), ForegroundContrast::Authored) => Some(INVISIBLE_INK_RATIO),
        _ => None,
    };
    if let (Some(threshold), Some(bg), Some(fg)) = (threshold, inherited, opaque_color(node)) {
        if color_contrast(fg, bg) < threshold {
            let dark = "#17191D";
            let light = "#FAF8F3";
            replace_color(
                node,
                if color_contrast(dark, bg) >= color_contrast(light, bg) {
                    dark
                } else {
                    light
                },
            );
        }
    }
    let background = if threshold.is_none() && node.get("fill").is_some() {
        opaque_color(node)
    } else {
        inherited
    }
    .map(str::to_owned);
    // An independently readable front layer proves the offset copy is ink,
    // not another label needing contrast. This matches the direct recorder.
    let protected = op_editor_core::text_effect_overlay::pair(node).and_then(|pair| {
        if pair
            .front
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|runs| runs.iter().any(|run| run.get("fill").is_some()))
        {
            return None;
        }
        let bg = background.as_deref()?;
        let fg = opaque_color(pair.front)?;
        (color_contrast(fg, bg) >= 4.5).then_some(1 - pair.front_index)
    });
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for (index, child) in children.iter_mut().enumerate() {
            if protected != Some(index) {
                repair(child, background.as_deref(), mode);
            }
        }
    }
}
