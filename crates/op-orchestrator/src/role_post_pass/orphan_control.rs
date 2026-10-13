//! Recover an empty button followed by its clearly named, misparented parts
//! before contrast repair or empty-stub cleanup can destroy the control.

use super::*;

fn part_stem(node: &Value) -> Option<String> {
    let name = node.get("name")?.as_str()?.to_lowercase();
    let mut words: Vec<_> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let original_len = words.len();
    while words.last().is_some_and(|word| {
        matches!(
            *word,
            "button" | "btn" | "icon" | "label" | "text" | "glyph"
        )
    }) {
        words.pop();
    }
    (!words.is_empty() && words.len() < original_len).then(|| words.join(" "))
}

fn positioned(node: &Value) -> bool {
    node.get("x").is_some() || node.get("y").is_some()
}

fn compact_button(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("frame")
        && matches!(role_of(node), Some("button" | "icon-button"))
        && !positioned(node)
        && node["width"]
            .as_f64()
            .is_some_and(|w| w > 0.0 && w <= 320.0)
        && node["height"]
            .as_f64()
            .is_some_and(|h| h > 0.0 && h <= 64.0)
}

fn generic_button_label(node: &Value, button: &Value) -> bool {
    if node["type"] != "text" {
        return false;
    }
    let name = node["name"].as_str().unwrap_or_default().to_lowercase();
    let words: Vec<_> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    if !matches!(words.as_slice(), ["button" | "btn", "label" | "text"]) {
        return false;
    }
    let Some(content) = node["content"].as_str() else {
        return false;
    };
    if content.is_empty() || content.contains(['\n', '\r']) {
        return false;
    }
    let padding = &button["padding"];
    let (left, right) = match padding {
        Value::Number(value) => (value.as_f64().unwrap_or(0.0), value.as_f64().unwrap_or(0.0)),
        Value::Array(values) if values.len() == 2 => (
            values[1].as_f64().unwrap_or(0.0),
            values[1].as_f64().unwrap_or(0.0),
        ),
        Value::Array(values) if values.len() == 4 => (
            values[3].as_f64().unwrap_or(0.0),
            values[1].as_f64().unwrap_or(0.0),
        ),
        _ => (0.0, 0.0),
    };
    let available = size_number(button, "width") - left - right;
    // One em per character is deliberately conservative: this fallback has no
    // shared name stem and must not turn a long adjacent note into a button.
    let upper_width = content.chars().count() as f64
        * (node["fontSize"].as_f64().unwrap_or(14.0) + size_number(node, "letterSpacing").max(0.0));
    upper_width <= available && size_number(node, "width") <= available
}

fn duplicate_shell(empty: &Value, populated: &Value) -> bool {
    if !compact_button(empty) || !compact_button(populated) {
        return false;
    }
    // Do not remove an intentional interactive, hidden, or dynamically painted
    // control. The matching populated control must carry the actual content.
    if [empty, populated].iter().any(|node| {
        [
            "events",
            "bindings",
            "variants",
            "animations",
            "lifecycle",
            "route",
            "gestures",
            "state",
            "constraints",
        ]
        .iter()
        .any(|key| node.get(key).is_some())
            || node.get("visible").and_then(Value::as_bool) == Some(false)
    }) || !empty["children"].as_array().is_none_or(Vec::is_empty)
        || populated["children"].as_array().is_none_or(Vec::is_empty)
        || empty["fill"]
            .as_array()
            .is_some_and(|fills| fills.iter().any(|fill| fill["type"] == "image"))
    {
        return false;
    }
    let Some(stem) = part_stem(empty) else {
        return false;
    };
    let inner_name = populated["name"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    let inner_words: Vec<_> = inner_name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let Some((suffix, prefix)) = inner_words.split_last() else {
        return false;
    };
    if !matches!(*suffix, "inner" | "content") || prefix.join(" ") != stem {
        return false;
    }
    // A misparented inner wrapper repeats the exact shell. Different paints,
    // sizes or layout are authored siblings, even when their names are alike.
    let is_property = |key: &str| !matches!(key, "id" | "name" | "children");
    empty
        .as_object()
        .zip(populated.as_object())
        .is_some_and(|(left, right)| {
            left.iter()
                .filter(|(key, _)| is_property(key))
                .all(|(key, value)| right.get(key) == Some(value))
                && right
                    .keys()
                    .filter(|key| is_property(key))
                    .all(|key| left.contains_key(key))
        })
}

/// Recover structure throughout the tree before any ancestor's recursive
/// contrast repair can recolor a misparented white button label as body text.
pub(super) fn repair_orphan_controls(node: &mut Value) {
    let mut references = Vec::new();
    collect_references(node, &mut references);
    repair_with_references(node, &references);
}

fn collect_references(node: &Value, out: &mut Vec<String>) {
    for key in [
        "events",
        "bindings",
        "animations",
        "lifecycle",
        "ref",
        "descendants",
        "route",
        "gestures",
    ] {
        if let Some(value) = node.get(key) {
            out.push(value.to_string());
        }
    }
    if let Some(children) = node["children"].as_array() {
        for child in children {
            collect_references(child, out);
        }
    }
}

fn repair_with_references(node: &mut Value, references: &[String]) {
    adopt_orphan_button_parts(node, references);
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            repair_with_references(child, references);
        }
    }
}

pub(super) fn adopt_orphan_button_parts(node: &mut Value, references: &[String]) {
    if node.get("layout").and_then(Value::as_str) != Some("horizontal") {
        return;
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    let mut index = 0;
    while index < children.len() {
        if children.get(index + 1).is_some_and(|next| {
            duplicate_shell(&children[index], next)
                && next["id"]
                    .as_str()
                    .is_some_and(|id| !references.iter().any(|value| value.contains(id)))
        }) {
            let mut duplicate = children.remove(index + 1);
            // Keep the original button identity. References to the duplicate
            // inner wrapper prevent this repair rather than becoming dangling.
            children[index]["children"] = duplicate["children"].take();
        }
        let button = &children[index];
        let eligible = button.get("type").and_then(Value::as_str) == Some("frame")
            && matches!(role_of(button), Some("button" | "icon-button"))
            && !positioned(button)
            && button
                .get("children")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty);
        if let Some(stem) = eligible.then(|| part_stem(button)).flatten() {
            let accept_generic_label = compact_button(button);
            let count = children[index + 1..]
                .iter()
                .take_while(|part| {
                    matches!(
                        part.get("type").and_then(Value::as_str),
                        Some("text" | "icon_font" | "path")
                    ) && !positioned(part)
                        && part.get("events").is_none()
                        && (part_stem(part).as_deref() == Some(stem.as_str())
                            || (accept_generic_label && generic_button_label(part, button)))
                })
                .count();
            // A compact control has at most an icon, label and trailing icon.
            // Larger runs are ambiguous; preserve them for validation.
            if (1..=3).contains(&count) {
                let parts: Vec<_> = children.drain(index + 1..index + 1 + count).collect();
                children[index]["children"] = Value::Array(parts);
            }
        }
        index += 1;
    }
}
