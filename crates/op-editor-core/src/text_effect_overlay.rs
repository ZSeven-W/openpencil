//! One anchored reading layer plus a small offset copy is a text effect.
//! Keep this proof shared by generation, geometry and contrast consumers.

use crate::PenNodeExt;
use jian_ops_schema::node::PenNode;
use serde_json::Value;
use std::collections::BTreeSet;

pub struct OverlayPair<'a> {
    pub front: &'a Value,
    pub back: &'a Value,
    pub front_index: usize,
}

pub fn pair(parent: &Value) -> Option<OverlayPair<'_>> {
    if parent.get("type")?.as_str()? != "frame"
        || parent.get("layout")?.as_str()? != "none"
        || parent.get("clipContent").and_then(Value::as_bool) == Some(true)
        || !opaque_visible(parent)
        || parent.get("events").is_some()
        || parent.get("fill").is_some_and(|v| !empty(v))
        || parent.get("effects").is_some_and(|v| !empty(v))
    {
        return None;
    }
    let kids = parent.get("children")?.as_array()?;
    if kids.len() != 2
        || kids.iter().any(|v| {
            v.get("type").and_then(Value::as_str) != Some("text")
                || !opaque_visible(v)
                || v.get("events").is_some()
                || v.get("constraints").is_some()
                || v.get("rotation").and_then(Value::as_f64).unwrap_or(0.0) != 0.0
        })
    {
        return None;
    }
    let copy = content(&kids[0])?;
    if copy != content(&kids[1])?
        || copy.chars().count() < 2
        || !copy.chars().any(char::is_alphabetic)
        || text_signature(&kids[0]) != text_signature(&kids[1])
    {
        return None;
    }
    let size = kids[0].get("fontSize")?.as_f64()?;
    if !size.is_finite()
        || size < 24.0
        || ![
            "fontFamily",
            "fontSize",
            "fontWeight",
            "fontStyle",
            "letterSpacing",
            "lineHeight",
            "textAlign",
            "textAlignVertical",
            "textGrowth",
            "width",
            "height",
            "rotation",
        ]
        .iter()
        .all(|key| kids[0].get(key) == kids[1].get(key))
    {
        return None;
    }
    let xy = |v: &Value| Some((v.get("x")?.as_f64()?, v.get("y")?.as_f64()?));
    let a = xy(&kids[0])?;
    let b = xy(&kids[1])?;
    let front_index = if a == (0.0, 0.0) {
        0
    } else if b == (0.0, 0.0) {
        1
    } else {
        return None;
    };
    let (x, y) = if front_index == 0 { b } else { a };
    let offset = x.abs().max(y.abs());
    if !offset.is_finite() || offset < 0.5 || offset > (size * 0.12).min(8.0) {
        return None;
    }
    Some(OverlayPair {
        front: &kids[front_index],
        back: &kids[1 - front_index],
        front_index,
    })
}

fn empty(v: &Value) -> bool {
    v.is_null() || v.as_array().is_some_and(Vec::is_empty)
}

fn opaque_visible(v: &Value) -> bool {
    v.get("visible").and_then(Value::as_bool) != Some(false)
        && v.get("enabled")
            .is_none_or(|e| e.is_null() || e.as_bool() == Some(true))
        && v.get("opacity")
            .is_none_or(|o| o.is_null() || o.as_f64() == Some(1.0))
}

pub fn content(v: &Value) -> Option<String> {
    let value = v.get("content")?;
    if let Some(copy) = value.as_str() {
        return Some(copy.to_string());
    }
    value
        .as_array()?
        .iter()
        .map(|run| run.get("text")?.as_str())
        .collect::<Option<Vec<_>>>()
        .map(|runs| runs.concat())
}

fn text_signature(v: &Value) -> Value {
    let mut content = v.get("content").cloned().unwrap_or_default();
    if let Some(runs) = content.as_array_mut() {
        for run in runs {
            if let Some(run) = run.as_object_mut() {
                run.remove("fill");
            }
        }
    }
    content
}

pub fn effect_pairs(root: &Value) -> BTreeSet<(String, String)> {
    let mut result = BTreeSet::new();
    fn walk(v: &Value, result: &mut BTreeSet<(String, String)>) {
        if let Some(pair) = pair(v) {
            if let (Some(a), Some(b)) = (
                pair.front.get("id").and_then(Value::as_str),
                pair.back.get("id").and_then(Value::as_str),
            ) {
                result.insert(if a < b {
                    (a.into(), b.into())
                } else {
                    (b.into(), a.into())
                });
            }
        }
        if let Some(kids) = v.get("children").and_then(Value::as_array) {
            for kid in kids {
                walk(kid, result);
            }
        }
    }
    walk(root, &mut result);
    result
}

/// The canonical child order paints earlier siblings on top. Normalize only
/// freshly generated matching copies; imported/user-authored trees stay intact.
pub fn normalize_generated_order(nodes: &mut [PenNode]) {
    for node in nodes {
        if node.children().is_some_and(|kids| {
            kids.len() == 2 && kids.iter().all(|n| matches!(n, PenNode::Text(_)))
        }) && serde_json::to_value(&*node)
            .ok()
            .and_then(|v| pair(&v).map(|p| p.front_index))
            == Some(1)
        {
            if let Some(kids) = node.children_mut() {
                kids.swap(0, 1);
            }
        }
        if let Some(kids) = node.children_mut() {
            normalize_generated_order(kids);
        }
    }
}
