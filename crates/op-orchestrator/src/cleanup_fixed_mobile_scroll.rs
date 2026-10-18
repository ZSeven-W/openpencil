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
    if repair_intrinsic_action_inset(sink, root_id, &value) {
        repair(sink, root_id);
        return true;
    }
    let Some(children) = value.get("children").and_then(Value::as_array) else {
        return false;
    };
    // Chrome normalization tags the INNER tab row, not the outer wrapper
    // containing its divider/safe area. Recognize that same whole section.
    let Some(nav) = children.last().filter(|_| {
        find_root(sink.state(), root_id)
            .and_then(|root| root.children().and_then(|nodes| nodes.last()))
            .is_some_and(is_bottom_chrome)
    }) else {
        if promote_nested_chrome(sink, root_id) {
            repair(sink, root_id);
            return true;
        }
        return repair_without_footer(sink, root_id, &value);
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
    // Explicit bottom chrome stays at the viewport edge on short pages too.
    // Otherwise a more compact, successful generation leaves a floating tab
    // bar and an unrelated strip of page background underneath it.
    if !(44.0..=128.0).contains(&nav_rect.bounds.size.y) {
        return false;
    }
    let bottom_gap = board.bounds.origin.y + board.bounds.size.y
        - nav_rect.bounds.origin.y
        - nav_rect.bounds.size.y;
    // Preserve a small authored closing inset; it is already bottom chrome.
    if (-1.0..=8.0).contains(&bottom_gap) {
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
    if !wrap_content(sink, root_id, &value, prefix, middle, 0.0) {
        return false;
    }
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

// Intrinsic vertical action bars can grow their closing inset without
// shrinking controls or changing the requested viewport. Fixed/positioned
// compositions and expression-authored padding need their own reflow.
fn repair_intrinsic_action_inset(sink: &mut dyn DocSink, root_id: &str, root: &Value) -> bool {
    let Some(bar) = root
        .get("children")
        .and_then(Value::as_array)
        .and_then(|c| c.last())
    else {
        return false;
    };
    let numeric_padding = |node: &Value| {
        node.get("padding").is_none_or(|pad| {
            pad.is_number()
                || pad
                    .as_array()
                    .is_some_and(|v| matches!(v.len(), 2 | 4) && v.iter().all(Value::is_number))
        })
    };
    if bar.get("layout").and_then(Value::as_str) != Some("vertical")
        || bar.get("height").and_then(Value::as_str) != Some("fit_content")
        || !numeric_padding(root)
        || !numeric_padding(bar)
        || ["constraints", "x", "y"]
            .iter()
            .any(|k| bar.get(k).is_some())
        || !bar
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|kids| {
                !kids.is_empty()
                    && kids.iter().all(|child| {
                        ["constraints", "x", "y"]
                            .iter()
                            .all(|k| child.get(k).is_none())
                    })
            })
        || !find_root(sink.state(), root_id)
            .and_then(|r| r.children().and_then(|c| c.last()))
            .is_some_and(crate::mobile_action_bar::is_action_bar)
    {
        return false;
    }
    let mut inset = padding(bar);
    let outer = padding(root)[2];
    let Some(closing_inset) = intrinsic_closing_inset(bar) else {
        return false;
    };
    if closing_inset + outer >= 24.0 {
        return false;
    }
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(page) = scene.active_page() else {
        return false;
    };
    let Some(id) = bar.get("id").and_then(Value::as_str) else {
        return false;
    };
    let (Some(board), Some(footer)) = (page.find(root_id), page.find(id)) else {
        return false;
    };
    let extra = 28.0 - outer - closing_inset;
    if (footer.bounds.size.x - board.bounds.size.x).abs() > 1.0
        || !(44.0..=128.0).contains(&footer.bounds.size.y)
        || footer.bounds.size.y as f64 + extra > 128.0
    {
        return false;
    }
    inset[2] += extra;
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(id),
        patch_json: json!({"padding":inset}).to_string(),
        page_id: None,
    })
}

// Content rails and inner bar wrappers can already own the safe area. Count
// trailing intrinsic flow. A horizontal row contributes its own padding, not
// a shorter trailing control's padding; fixed controls' internal padding stays
// inside their hit area and is not closing space below the control.
fn intrinsic_closing_inset(node: &Value) -> Option<f64> {
    if node.get("padding").is_some_and(|pad| {
        !pad.is_number()
            && !pad
                .as_array()
                .is_some_and(|v| matches!(v.len(), 2 | 4) && v.iter().all(Value::is_number))
    }) {
        return None;
    }
    if node
        .get("children")
        .and_then(Value::as_array)
        .is_some_and(|children| {
            children.iter().any(|child| {
                ["constraints", "x", "y"]
                    .iter()
                    .any(|key| child.get(key).is_some())
            })
        })
    {
        return None;
    }
    let mut inset = padding(node)[2];
    if node.get("layout").and_then(Value::as_str) == Some("horizontal") {
        return Some(inset);
    }
    if let Some(child) = node
        .get("children")
        .and_then(Value::as_array)
        .and_then(|children| children.last())
        .filter(|child| {
            matches!(
                child.get("layout").and_then(Value::as_str),
                Some("vertical" | "horizontal")
            ) && child.get("height").and_then(Value::as_str) == Some("fit_content")
        })
    {
        if ["constraints", "x", "y"]
            .iter()
            .any(|key| child.get(key).is_some())
        {
            return None;
        }
        inset += intrinsic_closing_inset(child)?;
    }
    Some(inset)
}

// A detail screen can have status chrome without a tab bar. Recognize that
// explicit phone contract, not arbitrary narrow posters or trailing cards.
fn repair_without_footer(sink: &mut dyn DocSink, root_id: &str, value: &Value) -> bool {
    let Some(children) = value.get("children").and_then(Value::as_array) else {
        return false;
    };
    if !children
        .first()
        .is_some_and(crate::cleanup::is_status_bar_from_json)
    {
        return false;
    }
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(board) = scene.active_page().and_then(|page| page.find(root_id)) else {
        return false;
    };
    if super::flow_bottom(value, board)
        <= (board.bounds.origin.y + board.bounds.size.y) as f64 + 1.0
    {
        return false;
    }
    // Preserve the existing single-screen player/control layout when its blank
    // space can supply a complete closing inset. That probe is transactional:
    // an overflowing detail which cannot fit keeps all its authored spacing.
    if super::repair_bottom_room(sink, root_id) {
        return true;
    }
    wrap_content(sink, root_id, value, 1, &children[1..], 28.0)
}

fn wrap_content(
    sink: &mut dyn DocSink,
    root_id: &str,
    value: &Value,
    prefix: usize,
    middle: &[Value],
    bottom_inset: f64,
) -> bool {
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
        "padding":[0,0,bottom_inset,0],
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

pub(super) fn is_bottom_chrome(node: &PenNode) -> bool {
    crate::cleanup::is_trailing_bottom_nav_section(node)
        || crate::mobile_action_bar::is_action_bar(node)
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
            // Older saves may already have wrapped the explicitly pinned CTA
            // into a viewport. Reuse that viewport; preserve its closing inset.
            let pad = padding(&shell);
            let viewport = shell.get("role").and_then(Value::as_str) == Some("scroll-area")
                && shell.get("clipContent").and_then(Value::as_bool) == Some(true)
                && shell.get("height").and_then(Value::as_str) == Some("fill_container")
                && pad[1] == 0.0
                && pad[3] == 0.0;
            if shell.get("layout").and_then(Value::as_str) != Some("vertical")
                || shell.get("constraints").is_some()
                || shell.get("x").is_some()
                || shell.get("y").is_some()
                || shell
                    .get("role")
                    .and_then(Value::as_str)
                    .is_some_and(|role| {
                        !matches!(role, "content" | "main" | "app-content") && !viewport
                    })
                || (!viewport && pad.iter().any(|p| *p != 0.0))
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
