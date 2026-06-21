use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

pub(crate) fn repair_mobile_structural_chrome(sink: &mut dyn DocSink, root_id: &str) {
    let shells: Vec<NodeId> = {
        let Some(root) = super::find_root(sink.state(), root_id) else {
            return;
        };
        if !super::is_mobile_root(root) {
            return;
        }
        let Some(children) = root.children() else {
            return;
        };
        children
            .iter()
            .filter(|child| should_strip_structural_shell(child))
            .map(|child| NodeId::new(child.id_str().to_string()))
            .collect()
    };

    for node_id in shells {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: r#"{"fill":null,"stroke":null,"effects":null,"cornerRadius":0}"#
                .to_string(),
            page_id: None,
        });
    }
}

fn should_strip_structural_shell(node: &PenNode) -> bool {
    if !node.is_container()
        || node
            .children()
            .map(|children| children.is_empty())
            .unwrap_or(true)
        || super::is_status_bar(node)
        || super::nav_surface_target(node).is_some()
        || is_visual_surface_role(node)
    {
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
