//! Repair lightly wrapped CJK headings with a one/two-character final line.
//! Explicit breaks and narrow editorial compositions keep their authored size.

use super::*;

const EPS: f64 = 1.0;
const MAX_SHRINK: f64 = 0.12;
const MAX_CANDIDATES: usize = 16;

fn measurement_copy(state: &EditorState) -> EditorState {
    let mut copy = EditorState::from_document(state.doc.clone());
    copy.ui.active_page_index = state.ui.active_page_index;
    copy.ui.variables.active_theme = state.ui.variables.active_theme.clone();
    copy
}

struct Candidate {
    id: String,
    content: String,
    size: f64,
    available: f64,
}

fn cjk(ch: char) -> bool {
    matches!(ch as u32, 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff)
}

fn collect(
    v: &Value,
    rects: &HashMap<String, Rect>,
    parent: Option<&Value>,
    out: &mut Vec<Candidate>,
) {
    if out.len() >= MAX_CANDIDATES
        || v.get("visible") == Some(&Value::Bool(false))
        || num(v, "rotation") != 0.0
    {
        return;
    }
    if v.get("type").and_then(Value::as_str) == Some("text") {
        let name = v
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let role = v.get("role").and_then(Value::as_str).unwrap_or("");
        let heading = role == "heading"
            || ["headline", "title", "主标题", "標題", "标题"]
                .iter()
                .any(|s| name.contains(s));
        let body = matches!(role, "body-text" | "caption" | "label")
            || ["subtitle", "caption", "body", "副标题", "副標題"]
                .iter()
                .any(|s| name.contains(s));
        let content = v.get("content").and_then(Value::as_str).unwrap_or("");
        let count = content.chars().count();
        let size = num(v, "fontSize");
        let growth = v.get("textGrowth").and_then(Value::as_str);
        if heading
            && !body
            && (4..=24).contains(&count)
            && size.is_finite()
            && size >= 36.0
            && !content.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
            && content.chars().filter(|ch| cjk(*ch)).count() * 2 >= count
            && content.chars().last().is_some_and(cjk)
            && num(v, "rotation") == 0.0
            && matches!(growth, Some("fixed-width" | "fixed-width-height"))
            && v.get("height").and_then(Value::as_str) == Some("fit_content")
        {
            if let Some((id, rect)) = v
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id).map(|r| (id, r)))
            {
                let right = parent.and_then(|p| {
                    p.get("id").and_then(Value::as_str).and_then(|id| {
                        rects
                            .get(id)
                            .map(|r| r.x + r.w - numeric_padding_sides(p).map_or(0.0, |s| s[1]))
                    })
                });
                let available = right.map_or(rect.w, |right| rect.w.min(right - rect.x));
                if available.is_finite() && available > size {
                    out.push(Candidate {
                        id: id.into(),
                        content: content.into(),
                        size,
                        available,
                    });
                }
            }
        }
    }
    for child in children(v) {
        collect(child, rects, Some(v), out);
    }
}

pub(crate) fn repair_headline_tails(sink: &mut dyn DocSink, root_id: &str) -> usize {
    if sink.state().editor_ui.preserve_authored_geometry
        || sink.state().editor_ui.home.imported_from.is_some()
        || !matches!(
            root_design_form(sink.state(), root_id),
            DesignForm::Card | DesignForm::Deck
        )
    {
        return 0;
    }
    let rects = resolved_rects(sink.state());
    let Some(root) =
        op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(root_id))
    else {
        return 0;
    };
    let Ok(value) = serde_json::to_value(root) else {
        return 0;
    };
    let mut candidates = Vec::new();
    collect(&value, &rects, None, &mut candidates);
    if candidates.is_empty() {
        return 0;
    }
    let mut natural = measurement_copy(sink.state());
    for c in &candidates {
        natural.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(&c.id),
            patch_json: r#"{"textGrowth":"auto","width":"fit_content","height":"fit_content"}"#
                .into(),
            page_id: None,
        });
    }
    let single = resolved_rects(&natural);
    let mut fixes = Vec::new();
    for c in candidates {
        let (Some(full), Some(one)) = (rects.get(&c.id), single.get(&c.id)) else {
            continue;
        };
        if full.h < one.h * 1.5
            || full.h > one.h * 2.5
            || one.w <= c.available + EPS
            || one.w * (1.0 - MAX_SHRINK) > c.available
        {
            continue;
        }
        let mut prefix = measurement_copy(sink.state());
        let chars: Vec<char> = c.content.chars().collect();
        let orphan = (1..=2).any(|tail| {
            if !chars[chars.len() - tail..].iter().all(|ch| cjk(*ch)) {
                return false;
            }
            prefix.apply(EditorCommand::SetNodeText {
                node_id: NodeId::new(&c.id),
                text: chars[..chars.len() - tail].iter().collect(),
            });
            resolved_rects(&prefix)
                .get(&c.id)
                .is_some_and(|r| (r.h - one.h).abs() <= EPS)
        });
        if !orphan {
            continue;
        }
        let floor = (c.size * (1.0 - MAX_SHRINK)).ceil();
        let mut size = (c.size * c.available / one.w)
            .floor()
            .max(floor)
            .min(c.size - 1.0);
        let mut authored = measurement_copy(sink.state());
        let mut verified = false;
        for _ in 0..16 {
            for state in [&mut authored, &mut natural] {
                state.apply(EditorCommand::SetNodeFontSize {
                    node_id: NodeId::new(&c.id),
                    font_size: size as f32,
                });
            }
            let fixed = resolved_rects(&authored);
            let auto = resolved_rects(&natural);
            if let (Some(f), Some(n)) = (fixed.get(&c.id), auto.get(&c.id)) {
                if (f.h - n.h).abs() <= EPS && n.w <= c.available + EPS {
                    verified = true;
                    break;
                }
            }
            if size <= floor {
                break;
            }
            size -= 1.0;
        }
        if verified {
            fixes.push(EditorCommand::SetNodeFontSize {
                node_id: NodeId::new(c.id),
                font_size: size as f32,
            });
        }
    }
    fixes
        .into_iter()
        .filter(|cmd| sink.apply(cmd.clone()))
        .count()
}

#[cfg(test)]
#[path = "headline_tail_fit_tests.rs"]
mod tests;
