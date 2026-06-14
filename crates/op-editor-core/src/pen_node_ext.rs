//! Uniform tree-access layer over `jian_ops_schema::node::PenNode`.
//!
//! `PenNode` is an `enum` of 12 variants; each carries a flattened
//! `base: PenNodeBase` (id / name / x / y / rotation / visible /
//! locked) plus per-variant geometry + an optional `children` vec on
//! the container-ish variants (Frame / Group / Rectangle / Ref).
//!
//! shell-core's `Document` used a flat `Node` struct, so its mutators
//! could touch `node.id` / `node.children` / `node.bounds` directly.
//! `op-editor-core` operates on the canonical `PenNode` instead — this
//! module is the shim that gives the ported mutators the same uniform
//! accessors (`id`, `children`, `children_mut`, geometry getters /
//! setters) without each mutator having to `match` all 12 variants.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::node::{AlignItems, JustifyContent, LayoutMode, PenNodeBase};
use jian_ops_schema::sizing::SizingBehavior;

/// Read a `SizingBehavior` as a concrete pixel number. Keyword /
/// expression sizing is child-derived (`fit_content`) or variable —
/// neither yields a literal px, so they read as `None`.
pub fn sizing_px(s: &Option<SizingBehavior>) -> Option<f64> {
    match s {
        Some(SizingBehavior::Number(n)) => Some(*n),
        _ => None,
    }
}

/// Tree-access surface every editor mutator needs. Implemented for
/// `PenNode` so a mutator can treat any variant uniformly.
pub trait PenNodeExt {
    /// Borrow the shared `PenNodeBase` (id / name / geometry / flags).
    fn base(&self) -> &PenNodeBase;
    /// Mutably borrow the shared `PenNodeBase`.
    fn base_mut(&mut self) -> &mut PenNodeBase;

    /// Borrow this node's children, if it is a container variant.
    fn children(&self) -> Option<&Vec<PenNode>>;
    /// Mutably borrow this node's children, if it is a container
    /// variant. Frame / Group / Rectangle / Ref carry `children`.
    fn children_mut(&mut self) -> Option<&mut Vec<PenNode>>;

    /// True when this variant can hold children (Frame / Group /
    /// Rectangle / Ref). Used by reparenting + group ops.
    fn is_container(&self) -> bool;

    /// True when this node is a `Group` — the ungroup target.
    fn is_group(&self) -> bool;

    /// True when this node is an auto-layout (flex) container —
    /// `layout: Vertical | Horizontal`. Its children are flow-
    /// positioned by the layout engine, NOT by stored `x` / `y`, so a
    /// move of the container must leave the children un-positioned.
    fn is_auto_layout_container(&self) -> bool;

    /// Explicit pixel width, when the variant carries one and it is
    /// a literal number (not `fit_content` / a `$variable`).
    fn width_px(&self) -> Option<f64>;
    /// Explicit pixel height — parallel to [`PenNodeExt::width_px`].
    fn height_px(&self) -> Option<f64>;
    /// Overwrite the variant's width with a literal pixel number.
    /// No-op on variants without a width field (Line / Text-input-
    /// less kinds handled per-variant).
    fn set_width_px(&mut self, w: f64);
    /// Overwrite the variant's height with a literal pixel number.
    fn set_height_px(&mut self, h: f64);

    /// Stable string id (from `base`).
    fn id_str(&self) -> &str {
        &self.base().id
    }
}

/// Macro body — every variant routes `base` / `base_mut` to its
/// flattened `base` field. Cuts a 12-arm match to one line per node.
macro_rules! base_arms {
    ($self:ident) => {
        match $self {
            PenNode::Frame(n) => &n.base,
            PenNode::Group(n) => &n.base,
            PenNode::Rectangle(n) => &n.base,
            PenNode::Ellipse(n) => &n.base,
            PenNode::Line(n) => &n.base,
            PenNode::Polygon(n) => &n.base,
            PenNode::Path(n) => &n.base,
            PenNode::Text(n) => &n.base,
            PenNode::TextInput(n) => &n.base,
            PenNode::Image(n) => &n.base,
            PenNode::IconFont(n) => &n.base,
            PenNode::TextArea(n) => &n.base,
            PenNode::Select(n) => &n.base,
            PenNode::Switch(n) => &n.base,
            PenNode::Checkbox(n) => &n.base,
            PenNode::Slider(n) => &n.base,
            PenNode::RadioGroup(n) => &n.base,
            PenNode::NumberInput(n) => &n.base,
            PenNode::Progress(n) => &n.base,
            PenNode::Tabs(n) => &n.base,
            PenNode::Ref(n) => &n.base,
        }
    };
}

macro_rules! base_mut_arms {
    ($self:ident) => {
        match $self {
            PenNode::Frame(n) => &mut n.base,
            PenNode::Group(n) => &mut n.base,
            PenNode::Rectangle(n) => &mut n.base,
            PenNode::Ellipse(n) => &mut n.base,
            PenNode::Line(n) => &mut n.base,
            PenNode::Polygon(n) => &mut n.base,
            PenNode::Path(n) => &mut n.base,
            PenNode::Text(n) => &mut n.base,
            PenNode::TextInput(n) => &mut n.base,
            PenNode::Image(n) => &mut n.base,
            PenNode::IconFont(n) => &mut n.base,
            PenNode::TextArea(n) => &mut n.base,
            PenNode::Select(n) => &mut n.base,
            PenNode::Switch(n) => &mut n.base,
            PenNode::Checkbox(n) => &mut n.base,
            PenNode::Slider(n) => &mut n.base,
            PenNode::RadioGroup(n) => &mut n.base,
            PenNode::NumberInput(n) => &mut n.base,
            PenNode::Progress(n) => &mut n.base,
            PenNode::Tabs(n) => &mut n.base,
            PenNode::Ref(n) => &mut n.base,
        }
    };
}

impl PenNodeExt for PenNode {
    fn base(&self) -> &PenNodeBase {
        base_arms!(self)
    }

    fn base_mut(&mut self) -> &mut PenNodeBase {
        base_mut_arms!(self)
    }

    fn children(&self) -> Option<&Vec<PenNode>> {
        match self {
            PenNode::Frame(n) => n.children.as_ref(),
            PenNode::Group(n) => n.children.as_ref(),
            PenNode::Rectangle(n) => n.children.as_ref(),
            PenNode::Tabs(n) => n.children.as_ref(),
            PenNode::Ref(n) => n.children.as_ref(),
            _ => None,
        }
    }

    fn children_mut(&mut self) -> Option<&mut Vec<PenNode>> {
        match self {
            PenNode::Frame(n) => Some(n.children.get_or_insert_with(Vec::new)),
            PenNode::Group(n) => Some(n.children.get_or_insert_with(Vec::new)),
            PenNode::Rectangle(n) => Some(n.children.get_or_insert_with(Vec::new)),
            PenNode::Tabs(n) => Some(n.children.get_or_insert_with(Vec::new)),
            PenNode::Ref(n) => Some(n.children.get_or_insert_with(Vec::new)),
            _ => None,
        }
    }

    fn is_container(&self) -> bool {
        matches!(
            self,
            PenNode::Frame(_)
                | PenNode::Group(_)
                | PenNode::Rectangle(_)
                | PenNode::Tabs(_)
                | PenNode::Ref(_)
        )
    }

    fn is_group(&self) -> bool {
        matches!(self, PenNode::Group(_))
    }

    fn is_auto_layout_container(&self) -> bool {
        use jian_ops_schema::node::container::LayoutMode;
        let layout = match self {
            PenNode::Frame(n) => n.container.layout.as_ref(),
            PenNode::Group(n) => n.container.layout.as_ref(),
            PenNode::Rectangle(n) => n.container.layout.as_ref(),
            _ => None,
        };
        matches!(layout, Some(LayoutMode::Vertical | LayoutMode::Horizontal))
    }

    fn width_px(&self) -> Option<f64> {
        match self {
            PenNode::Frame(n) => sizing_px(&n.container.width),
            PenNode::Group(n) => sizing_px(&n.container.width),
            PenNode::Rectangle(n) => sizing_px(&n.container.width),
            PenNode::Ellipse(n) => sizing_px(&n.width),
            PenNode::Polygon(n) => sizing_px(&n.width),
            PenNode::Path(n) => sizing_px(&n.width),
            PenNode::Text(n) => sizing_px(&n.width),
            PenNode::TextInput(n) => sizing_px(&n.width),
            PenNode::Image(n) => sizing_px(&n.width),
            PenNode::IconFont(n) => sizing_px(&n.width),
            PenNode::TextArea(n) => sizing_px(&n.width),
            PenNode::Select(n) => sizing_px(&n.width),
            PenNode::Switch(n) => sizing_px(&n.width),
            PenNode::Checkbox(n) => sizing_px(&n.width),
            PenNode::Slider(n) => sizing_px(&n.width),
            PenNode::RadioGroup(n) => sizing_px(&n.width),
            PenNode::NumberInput(n) => sizing_px(&n.width),
            PenNode::Progress(n) => sizing_px(&n.width),
            PenNode::Tabs(n) => sizing_px(&n.width),
            PenNode::Line(_) | PenNode::Ref(_) => None,
        }
    }

    fn height_px(&self) -> Option<f64> {
        match self {
            PenNode::Frame(n) => sizing_px(&n.container.height),
            PenNode::Group(n) => sizing_px(&n.container.height),
            PenNode::Rectangle(n) => sizing_px(&n.container.height),
            PenNode::Ellipse(n) => sizing_px(&n.height),
            PenNode::Polygon(n) => sizing_px(&n.height),
            PenNode::Path(n) => sizing_px(&n.height),
            PenNode::Text(n) => sizing_px(&n.height),
            PenNode::TextInput(n) => sizing_px(&n.height),
            PenNode::Image(n) => sizing_px(&n.height),
            PenNode::IconFont(n) => sizing_px(&n.height),
            PenNode::TextArea(n) => sizing_px(&n.height),
            PenNode::Select(n) => sizing_px(&n.height),
            PenNode::Switch(n) => sizing_px(&n.height),
            PenNode::Checkbox(n) => sizing_px(&n.height),
            PenNode::Slider(n) => sizing_px(&n.height),
            PenNode::RadioGroup(n) => sizing_px(&n.height),
            PenNode::NumberInput(n) => sizing_px(&n.height),
            PenNode::Progress(n) => sizing_px(&n.height),
            PenNode::Tabs(n) => sizing_px(&n.height),
            PenNode::Line(_) | PenNode::Ref(_) => None,
        }
    }

    fn set_width_px(&mut self, w: f64) {
        let v = SizingBehavior::Number(w.max(0.0));
        match self {
            PenNode::Frame(n) => n.container.width = Some(v),
            PenNode::Group(n) => n.container.width = Some(v),
            PenNode::Rectangle(n) => n.container.width = Some(v),
            PenNode::Ellipse(n) => n.width = Some(v),
            PenNode::Polygon(n) => n.width = Some(v),
            PenNode::Path(n) => n.width = Some(v),
            PenNode::Text(n) => n.width = Some(v),
            PenNode::TextInput(n) => n.width = Some(v),
            PenNode::Image(n) => n.width = Some(v),
            PenNode::IconFont(n) => n.width = Some(v),
            PenNode::TextArea(n) => n.width = Some(v),
            PenNode::Select(n) => n.width = Some(v),
            PenNode::Switch(n) => n.width = Some(v),
            PenNode::Checkbox(n) => n.width = Some(v),
            PenNode::Slider(n) => n.width = Some(v),
            PenNode::RadioGroup(n) => n.width = Some(v),
            PenNode::NumberInput(n) => n.width = Some(v),
            PenNode::Progress(n) => n.width = Some(v),
            PenNode::Tabs(n) => n.width = Some(v),
            PenNode::Line(_) | PenNode::Ref(_) => {}
        }
    }

    fn set_height_px(&mut self, h: f64) {
        let v = SizingBehavior::Number(h.max(0.0));
        match self {
            PenNode::Frame(n) => n.container.height = Some(v),
            PenNode::Group(n) => n.container.height = Some(v),
            PenNode::Rectangle(n) => n.container.height = Some(v),
            PenNode::Ellipse(n) => n.height = Some(v),
            PenNode::Polygon(n) => n.height = Some(v),
            PenNode::Path(n) => n.height = Some(v),
            PenNode::Text(n) => n.height = Some(v),
            PenNode::TextInput(n) => n.height = Some(v),
            PenNode::Image(n) => n.height = Some(v),
            PenNode::IconFont(n) => n.height = Some(v),
            PenNode::TextArea(n) => n.height = Some(v),
            PenNode::Select(n) => n.height = Some(v),
            PenNode::Switch(n) => n.height = Some(v),
            PenNode::Checkbox(n) => n.height = Some(v),
            PenNode::Slider(n) => n.height = Some(v),
            PenNode::RadioGroup(n) => n.height = Some(v),
            PenNode::NumberInput(n) => n.height = Some(v),
            PenNode::Progress(n) => n.height = Some(v),
            PenNode::Tabs(n) => n.height = Some(v),
            PenNode::Line(_) | PenNode::Ref(_) => {}
        }
    }
}

/// Build a bare `Group` node with the given id / name / children.
/// Used by `group_selected`. All optional container props default
/// to `None`, which the canonical schema serializes away.
pub fn make_group(id: String, name: &str, children: Vec<PenNode>) -> PenNode {
    use jian_ops_schema::node::{ContainerProps, GroupNode};
    PenNode::Group(GroupNode {
        base: PenNodeBase {
            id,
            name: Some(name.to_string()),
            ..Default::default()
        },
        container: ContainerProps::default(),
        children: Some(children),
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// Build a bare `Path` node with one starting anchor. Used by the
/// pen-tool builder. The path stores anchors in `anchors`; geometry
/// fields default to `None` until the bounds recompute writes them.
pub fn make_path(id: String, name: &str, first: (f64, f64)) -> PenNode {
    use jian_ops_schema::node::{PathNode, PenPathAnchor};
    PenNode::Path(PathNode {
        base: PenNodeBase {
            id,
            name: Some(name.to_string()),
            x: Some(first.0),
            y: Some(first.1),
            ..Default::default()
        },
        icon_id: None,
        d: None,
        anchors: Some(vec![PenPathAnchor {
            x: first.0,
            y: first.1,
            handle_in: None,
            handle_out: None,
            point_type: None,
        }]),
        closed: None,
        width: None,
        height: None,
        fill: None,
        stroke: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// Layout-mode helpers — exposed so `mutators` can map the editor's
/// `Free / Vertical / Horizontal` onto the canonical `LayoutMode`.
pub fn layout_mode_eq(a: &Option<LayoutMode>, b: LayoutMode) -> bool {
    matches!(a, Some(m) if std::mem::discriminant(m) == std::mem::discriminant(&b))
}

/// Re-export so call sites that only need the alignment enums don't
/// have to reach into `jian_ops_schema` directly.
pub use jian_ops_schema::node::{AlignItems as OpsAlignItems, JustifyContent as OpsJustifyContent};

#[allow(dead_code)]
fn _enum_use(_a: AlignItems, _j: JustifyContent) {}
