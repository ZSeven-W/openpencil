use std::collections::BTreeMap;

use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;

pub fn rects_for_roots(roots: &[PenNode]) -> BTreeMap<String, [f32; 4]> {
    let mut out = BTreeMap::new();
    for root in roots {
        collect_rect(root, 0.0, 0.0, &mut out);
    }
    out
}

fn collect_rect(
    node: &PenNode,
    parent_x: f32,
    parent_y: f32,
    out: &mut BTreeMap<String, [f32; 4]>,
) {
    let base = base_of(node);
    let x = parent_x + base.x.unwrap_or(0.0) as f32;
    let y = parent_y + base.y.unwrap_or(0.0) as f32;
    let (w, h) = size_of(node);
    out.insert(base.id.clone(), [x, y, w, h]);
    for child in children_of(node) {
        collect_rect(child, x, y, out);
    }
}

fn size_of(node: &PenNode) -> (f32, f32) {
    match node {
        PenNode::Frame(n) => numeric_size(&n.container.width, &n.container.height),
        PenNode::Group(n) => numeric_size(&n.container.width, &n.container.height),
        PenNode::Rectangle(n) => numeric_size(&n.container.width, &n.container.height),
        PenNode::Ellipse(n) => numeric_size(&n.width, &n.height),
        PenNode::Polygon(n) => numeric_size(&n.width, &n.height),
        PenNode::Path(n) => numeric_size(&n.width, &n.height),
        PenNode::Text(n) => numeric_size(&n.width, &n.height),
        PenNode::TextInput(n) => numeric_size(&n.width, &n.height),
        PenNode::TextArea(n) => numeric_size(&n.width, &n.height),
        PenNode::Select(n) => numeric_size(&n.width, &n.height),
        PenNode::Switch(n) => numeric_size(&n.width, &n.height),
        PenNode::Checkbox(n) => numeric_size(&n.width, &n.height),
        PenNode::Slider(n) => numeric_size(&n.width, &n.height),
        PenNode::RadioGroup(n) => numeric_size(&n.width, &n.height),
        PenNode::NumberInput(n) => numeric_size(&n.width, &n.height),
        PenNode::Progress(n) => numeric_size(&n.width, &n.height),
        PenNode::Tabs(n) => numeric_size(&n.width, &n.height),
        PenNode::Image(n) => numeric_size(&n.width, &n.height),
        PenNode::IconFont(n) => numeric_size(&n.width, &n.height),
        PenNode::Line(_) => (f32::NAN, f32::NAN),
        PenNode::Ref(_) => (0.0, 0.0),
    }
}

fn numeric_size(w: &Option<SizingBehavior>, h: &Option<SizingBehavior>) -> (f32, f32) {
    (numeric(w), numeric(h))
}

fn numeric(value: &Option<SizingBehavior>) -> f32 {
    match value {
        Some(SizingBehavior::Number(n)) => *n as f32,
        _ => 0.0,
    }
}

fn children_of(node: &PenNode) -> &[PenNode] {
    match node {
        PenNode::Frame(n) => n.children.as_deref().unwrap_or(&[]),
        PenNode::Group(n) => n.children.as_deref().unwrap_or(&[]),
        PenNode::Rectangle(n) => n.children.as_deref().unwrap_or(&[]),
        PenNode::Tabs(n) => n.children.as_deref().unwrap_or(&[]),
        PenNode::Ref(n) => n.children.as_deref().unwrap_or(&[]),
        _ => &[],
    }
}

fn base_of(node: &PenNode) -> &PenNodeBase {
    match node {
        PenNode::Frame(n) => &n.base,
        PenNode::Group(n) => &n.base,
        PenNode::Rectangle(n) => &n.base,
        PenNode::Ellipse(n) => &n.base,
        PenNode::Line(n) => &n.base,
        PenNode::Polygon(n) => &n.base,
        PenNode::Path(n) => &n.base,
        PenNode::Text(n) => &n.base,
        PenNode::TextInput(n) => &n.base,
        PenNode::TextArea(n) => &n.base,
        PenNode::Select(n) => &n.base,
        PenNode::Switch(n) => &n.base,
        PenNode::Checkbox(n) => &n.base,
        PenNode::Slider(n) => &n.base,
        PenNode::RadioGroup(n) => &n.base,
        PenNode::NumberInput(n) => &n.base,
        PenNode::Progress(n) => &n.base,
        PenNode::Tabs(n) => &n.base,
        PenNode::Image(n) => &n.base,
        PenNode::IconFont(n) => &n.base,
        PenNode::Ref(n) => &n.base,
    }
}
