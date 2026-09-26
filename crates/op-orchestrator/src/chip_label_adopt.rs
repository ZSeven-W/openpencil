//! Adopt a segmented control's labels into the EMPTY chips they were meant to
//! sit in.
//!
//! Models author a period switcher / segmented control as a flat row
//! `[chip, label, chip, label, …]`: each chip is a fixed-size, centring,
//! childless frame (`justifyContent`/`alignItems: center`, no children) and
//! its label is the NEXT flow sibling instead of the chip's child (measured:
//! `arena-m03` `range-switcher`, three 44×28 chips followed by `1周 / 1月 /
//! 3月` — rendered as a blank "selected" pill with its label beside it). A
//! centring frame with nothing to centre, immediately followed by a text that
//! fits inside it, is that intent by construction: move the text in.
//!
//! **Tier: Contract** (see [`crate::repair_tier`]). A screenshot of the
//! unrepaired row shows an empty pill next to its own label — no author means
//! that. Like the sibling `chip_repair::adopt_*` passes it runs ungated in the
//! cleanup driver. Only the text moves; the chip keeps its fill / stroke, so a
//! selected chip stays highlighted.
//!
//! What it refuses, and why:
//! - **A lone pair.** A single empty frame next to a text is a colour swatch
//!   beside its name as often as it is a chip; only a row with ≥2 pairs is a
//!   control.
//! - **A text that does not fit.** The label's measured natural width (real
//!   jian layout) and line height must fit the chip's inner box. A fixed-width
//!   chip is NOT widened: a label wider than its chip is not provably the
//!   chip's label (a legend swatch is smaller than any name).
//! - **Anything positioned.** A chip or text with authored `x`/`y` is absolute
//!   in the engine — placed on purpose, not a flow slip.
//! - **Status bars and image-filled frames.**

use std::collections::HashMap;

use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use serde_json::Value;

use crate::types::DocSink;

/// Largest chip this pass treats as a segmented-control cell.
const MAX_CHIP_W: f64 = 160.0;
const MAX_CHIP_H: f64 = 64.0;
/// Rounding slack when comparing a measured text box against the chip's.
const FIT_EPS: f64 = 0.5;
const MEASURE_PREFIX: &str = "__chip_label_m_";

/// One `(chip, label)` candidate: the ids plus what the fit check needs.
struct Pair {
    chip_id: String,
    text_id: String,
    /// Chip inner box; `None` width = `fit_content` (always fits).
    inner_w: Option<f64>,
    inner_h: f64,
    /// Authored fixed text width, when the text has one.
    fixed_text_w: Option<f64>,
}

/// Move each label authored beside its empty chip into that chip. Returns the
/// number of labels the sink accepted.
pub(crate) fn adopt_empty_chip_labels(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let rows = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return 0;
        };
        let Ok(v) = serde_json::to_value(root) else {
            return 0;
        };
        let mut rows = Vec::new();
        collect_rows(&v, &mut rows);
        rows
    };
    if rows.is_empty() {
        return 0;
    }
    let texts: Vec<&Value> = rows.iter().flat_map(|(_, texts)| texts.iter()).collect();
    let measured = measure_texts(sink.state(), &texts);
    let mut moved = 0;
    for (pairs, _) in &rows {
        let fitting: Vec<&Pair> = pairs.iter().filter(|p| fits(p, &measured)).collect();
        if fitting.len() < 2 {
            continue;
        }
        for pair in fitting {
            if sink.apply(EditorCommand::MoveNode {
                node_id: NodeId::new(pair.text_id.clone()),
                target_parent: NodeId::new(pair.chip_id.clone()),
                page_id: None,
                index: Some(0),
            }) {
                moved += 1;
            }
        }
    }
    moved
}

/// Every horizontal row (outside status bars) holding ≥2 structural pairs,
/// with the pair texts' JSON for measuring.
fn collect_rows(v: &Value, out: &mut Vec<(Vec<Pair>, Vec<Value>)>) {
    if crate::cleanup::is_status_bar_from_json(v) {
        return;
    }
    let kids = children(v);
    if v.get("layout").and_then(Value::as_str) == Some("horizontal") {
        let mut pairs = Vec::new();
        let mut texts = Vec::new();
        let mut i = 0;
        while i + 1 < kids.len() {
            match pair_at(&kids[i], &kids[i + 1]) {
                Some(pair) => {
                    pairs.push(pair);
                    texts.push(kids[i + 1].clone());
                    i += 2;
                }
                None => i += 1,
            }
        }
        if pairs.len() >= 2 {
            out.push((pairs, texts));
        }
    }
    for kid in kids {
        collect_rows(kid, out);
    }
}

fn pair_at(chip: &Value, text: &Value) -> Option<Pair> {
    let (inner_w, inner_h) = empty_chip_inner(chip)?;
    if !is_flow_label(text) {
        return None;
    }
    Some(Pair {
        chip_id: str_field(chip, "id")?.to_string(),
        text_id: str_field(text, "id")?.to_string(),
        inner_w,
        inner_h,
        fixed_text_w: numeric(text, "width"),
    })
}

/// Inner box of a childless, unpositioned, centring, small, image-free frame.
fn empty_chip_inner(v: &Value) -> Option<(Option<f64>, f64)> {
    if str_field(v, "type") != Some("frame") || is_positioned(v) || !children(v).is_empty() {
        return None;
    }
    let centring = ["justifyContent", "alignItems"]
        .iter()
        .any(|key| str_field(v, key) == Some("center"));
    if !centring || has_image_fill(v) {
        return None;
    }
    let h = numeric(v, "height").filter(|h| *h > 0.0 && *h <= MAX_CHIP_H)?;
    let width = match v.get("width") {
        Some(Value::String(s)) if s == "fit_content" => None,
        _ => Some(numeric(v, "width").filter(|w| *w > 0.0 && *w <= MAX_CHIP_W)?),
    };
    let [top, right, bottom, left] = padding(v)?;
    Some((width.map(|w| w - left - right), h - top - bottom))
}

fn is_flow_label(v: &Value) -> bool {
    str_field(v, "type") == Some("text")
        && !is_positioned(v)
        && str_field(v, "content").is_some_and(|c| !c.trim().is_empty())
}

fn fits(pair: &Pair, measured: &HashMap<String, (f64, f64)>) -> bool {
    let Some(&(natural_w, h)) = measured.get(&pair.text_id) else {
        return false;
    };
    let w = pair.fixed_text_w.unwrap_or(natural_w);
    let fits_w = pair.inner_w.is_none_or(|inner| w <= inner + FIT_EPS);
    fits_w && h <= pair.inner_h + FIT_EPS
}

/// Lay hugging copies of `texts` out in an unconstrained scratch document
/// (same variables / themes) and read their natural single-line boxes back
/// from the real jian layout — the fit check is the platform's own text
/// metrics, not an estimate.
fn measure_texts(state: &EditorState, texts: &[&Value]) -> HashMap<String, (f64, f64)> {
    let kids: Vec<Value> = texts
        .iter()
        .filter_map(|t| {
            let id = str_field(t, "id")?;
            let mut copy = (*t).clone();
            let obj = copy.as_object_mut()?;
            for key in ["x", "y", "height", "animations", "children"] {
                obj.remove(key);
            }
            obj.insert("id".into(), Value::String(format!("{MEASURE_PREFIX}{id}")));
            obj.insert("width".into(), Value::String("fit_content".into()));
            obj.insert("textGrowth".into(), Value::String("auto".into()));
            Some(copy)
        })
        .collect();
    let doc = serde_json::json!({
        "version": state.doc.version,
        "variables": state.doc.variables,
        "themes": state.doc.themes,
        "children": [{
            "type": "frame", "id": "__chip_label_root", "width": 100000,
            "height": "fit_content", "layout": "vertical", "alignItems": "start",
            "children": kids,
        }],
    });
    let mut measured = HashMap::new();
    let Ok(doc) = serde_json::from_value::<jian_ops_schema::PenDocument>(doc) else {
        return measured;
    };
    let scene =
        op_pen_loader::pen_document_to_layout_scene(&doc, &std::collections::BTreeMap::new(), 0);
    if let Some(page) = scene.active_page() {
        collect_boxes(&page.children, &mut measured);
    }
    measured
}

fn collect_boxes(nodes: &[SceneNode], out: &mut HashMap<String, (f64, f64)>) {
    for node in nodes {
        if let Some(orig) = node.id.strip_prefix(MEASURE_PREFIX) {
            let b = node.aggregate_bounds();
            let (w, h) = (f64::from(b.size.x), f64::from(b.size.y));
            if w.is_finite() && w > 0.0 && h.is_finite() && h > 0.0 {
                out.insert(orig.to_string(), (w, h));
            }
        }
        collect_boxes(&node.children, out);
    }
}

/// `[top, right, bottom, left]`; `None` for an expression padding the pass
/// cannot evaluate.
fn padding(v: &Value) -> Option<[f64; 4]> {
    let num = |x: &Value| x.as_f64();
    match v.get("padding") {
        None | Some(Value::Null) => Some([0.0; 4]),
        Some(Value::Number(n)) => n.as_f64().map(|p| [p; 4]),
        Some(Value::Array(a)) if a.len() == 2 => {
            let (y, x) = (num(&a[0])?, num(&a[1])?);
            Some([y, x, y, x])
        }
        Some(Value::Array(a)) if a.len() == 4 => {
            Some([num(&a[0])?, num(&a[1])?, num(&a[2])?, num(&a[3])?])
        }
        _ => None,
    }
}

fn has_image_fill(v: &Value) -> bool {
    let is_image = |f: &Value| str_field(f, "type") == Some("image");
    match v.get("fill") {
        Some(Value::Array(fills)) => fills.iter().any(is_image),
        Some(fill @ Value::Object(_)) => is_image(fill),
        _ => false,
    }
}

fn is_positioned(v: &Value) -> bool {
    ["x", "y"]
        .iter()
        .any(|key| v.get(*key).is_some_and(|x| !x.is_null()))
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
#[path = "chip_label_adopt_tests.rs"]
mod tests;
