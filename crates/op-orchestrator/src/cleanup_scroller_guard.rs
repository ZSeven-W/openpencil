use jian_ops_schema::node::{container::LayoutMode, PenNode};
use op_editor_core::PenNodeExt;

pub(crate) fn is_intentional_horizontal_scroller(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        PenNode::Group(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        PenNode::Rectangle(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        _ => false,
    }
}

pub(crate) fn subtree_contains_intentional_horizontal_scroller(node: &PenNode) -> bool {
    is_intentional_horizontal_scroller(node)
        || node.children().is_some_and(|children| {
            children
                .iter()
                .any(subtree_contains_intentional_horizontal_scroller)
        })
}
