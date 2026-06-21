use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{first_solid_fill_hex, EditorCommand, NodeId, PenNodeExt};

pub(crate) fn repair_mobile_structural_chrome(sink: &mut dyn DocSink, root_id: &str) {
    let repairs = {
        let Some(root) = super::find_root(sink.state(), root_id) else {
            return;
        };
        if !super::is_mobile_root(root) {
            return;
        }
        let root_width = root.width_px().unwrap_or(0.0);
        let Some(children) = root.children() else {
            return;
        };
        let mut repairs = MobileChromeRepairs::default();
        for child in children {
            if should_strip_structural_shell(child) {
                repairs
                    .structural_shells
                    .push(NodeId::new(child.id_str().to_string()));
            }
            collect_bottom_nav_chrome_repairs(child, root_width, &mut repairs);
        }
        repairs
    };

    for node_id in repairs.structural_shells {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: r#"{"fill":null,"stroke":null,"effects":null,"cornerRadius":0}"#
                .to_string(),
            page_id: None,
        });
    }
    for (node_id, root_width) in repairs.bottom_nav_surfaces {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: format!(
                r#"{{"x":0,"width":{},"stroke":null,"effects":null,"cornerRadius":0}}"#,
                root_width.round()
            ),
            page_id: None,
        });
    }
    for node_id in repairs.bottom_nav_items {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: r#"{"fill":null,"stroke":null,"effects":null,"cornerRadius":0}"#
                .to_string(),
            page_id: None,
        });
    }
}

#[derive(Default)]
struct MobileChromeRepairs {
    structural_shells: Vec<NodeId>,
    bottom_nav_surfaces: Vec<(NodeId, f64)>,
    bottom_nav_items: Vec<NodeId>,
}

fn should_strip_structural_shell(node: &PenNode) -> bool {
    if !node.is_container()
        || node
            .children()
            .map(|children| children.is_empty())
            .unwrap_or(true)
        || super::is_status_bar(node)
        || super::nav_surface_target(node).is_some()
    {
        return false;
    }

    if has_nested_filled_atomic_component(node) {
        return true;
    }
    if is_visual_surface_role(node) {
        return false;
    }

    let hay = super::node_identity_haystack(node);
    let section_like = node
        .base()
        .role
        .as_deref()
        .map(|role| role.eq_ignore_ascii_case("section"))
        .unwrap_or(false);
    (section_like || hay.contains("section"))
        && (is_search_or_category_haystack(&hay) || has_search_or_category_child(node))
}

fn is_visual_surface_role(node: &PenNode) -> bool {
    matches!(
        node.base()
            .role
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "badge"
            | "button"
            | "card"
            | "chip"
            | "feature-card"
            | "form-input"
            | "icon-button"
            | "image-card"
            | "input"
            | "pricing-card"
            | "search-bar"
            | "stat-card"
            | "tag"
    )
}

fn has_nested_filled_atomic_component(node: &PenNode) -> bool {
    let Some(parent_role) = normalized_role(node) else {
        return false;
    };
    if !is_atomic_protected_role(&parent_role) {
        return false;
    }
    node.children()
        .map(|children| {
            children.iter().any(|child| {
                let child_role = normalized_role(child);
                if child_role.as_deref() == Some(parent_role.as_str()) {
                    return true;
                }
                if child_role
                    .as_deref()
                    .map(is_primary_atomic_role)
                    .unwrap_or(false)
                    && first_solid_fill_hex(child).is_some()
                {
                    return true;
                }
                child_role.is_none()
                    && first_solid_fill_hex(child).is_some()
                    && is_input_like_haystack(&super::node_identity_haystack(child))
            })
        })
        .unwrap_or(false)
}

fn normalized_role(node: &PenNode) -> Option<String> {
    node.base()
        .role
        .as_deref()
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .map(str::to_lowercase)
}

fn is_atomic_protected_role(role: &str) -> bool {
    matches!(
        role,
        "badge"
            | "button"
            | "chip"
            | "form-input"
            | "icon-button"
            | "input"
            | "pill"
            | "search-bar"
            | "tag"
    )
}

fn is_primary_atomic_role(role: &str) -> bool {
    matches!(role, "form-input" | "input" | "search-bar")
}

fn is_input_like_haystack(hay: &str) -> bool {
    super::contains_any(hay, &["form input", "form-input", "input", "search"])
}

fn collect_bottom_nav_chrome_repairs(
    root_child: &PenNode,
    root_width: f64,
    repairs: &mut MobileChromeRepairs,
) {
    let Some(nav) = bottom_nav_surface_target(root_child) else {
        return;
    };
    repairs
        .bottom_nav_surfaces
        .push((NodeId::new(nav.id_str().to_string()), root_width));

    let Some(children) = nav.children() else {
        return;
    };
    if children.len() < 3 {
        return;
    }
    for child in children {
        if is_bottom_nav_item(child) {
            repairs
                .bottom_nav_items
                .push(NodeId::new(child.id_str().to_string()));
        }
    }
}

fn bottom_nav_surface_target(root_child: &PenNode) -> Option<&PenNode> {
    if is_bottom_nav_surface(root_child) {
        return Some(root_child);
    }
    let children = root_child.children()?;
    if root_child
        .base()
        .role
        .as_deref()
        .map(|role| role.eq_ignore_ascii_case("section"))
        .unwrap_or(false)
        && children.len() == 1
    {
        return children
            .first()
            .filter(|inner| is_bottom_nav_surface(inner));
    }
    None
}

fn is_bottom_nav_surface(node: &PenNode) -> bool {
    let role = normalized_role(node);
    if matches!(role.as_deref(), Some("bottom-tab-bar")) {
        return true;
    }
    let hay = super::node_identity_haystack(node);
    super::contains_any(
        &hay,
        &[
            "bottom nav",
            "bottom-nav",
            "bottom navigation",
            "bottom-navigation",
            "bottom tab",
            "bottom-tab",
        ],
    )
}

fn is_bottom_nav_item(node: &PenNode) -> bool {
    if !node.is_container() {
        return false;
    }
    if matches!(
        normalized_role(node).as_deref(),
        Some(
            "button"
                | "icon-button"
                | "nav-item"
                | "nav-item-active"
                | "pill"
                | "search-bar"
                | "tab"
                | "tab-active"
        )
    ) {
        return true;
    }
    let hay = super::node_identity_haystack(node);
    super::contains_any(
        &hay,
        &[
            "account", "cart", "discover", "home", "likes", "orders", "profile", "search", "tab",
        ],
    )
}

fn has_search_or_category_child(node: &PenNode) -> bool {
    node.children()
        .map(|children| {
            children
                .iter()
                .any(|child| is_search_or_category_haystack(&super::node_identity_haystack(child)))
        })
        .unwrap_or(false)
}

fn is_search_or_category_haystack(hay: &str) -> bool {
    super::contains_any(
        hay,
        &["categor", "category", "chip", "filter", "search", "sliders"],
    )
}
