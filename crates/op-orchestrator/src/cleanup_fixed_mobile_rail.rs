//! Lift explicit persistent chrome through ordinary page gutters, preserving width.
use super::*;
use jian_scene::layout_scene::SceneNode;

struct Candidate {
    id: String,
    gutter: f64,
    action: bool,
}

pub(super) fn promote_nested_chrome(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(candidate) = candidate(sink.state(), root_id) else {
        return false;
    };
    if candidate.gutter == 0.0 {
        return move_footer(sink, &candidate.id, root_id);
    }
    // The original bar remains the same node with the same internal padding,
    // events and width. Only the page-owned gutter moves outside the scroller.
    let wrapper: PenNode = serde_json::from_value(json!({
        "type":"frame", "id":"persistent-chrome-rail", "name":"Persistent chrome rail",
        "role":if candidate.action { "bottom-action-bar" } else { "bottom-tab-bar" },
        "layout":"vertical", "width":"fill_container", "height":"fit_content",
        "padding":[0,candidate.gutter,0,candidate.gutter], "children":[]
    }))
    .expect("valid persistent chrome rail");
    let Some(ids) = sink.insert_subtree_returning_root_ids(vec![wrapper], &NodeId::new(root_id))
    else {
        return false;
    };
    let Some(wrapper_id) = ids.first() else {
        return false;
    };
    if move_footer(sink, &candidate.id, wrapper_id) {
        return true;
    }
    // A rejected move must not leave a new empty bar in the document.
    sink.apply(EditorCommand::DeleteNode {
        node_id: NodeId::new(wrapper_id),
        page_id: None,
    });
    false
}

fn move_footer(sink: &mut dyn DocSink, footer: &str, parent: &str) -> bool {
    sink.apply(EditorCommand::MoveNode {
        node_id: NodeId::new(footer),
        target_parent: NodeId::new(parent),
        page_id: None,
        index: None,
    })
}

fn candidate(state: &EditorState, root_id: &str) -> Option<Candidate> {
    let root = find_root(state, root_id)?;
    let mut tail = root.children()?.last()?;
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let page = scene.active_page()?;
    let board = page.find(root_id)?;
    let mut gutter = 0.0;
    for _ in 0..4 {
        let inset = if existing_scroll_viewport(tail) {
            0.0
        } else {
            if !crate::mobile_action_bar_plan::is_safe_full_page_wrapper(tail, false) {
                return None;
            }
            crate::mobile_action_bar_plan::full_page_wrapper_gutter(tail)?
        };
        let shell = page.find(tail.id_str())?;
        if shell.hidden
            || (shell.bounds.size.x as f64 - (board.bounds.size.x as f64 - 2.0 * gutter)).abs()
                > 1.0
        {
            return None;
        }
        gutter += inset;
        let children = tail.children()?;
        let (last, preceding) = children.split_last()?;
        // Multiple persistent bars in one shell have no unique ownership.
        if preceding.iter().any(is_bottom_chrome) {
            return None;
        }
        if is_bottom_chrome(last) {
            let footer = page.find(last.id_str())?;
            if !bounded_footer(last, footer)
                || (footer.bounds.size.x as f64 - (board.bounds.size.x as f64 - 2.0 * gutter)).abs()
                    > 1.0
            {
                return None;
            }
            return Some(Candidate {
                id: last.id_str().into(),
                gutter,
                action: crate::mobile_action_bar::is_action_bar(last),
            });
        }
        tail = last;
    }
    None
}

fn bounded_footer(node: &PenNode, scene: &SceneNode) -> bool {
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    if ["x", "y", "constraints", "position", "pin"]
        .iter()
        .any(|key| value.get(key).is_some())
        || !matches!(
            value["layout"].as_str(),
            None | Some("horizontal" | "vertical")
        )
        || !(value["width"].as_str() == Some("fill_container")
            || value["width"]
                .as_f64()
                .is_some_and(|n| n.is_finite() && n > 0.0))
        || !matches!(value.get("height"), None | Some(Value::Number(_)))
            && value["height"].as_str() != Some("fit_content")
        || scene.hidden
        || value["visible"].as_bool() == Some(false)
        || value
            .get("opacity")
            .is_some_and(|opacity| !opacity.as_f64().is_some_and(|n| n.is_finite() && n > 0.0))
        || numeric_padding(&value).is_none()
        || !(44.0..=128.0).contains(&scene.bounds.size.y)
    {
        return false;
    }
    let top = scene.bounds.origin.y;
    let bottom = top + scene.bounds.size.y;
    fn fits(node: &SceneNode, top: f32, bottom: f32) -> bool {
        node.hidden
            || (node.bounds.origin.y.is_finite()
                && node.bounds.size.y.is_finite()
                && node.bounds.origin.y >= top - 1.0
                && node.bounds.origin.y + node.bounds.size.y <= bottom + 1.0
                && node.children.iter().all(|child| fits(child, top, bottom)))
    }
    scene.children.iter().all(|child| fits(child, top, bottom))
}

fn existing_scroll_viewport(node: &PenNode) -> bool {
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    value["role"].as_str() == Some("scroll-area")
        && value["layout"].as_str() == Some("vertical")
        && value["height"].as_str() == Some("fill_container")
        && value["width"].as_str() == Some("fill_container")
        && value["clipContent"].as_bool() == Some(true)
        && ["x", "y", "constraints", "position", "pin"]
            .iter()
            .all(|key| value.get(key).is_none())
        && numeric_padding(&value).is_some_and(|pad| pad[1] == 0.0 && pad[3] == 0.0)
}

#[cfg(test)]
#[path = "cleanup_fixed_mobile_rail_tests.rs"]
mod tests;

// Unlike the general spacing reader, invalid expressions are not zero padding.
fn numeric_padding(node: &Value) -> Option<[f64; 4]> {
    let number = |value: &Value| value.as_f64().filter(|n| n.is_finite() && *n >= 0.0);
    match node.get("padding") {
        None => Some([0.0; 4]),
        Some(Value::Number(n)) => n
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .map(|n| [n; 4]),
        Some(Value::Array(a)) if a.len() == 2 => Some([
            number(&a[0])?,
            number(&a[1])?,
            number(&a[0])?,
            number(&a[1])?,
        ]),
        Some(Value::Array(a)) if a.len() == 4 => Some([
            number(&a[0])?,
            number(&a[1])?,
            number(&a[2])?,
            number(&a[3])?,
        ]),
        _ => None,
    }
}
