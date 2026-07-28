use jian_ops_schema::{
    node::{ImageSrc, PenNode},
    state_override::{StyleOverride, WidgetStates},
    style::{PenFill, PenStroke},
    PenDocument,
};

use crate::{CollabMessage, CollabOp, CollabTxn};

const FILE_TABLE_REFERENCE_PREFIX: &str = "op-image:";

// Keep this typed traversal aligned with Jian's structural image-table visitor.
// Checking source strings before save-scope serialization prevents a literal
// file-table ref from being confused with a collector-generated ref of the
// same id.
pub(crate) fn message_has_external_image_ref(message: &CollabMessage) -> bool {
    match message {
        CollabMessage::Submit(message) => txn_has_external_image_ref(&message.txn),
        CollabMessage::Commit(message) => txn_has_external_image_ref(&message.txn),
        CollabMessage::Snapshot(snapshot) => document_has_external_image_ref(&snapshot.document),
        _ => false,
    }
}

pub(crate) fn txn_has_external_image_ref(txn: &CollabTxn) -> bool {
    txn.ops.iter().any(|operation| match operation {
        CollabOp::InsertExact { node, .. } | CollabOp::ReplaceExact { node, .. } => {
            node_has_external_image_ref(node)
        }
        CollabOp::DeleteExact { .. } | CollabOp::MoveExact { .. } => false,
    })
}

pub(crate) fn document_has_external_image_ref(document: &PenDocument) -> bool {
    let mut stack: Vec<&PenNode> = document.children.iter().collect();
    if let Some(pages) = &document.pages {
        for page in pages {
            stack.extend(page.children.iter());
        }
    }
    walk_nodes(stack)
}

pub(crate) fn node_has_external_image_ref(node: &PenNode) -> bool {
    walk_nodes(vec![node])
}

fn walk_nodes(mut stack: Vec<&PenNode>) -> bool {
    while let Some(node) = stack.pop() {
        if node_sources_have_external_ref(node) {
            return true;
        }
        if let Some(children) = node_children(node) {
            stack.extend(children);
        }
    }
    false
}

fn node_sources_have_external_ref(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(node) => container_has_external_ref(&node.container),
        PenNode::Group(node) => container_has_external_ref(&node.container),
        PenNode::Rectangle(node) => container_has_external_ref(&node.container),
        PenNode::Ellipse(node) => {
            style_has_external_ref(node.fill.as_deref(), node.stroke.as_ref(), None)
        }
        PenNode::Line(node) => stroke_has_external_ref(node.stroke.as_ref()),
        PenNode::Polygon(node) => {
            style_has_external_ref(node.fill.as_deref(), node.stroke.as_ref(), None)
        }
        PenNode::Path(node) => {
            style_has_external_ref(node.fill.as_deref(), node.stroke.as_ref(), None)
        }
        PenNode::Text(node) => fills_have_external_ref(node.fill.as_deref()),
        PenNode::TextInput(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Image(node) => source_is_external_ref(&node.src),
        PenNode::IconFont(node) => {
            style_has_external_ref(node.fill.as_deref(), node.stroke.as_ref(), None)
        }
        PenNode::TextArea(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Select(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Switch(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Checkbox(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Slider(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::RadioGroup(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::NumberInput(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Progress(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Tabs(node) => style_has_external_ref(
            node.fill.as_deref(),
            node.stroke.as_ref(),
            node.states.as_ref(),
        ),
        PenNode::Ref(_) => false,
    }
}

fn container_has_external_ref(container: &jian_ops_schema::node::ContainerProps) -> bool {
    style_has_external_ref(container.fill.as_deref(), container.stroke.as_ref(), None)
}

fn style_has_external_ref(
    fills: Option<&[PenFill]>,
    stroke: Option<&PenStroke>,
    states: Option<&WidgetStates>,
) -> bool {
    fills_have_external_ref(fills)
        || stroke_has_external_ref(stroke)
        || states.is_some_and(states_have_external_ref)
}

fn fills_have_external_ref(fills: Option<&[PenFill]>) -> bool {
    fills.is_some_and(|fills| {
        fills
            .iter()
            .any(|fill| matches!(fill, PenFill::Image(image) if source_is_external_ref(&image.url)))
    })
}

fn stroke_has_external_ref(stroke: Option<&PenStroke>) -> bool {
    stroke.is_some_and(|stroke| fills_have_external_ref(stroke.fill.as_deref()))
}

fn states_have_external_ref(states: &WidgetStates) -> bool {
    [
        states.hover.as_ref(),
        states.pressed.as_ref(),
        states.focused.as_ref(),
        states.disabled.as_ref(),
    ]
    .into_iter()
    .flatten()
    .any(style_override_has_external_ref)
}

fn style_override_has_external_ref(style: &StyleOverride) -> bool {
    fills_have_external_ref(style.fill.as_deref()) || stroke_has_external_ref(style.stroke.as_ref())
}

fn source_is_external_ref(source: &ImageSrc) -> bool {
    source.as_str().starts_with(FILE_TABLE_REFERENCE_PREFIX)
}

fn node_children(node: &PenNode) -> Option<&[PenNode]> {
    match node {
        PenNode::Frame(node) => node.children.as_deref(),
        PenNode::Group(node) => node.children.as_deref(),
        PenNode::Rectangle(node) => node.children.as_deref(),
        PenNode::Tabs(node) => node.children.as_deref(),
        PenNode::Ref(node) => node.children.as_deref(),
        _ => None,
    }
}
