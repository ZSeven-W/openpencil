//! Keep a fixed phone's navigation and action bars visible while content scrolls.
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
    // Chrome normalization tags the INNER tab row, not the outer wrapper
    // containing its divider/safe area. Recognize that same whole section.
    let Some(nav) = children.last().filter(|_| {
        root.children()
            .and_then(|nodes| nodes.last())
            .is_some_and(is_bottom_chrome)
    }) else {
        return promote_nested_chrome(sink, root_id) && repair(sink, root_id);
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
        || !(44.0..=128.0).contains(&nav_rect.bounds.size.y)
    {
        return false;
    }
    let prefix = usize::from(
        children
            .first()
            .is_some_and(crate::cleanup::is_status_bar_from_json),
    );
    let middle = &children[prefix..children.len() - 1];
    // Reserve the complete child row plus chrome padding. A centered row
    // taller than the parent's inner box grows its bounds only halfway;
    // merely copying that partial overflow back as height is not sufficient.
    let nav_kids = nav.get("children").and_then(Value::as_array);
    let pad = super::padding(nav);
    let required = nav_kids
        .filter(|kids| {
            !kids.is_empty()
                && kids.iter().all(|child| {
                    child.get("constraints").is_none()
                        && child.get("x").is_none()
                        && child.get("y").is_none()
                })
        })
        .filter(|_| nav.get("layout").and_then(Value::as_str) == Some("horizontal"))
        .map(|kids| {
            kids.iter()
                .filter_map(|child| {
                    child
                        .get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| page.find(id))
                })
                .map(|child| child.bounds.size.y as f64)
                .fold(0.0, f64::max)
                + pad[0]
                + pad[2]
        });
    let needs_nav_height = nav.get("constraints").is_none()
        && nav.get("x").is_none()
        && nav.get("y").is_none()
        && nav
            .get("height")
            .and_then(Value::as_f64)
            .is_some_and(|h| required.is_some_and(|needed| needed > h + 1.0 && needed <= 96.0));
    if let [viewport] = middle {
        if viewport.get("role").and_then(Value::as_str) == Some("scroll-area")
            && viewport.get("constraints").is_none()
            && viewport.get("x").is_none()
            && viewport.get("y").is_none()
            && viewport.get("clipContent").and_then(Value::as_bool) == Some(true)
            && viewport.get("height").and_then(Value::as_str) == Some("fill_container")
            && needs_nav_height
        {
            return sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(nav_id),
                patch_json: json!({"height":required.unwrap().ceil()}).to_string(),
                page_id: None,
            });
        }
    }
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
    // The first insertion must reserve the full intrinsic nav row too. Waiting
    // for another finalize leaves a centered nested tab row below the viewport.
    if needs_nav_height {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(nav_id),
            patch_json: json!({"height":required.unwrap().ceil()}).to_string(),
            page_id: None,
        });
    }
    true
}

pub(super) fn is_bottom_chrome(node: &PenNode) -> bool {
    if crate::cleanup::is_trailing_bottom_nav_section(node) {
        return true;
    }
    // A cart/checkout bar is persistent chrome too, but an ordinary last
    // button or card must not become a pinned footer just because it is last.
    let identity = crate::cleanup::node_identity_haystack(node);
    let action_bar = matches!(
        node.base().role.as_deref(),
        Some("bottom-action-bar" | "cart-bar" | "checkout-bar" | "submit-bar")
    ) || [
        "cart-bar",
        "cartbar",
        "cart bar",
        "checkout-bar",
        "bottom action",
        "bottom-action",
        "submit-bar",
        "底部购物车",
        "购物车栏",
        "底部操作栏",
        "底部提交",
    ]
    .iter()
    .any(|name| identity.contains(name));
    node.is_container() && action_bar && has_action(node)
}

fn promote_nested_chrome(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let footer_id = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return false;
        };
        let Some(mut tail) = root.children().and_then(|children| children.last()) else {
            return false;
        };
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
        let Some(page) = scene.active_page() else {
            return false;
        };
        let Some(board) = page.find(root_id) else {
            return false;
        };
        let mut candidate = None;
        // Whole-screen generations can nest persistent chrome inside a plain
        // full-width content shell. Move only its bounded trailing bar; never
        // pull an action out of a card or an inset/positioned composition.
        for _ in 0..3 {
            let Ok(shell) = serde_json::to_value(tail) else {
                return false;
            };
            if shell.get("layout").and_then(Value::as_str) != Some("vertical")
                || shell.get("constraints").is_some()
                || shell.get("x").is_some()
                || shell.get("y").is_some()
                || shell
                    .get("role")
                    .and_then(Value::as_str)
                    .is_some_and(|role| !matches!(role, "content" | "main" | "app-content"))
                || padding(&shell).iter().any(|p| *p != 0.0)
                || !page
                    .find(tail.id_str())
                    .is_some_and(|node| (node.bounds.size.x - board.bounds.size.x).abs() < 1.0)
            {
                break;
            }
            let Some(last) = tail.children().and_then(|children| children.last()) else {
                break;
            };
            if is_bottom_chrome(last) {
                if page.find(last.id_str()).is_some_and(|footer| {
                    (44.0..=128.0).contains(&footer.bounds.size.y)
                        && footer.bounds.origin.y + footer.bounds.size.y
                            > board.bounds.origin.y + board.bounds.size.y + 1.0
                }) {
                    candidate = Some(NodeId::new(last.id_str()));
                }
                break;
            }
            tail = last;
        }
        candidate
    };
    footer_id.is_some_and(|node_id| {
        sink.apply(EditorCommand::MoveNode {
            node_id,
            target_parent: NodeId::new(root_id),
            page_id: None,
            index: None,
        })
    })
}

fn has_action(node: &PenNode) -> bool {
    matches!(node.base().role.as_deref(), Some("button" | "icon-button"))
        || crate::wire_screen_navigation::node_has_events(node)
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_action))
}
