//! Keep a fixed phone's navigation visible when its content needs scrolling.
use super::*;
use serde_json::{json, Value};

pub(super) fn repair(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(root) = find_root(sink.state(), root_id) else {
        return false;
    };
    let Ok(value) = serde_json::to_value(root) else {
        return false;
    };
    let width = value.get("width").and_then(Value::as_f64).unwrap_or(0.0);
    let height = value.get("height").and_then(Value::as_f64).unwrap_or(0.0);
    if !(240.0..=480.0).contains(&width)
        || height < 500.0
        || value.get("layout").and_then(Value::as_str) != Some("vertical")
        || value.get("clipContent").and_then(Value::as_bool) == Some(false)
    {
        return false;
    }
    let Some(children) = value.get("children").and_then(Value::as_array) else {
        return false;
    };
    let Some(nav) = children
        .last()
        .filter(|n| n.get("role").and_then(Value::as_str) == Some("bottom-tab-bar"))
    else {
        return false;
    };
    let Some(nav_id) = nav.get("id").and_then(Value::as_str) else {
        return false;
    };
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(page) = scene.active_page() else {
        return false;
    };
    let (Some(board), Some(nav_rect)) = (page.find(root_id), page.find(nav_id)) else {
        return false;
    };
    if nav_rect.bounds.origin.y + nav_rect.bounds.size.y
        <= board.bounds.origin.y + board.bounds.size.y + 1.0
        || !(44.0..=96.0).contains(&nav_rect.bounds.size.y)
    {
        return false;
    }
    let prefix = usize::from(
        children
            .first()
            .is_some_and(|c| c.get("role").and_then(Value::as_str) == Some("status-bar")),
    );
    let middle = &children[prefix..children.len() - 1];
    if middle.is_empty()
        || middle.iter().any(|n| {
            n.get("constraints").is_some()
                || n.get("x").is_some()
                || n.get("y").is_some()
                || n.get("role").and_then(Value::as_str) == Some("scroll-area")
        })
    {
        return false;
    }
    let ids: Vec<_> = middle
        .iter()
        .filter_map(|n| n.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    if ids.len() != middle.len() {
        return false;
    }
    let gap = value.get("gap").and_then(Value::as_f64).unwrap_or(0.0);
    let wrapper: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"content-scroll","name":"Scrollable content","role":"scroll-area",
        "width":"fill_container","height":"fill_container","layout":"vertical","gap":gap,"clipContent":true,
        "state":{"scrolled":{"type":"bool","default":false}},
        "events":{"onScroll":[{"set":{"$state.scrolled":"true"}}]},"children":[]
    })).expect("valid scroll viewport");
    let Some(inserted) =
        sink.insert_subtree_returning_root_ids(vec![wrapper], &NodeId::new(root_id))
    else {
        return false;
    };
    let Some(wrapper_id) = inserted.first() else {
        return false;
    };
    sink.apply(EditorCommand::MoveNode {
        node_id: NodeId::new(wrapper_id),
        target_parent: NodeId::new(root_id),
        page_id: None,
        index: Some(prefix),
    });
    for (index, id) in ids.iter().enumerate() {
        sink.apply(EditorCommand::MoveNode {
            node_id: NodeId::new(id),
            target_parent: NodeId::new(wrapper_id),
            page_id: None,
            index: Some(index),
        });
    }
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(root_id),
        patch_json: json!({"gap":0}).to_string(),
        page_id: None,
    });
    true
}
