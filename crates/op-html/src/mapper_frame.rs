use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::{FrameNode, PenNode};

pub(super) fn frame(
    base: PenNodeBase,
    container: ContainerProps,
    children: Vec<PenNode>,
) -> PenNode {
    PenNode::Frame(FrameNode {
        base,
        container,
        children: Some(children),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        screen: None,
        breakpoint: None,
    })
}
