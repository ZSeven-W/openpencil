//! Reject inert, visibly authored quantity steppers before committing a section.
//! This is a read-only gate, not an inference of prices, limits, or business rules.

use jian_core::expression::Expression;
use jian_ops_schema::node::PenNode;
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn rejection(prompt: &str, nodes: &[PenNode]) -> Option<String> {
    let prompt = prompt.to_lowercase();
    if [
        "static only",
        "static-only",
        "mockup only",
        "仅静态",
        "只做静态",
        "无需交互",
        "不需要交互",
    ]
    .iter()
    .any(|word| prompt.contains(word))
        || ![
            "interactive",
            "ontap",
            "可交互",
            "可操作",
            "可点击",
            "数量加减",
        ]
        .iter()
        .any(|word| prompt.contains(word))
    {
        return None;
    }
    let nodes = serde_json::to_value(nodes).ok()?;
    nodes.as_array()?.iter().find_map(inspect)
}

fn children(value: &Value) -> &[Value] {
    value["children"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn single_text(value: &Value) -> Option<&str> {
    if value["type"] == "text" {
        return value["content"].as_str();
    }
    let mut texts = children(value).iter().filter_map(single_text);
    let first = texts.next()?;
    texts.next().is_none().then_some(first)
}

fn sign(value: &Value, positive: bool) -> bool {
    if let Some(text) = single_text(value) {
        return if positive {
            matches!(text.trim(), "+" | "＋")
        } else {
            matches!(text.trim(), "-" | "−" | "－" | "–")
        };
    }
    let icon = value["iconFontName"]
        .as_str()
        .or_else(|| value["iconId"].as_str());
    icon.is_some_and(|icon| {
        icon.rsplit(':').next() == Some(if positive { "plus" } else { "minus" })
    }) || (children(value).len() == 1 && sign(&children(value)[0], positive))
}

fn reads(value: &Value, out: &mut BTreeSet<String>) {
    if let Some(source) = value["bindings"]["content"].as_str() {
        if let Ok(expr) = Expression::compile(source) {
            out.extend(expr.chunk.scope_paths.into_iter().map(canonical_path));
        }
    }
    for child in children(value) {
        reads(child, out);
    }
}

fn canonical_path(path: String) -> String {
    path.strip_prefix("$state.")
        .map(|key| format!("$app.{key}"))
        .unwrap_or(path)
}

fn writes(value: &Value, out: &mut BTreeSet<String>) {
    if let Some(set) = value.get("set").and_then(Value::as_object) {
        if let Some(path) = set.get("target").and_then(Value::as_str) {
            out.insert(canonical_path(path.into()));
        } else {
            out.extend(
                set.keys()
                    .filter(|key| key.starts_with('$'))
                    .cloned()
                    .map(canonical_path),
            );
        }
    }
    match value {
        Value::Object(map) => {
            for value in map.values() {
                writes(value, out);
            }
        }
        Value::Array(values) => {
            for value in values {
                writes(value, out);
            }
        }
        _ => {}
    }
}

fn tap_writes(value: &Value) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    writes(&value["events"]["onTap"], &mut result);
    for child in children(value) {
        result.extend(tap_writes(child));
    }
    result
}

fn inspect(value: &Value) -> Option<String> {
    if value["visible"] == false {
        return None;
    }
    let kids = children(value);
    if value["layout"] == "horizontal" {
        // Thin inert separators are visual decoration, not a fourth control.
        let controls: Vec<_> = kids
            .iter()
            .filter(|child| {
                !(child["type"] == "rectangle"
                    && child["width"].as_f64().is_some_and(|w| w <= 2.0)
                    && children(child).is_empty()
                    && child.get("events").is_none())
            })
            .collect();
        for trio in controls.windows(3) {
            if !sign(trio[0], false)
                || !sign(trio[2], true)
                || single_text(trio[1]).is_none_or(|text| text.trim().parse::<u32>().is_err())
            {
                continue;
            }
            let mut paths = BTreeSet::new();
            reads(trio[1], &mut paths);
            let minus = tap_writes(trio[0]);
            let plus = tap_writes(trio[2]);
            if paths.is_empty() || paths.is_disjoint(&minus) || paths.is_disjoint(&plus) {
                return Some(format!(
                    "self-check failed: interaction-inert-stepper: control {:?} shows minus / quantity / plus, \
                     but both buttons must write the SAME state read by the quantity label. \
                     Keep this layout and copy; declare its initial quantity, bind its label, and implement \
                     the actual onTap decrement/increment with the requested limits. Do not infer new \
                     prices or limits. A toast or an unbound number is not a working quantity control.",
                    value["id"].as_str().unwrap_or("unknown")
                ));
            }
        }
    }
    kids.iter().find_map(inspect)
}

#[cfg(test)]
#[path = "generated_stepper_check_tests.rs"]
mod tests;
