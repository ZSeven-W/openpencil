//! Root-level geometry probes: authored origin, base coordinates and
//! the available size / sizing behaviour a layout pass starts from.

use super::*;

/// `(base.x, base.y)` for any `PenNode`, defaulting to `(0, 0)`
/// when the schema didn't author them. Used by `compute_layout`
/// to offset taffy's root-relative rects onto the infinite canvas.
///
/// Public (paired with [`root_available_size`]) so the Canvas Preview
/// (Play) path in `op-host-native` can translate a scene-space tap back
/// into the jian runtime's root-relative hit-test space: the design
/// scene offsets every root by this origin, but the runtime lays each
/// root at its own (0, 0), so a tap must subtract the containing root's
/// authored origin before it reaches `Runtime::dispatch_pointer`.
pub fn root_authored_origin(n: &PenNode) -> (f32, f32) {
    let base = pen_base(n);
    (base.x.unwrap_or(0.0) as f32, base.y.unwrap_or(0.0) as f32)
}

/// `Some(_)` when the node authored an `x` or `y` — the explicit-absolute
/// signal `layout_repair`'s flow inference must yield to.
pub(crate) fn node_base_xy(n: &PenNode) -> Option<(f64, f64)> {
    let base = pen_base(n);
    match (base.x, base.y) {
        (None, None) => None,
        (x, y) => Some((x.unwrap_or(0.0), y.unwrap_or(0.0))),
    }
}

fn pen_base(n: &PenNode) -> &jian_ops_schema::node::base::PenNodeBase {
    match n {
        PenNode::Frame(f) => &f.base,
        PenNode::Group(g) => &g.base,
        PenNode::Rectangle(r) => &r.base,
        PenNode::Ellipse(e) => &e.base,
        PenNode::Line(l) => &l.base,
        PenNode::Polygon(p) => &p.base,
        PenNode::Path(p) => &p.base,
        PenNode::Text(t) => &t.base,
        PenNode::TextInput(t) => &t.base,
        PenNode::TextArea(t) => &t.base,
        PenNode::Select(s) => &s.base,
        PenNode::Switch(s) => &s.base,
        PenNode::Checkbox(c) => &c.base,
        PenNode::Slider(s) => &s.base,
        PenNode::RadioGroup(r) => &r.base,
        PenNode::NumberInput(n) => &n.base,
        PenNode::Progress(p) => &p.base,
        PenNode::Tabs(t) => &t.base,
        PenNode::Image(i) => &i.base,
        PenNode::IconFont(i) => &i.base,
        PenNode::Ref(r) => &r.base,
    }
}

/// Choose the (available_width, available_height) the layout engine
/// solves the root against. Numeric roots use their authored size;
/// flex-token roots fall back to a generous default that doesn't
/// drive taffy into trying to wrap a 1×1 canvas.
///
/// Public so the Canvas Preview (Play) path in `op-host-native` lays
/// the document out against the SAME available size as this static
/// design-canvas path — see `op_host_native::preview`. Laying a
/// `fill_container` root against the whole editor canvas region (as
/// the preview previously did via `runtime.build_layout(canvas_size)`)
/// expands the root and scatters its flex children; mirroring the
/// design canvas keeps both surfaces bit-identical.
pub fn root_available_size(root: &PenNode) -> (f32, f32) {
    let (w_sizing, h_sizing) = root_sizing(root);
    let w = match w_sizing {
        Some(SizingBehavior::Number(n)) => n as f32,
        _ => ROOT_FALLBACK_W,
    };
    let h = match h_sizing {
        Some(SizingBehavior::Number(n)) => n as f32,
        _ => ROOT_FALLBACK_H,
    };
    (w, h)
}

fn root_sizing(root: &PenNode) -> (Option<SizingBehavior>, Option<SizingBehavior>) {
    match root {
        PenNode::Frame(f) => (f.container.width.clone(), f.container.height.clone()),
        PenNode::Group(g) => (g.container.width.clone(), g.container.height.clone()),
        PenNode::Rectangle(r) => (r.container.width.clone(), r.container.height.clone()),
        PenNode::Ellipse(e) => (e.width.clone(), e.height.clone()),
        PenNode::Text(t) => (t.width.clone(), t.height.clone()),
        PenNode::TextInput(t) => (t.width.clone(), t.height.clone()),
        PenNode::TextArea(t) => (t.width.clone(), t.height.clone()),
        PenNode::Select(s) => (s.width.clone(), s.height.clone()),
        PenNode::Switch(s) => (s.width.clone(), s.height.clone()),
        PenNode::Checkbox(c) => (c.width.clone(), c.height.clone()),
        PenNode::Slider(s) => (s.width.clone(), s.height.clone()),
        PenNode::Image(i) => (i.width.clone(), i.height.clone()),
        PenNode::IconFont(i) => (i.width.clone(), i.height.clone()),
        PenNode::Polygon(p) => (p.width.clone(), p.height.clone()),
        PenNode::Path(p) => (p.width.clone(), p.height.clone()),
        _ => (None, None),
    }
}
