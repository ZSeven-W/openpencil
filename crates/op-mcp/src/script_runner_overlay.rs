//! Restore recorded decoration ink only after both fresh layers are known.
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) struct Insert {
    binding: String,
    parent: String,
    node: Option<Value>,
    original_fill: Option<Value>,
    background: String,
    line: usize,
}

pub(super) fn capture(
    binding: String,
    parent: String,
    node: &str,
    original_fill: &str,
    background: String,
    line: usize,
) -> Insert {
    // Metadata is subordinate to the existing recorded output, never a second
    // unbounded transport. Large/unprovable nodes simply keep the old repair.
    Insert {
        binding,
        parent,
        node: if node.len() <= 16_384 {
            serde_json::from_str(node).ok()
        } else {
            None
        },
        original_fill: if original_fill.len() <= 512 {
            serde_json::from_str(original_fill).ok()
        } else {
            None
        },
        background: if background.len() <= 16 {
            background
        } else {
            String::new()
        },
        line,
    }
}

pub(super) fn restore(lines: &mut Vec<String>, inserts: &[Insert]) {
    let mut groups: BTreeMap<&str, Vec<&Insert>> = BTreeMap::new();
    for insert in inserts {
        groups.entry(&insert.parent).or_default().push(insert);
    }
    let parents: BTreeMap<&str, &Insert> =
        inserts.iter().map(|i| (i.binding.as_str(), i)).collect();
    let mut candidate = lines.clone();
    for (parent, kids) in groups {
        if kids.len() != 2 {
            continue;
        }
        let Some(parent_node) = parents.get(parent).and_then(|i| i.node.as_ref()) else {
            continue;
        };
        // Inline children, descendants or edits are not this two-I() proof.
        if parent_node
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|c| !c.is_empty())
        {
            continue;
        }
        let (Some(a), Some(b)) = (kids[0].node.as_ref(), kids[1].node.as_ref()) else {
            continue;
        };
        let mut wrapper = parent_node.clone();
        wrapper["children"] = json!([a, b]);
        let Some(pair) = op_editor_core::text_effect_overlay::pair(&wrapper) else {
            continue;
        };
        let front = kids[pair.front_index];
        let back = kids[1 - pair.front_index];
        let Some(color) = opaque_color(pair.front.get("fill")) else {
            continue;
        };
        if let Some(runs) = pair.front.get("content").and_then(Value::as_array) {
            if runs.iter().any(|r| r.get("fill").is_some()) {
                continue;
            }
        }
        if opaque_hex(&back.background).is_none()
            || op_design_lint::color::color_contrast(color, &back.background) < 4.5
        {
            continue;
        }
        let Some(original) = back
            .original_fill
            .as_ref()
            .filter(|f| opaque_color(Some(f)).is_some())
        else {
            continue;
        };
        let mut restored = pair.back.clone();
        restored["fill"] = original.clone();
        candidate[back.line] = format!("{}=I({}, {})", back.binding, back.parent, restored);
        // Leaf inserts share an existing parent and have no dependent children;
        // stable bindings allow the anchored reading layer to paint on top.
        if pair.front_index == 1 {
            candidate.swap(front.line, back.line);
        }
    }
    if candidate.iter().map(String::len).sum::<usize>() <= super::MAX_RECORDED_BYTES {
        *lines = candidate;
    }
}

fn opaque_hex(color: &str) -> Option<&str> {
    let s = color.trim();
    let h = s.strip_prefix('#')?;
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (h.len() == 3 || h.len() == 6 || (h.len() == 8 && h[6..].eq_ignore_ascii_case("ff")))
        .then_some(s)
}

fn opaque_color(fill: Option<&Value>) -> Option<&str> {
    let fill = fill?;
    if let Some(color) = fill.as_str() {
        return opaque_hex(color);
    }
    let paint = if let Some(fills) = fill.as_array() {
        if fills.len() != 1 {
            return None;
        }
        &fills[0]
    } else {
        fill
    };
    if paint.get("type")?.as_str()?.to_lowercase() != "solid"
        || paint.get("visible").and_then(Value::as_bool) == Some(false)
        || paint.get("enabled").and_then(Value::as_bool) == Some(false)
        || !paint
            .get("opacity")
            .is_none_or(|o| o.is_null() || o.as_f64() == Some(1.0))
    {
        return None;
    }
    opaque_hex(paint.get("color")?.as_str()?)
}
