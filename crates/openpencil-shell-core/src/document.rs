//! Step 2 kill-spike — minimal `Document` model for the Rust shell.
//!
//! TS-side `apps/web` ships `pen-types::PenDocument` with ~25 node
//! types, design variables, multi-page layout, fills/strokes/effects,
//! component instances, and so on. Step 2 starts the Rust mirror with
//! the smallest surface that lets shell-core editor-UI widgets
//! (LayerPanel / PropertyPanel / Toolbar) consume something
//! resembling a real document. Phase parity (per memory
//! `feedback_rust_port_feature_parity.md`: "TS → Rust 移植必须含
//! v0.8.0+ 全部功能") is a journey — this commit lays the spine.
//!
//! Shape choices (matching pen-types loosely):
//!
//! - [`NodeId(u64)`] is the stable identifier for tree nodes.
//!   0 is reserved for an absent / "no selection" sentinel — see
//!   [`NodeId::is_real`]. Real ids start at 1.
//! - [`NodeKind`] enumerates the document-side node kinds. Step 2
//!   covers Frame / Group / Rect / Text — enough for the editor-UI
//!   demo. Component instances / images / paths land in Step 3+.
//! - [`Node`] is a recursive tree node holding kind + name +
//!   children. Fills / strokes / position / size are deliberately
//!   omitted for Slice 1 — the editor UI only needs name + structure to
//!   draw the LayerPanel; PropertyPanel uses kind for the row set.
//! - [`Page`] groups nodes; [`Document`] holds pages plus a
//!   selection sentinel.
//!
//! Mobile + wasm32 clean: this module imports nothing platform-
//! specific. shell-core stays on the existing wasm32-clean cargo
//! check baseline (spec §1.2) AND the new mobile (iOS / Android)
//! widget render stack (per 2026-05-10 user directive).

/// Stable identifier for [`Node`]s within a [`Document`]. The layer
/// host assigns these from a counter at insertion time; editor-UI
/// widgets convert them to `widgets::WidgetId` via
/// [`NodeId::to_widget_id`] when they need the renderer-side id.
///
/// Like [`crate::widgets::WidgetId`], `NodeId(0)` is reserved as
/// the "no node" sentinel — used by [`Document::selected`] to mean
/// "nothing selected". Real ids start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Sentinel id for "no node" (used by `Document::selected` and
    /// by sample fixtures that need a placeholder).
    pub const NONE: NodeId = NodeId(0);

    /// Returns `true` for non-sentinel ids (id != 0).
    pub const fn is_real(self) -> bool {
        self.0 != 0
    }

    /// Construct a real (non-sentinel) id; `debug_assert`s against 0
    /// in dev builds, mirroring `WidgetId::new`.
    #[inline]
    pub const fn new(id: u64) -> Self {
        debug_assert!(id != 0, "NodeId::new(0) — id 0 is reserved for NodeId::NONE");
        Self(id)
    }

    /// Convert to a editor-UI-side `WidgetId`. The mapping is identity
    /// today; both share the `pub u64` shape and reserve 0 as the
    /// "no node / no widget" sentinel. If the editor UI ever
    /// needs a wider numeric range (multiple widgets per node),
    /// this helper becomes the seam to evolve.
    #[inline]
    pub const fn to_widget_id(self) -> crate::widgets::WidgetId {
        crate::widgets::WidgetId(self.0)
    }
}

/// Node kinds covered by Step 2. Mirrors the most common subset of
/// `pen-types::PenNode.type` values seen in real documents:
/// frames hold layout, groups hold logical groupings, rects + text
/// are leaf primitives. The editor-UI demo uses `kind` to drive the
/// PropertyPanel's row set (different kinds expose different
/// properties).
///
/// `Other(String)` covers unknown / future kinds round-tripped from
/// a serialised document so the host never errors on a node it
/// doesn't recognise — the editor falls back to a generic property
/// row set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Frame,
    Group,
    Rect,
    Text,
    Other(String),
}

impl NodeKind {
    /// Short human-facing label used by the LayerPanel's secondary
    /// column and by the PropertyPanel header row.
    pub fn label(&self) -> &str {
        match self {
            NodeKind::Frame => "Frame",
            NodeKind::Group => "Group",
            NodeKind::Rect => "Rect",
            NodeKind::Text => "Text",
            NodeKind::Other(s) => s.as_str(),
        }
    }
}

/// Document tree node — id + kind + display name + children.
///
/// Step 2 deliberately omits fills / strokes / transform — editor-UI
/// widgets only need `id`, `kind`, `name`, `children` to draw the
/// inspector. Render-side primitives (Color, Rect bounds, etc.)
/// land in Step 3 alongside the canvas-render surface.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    pub children: Vec<Node>,
}

impl Node {
    pub fn leaf(id: u64, kind: NodeKind, name: impl Into<String>) -> Self {
        Self {
            id: NodeId::new(id),
            kind,
            name: name.into(),
            children: Vec::new(),
        }
    }

    pub fn with_children(
        id: u64,
        kind: NodeKind,
        name: impl Into<String>,
        children: Vec<Node>,
    ) -> Self {
        Self {
            id: NodeId::new(id),
            kind,
            name: name.into(),
            children,
        }
    }

    /// Search the subtree for a node with the given id, returning a
    /// borrow if found. Used by PropertyPanel to look up the
    /// currently-selected node when the editor host wants to draw
    /// per-node properties.
    pub fn find(&self, id: NodeId) -> Option<&Node> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(hit) = child.find(id) {
                return Some(hit);
            }
        }
        None
    }
}

/// A document page. `pen-types::PenPage` mirror — id + name +
/// top-level node children. Single-page documents (Step 2 default)
/// still go through this layer so the multi-page upgrade is just an
/// extension to `Document::pages.len() > 1`.
#[derive(Debug, Clone)]
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub children: Vec<Node>,
}

impl Page {
    pub fn new(id: u64, name: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            id: NodeId::new(id),
            name: name.into(),
            children,
        }
    }

    /// Walk the page's child forest looking for a node by id. Pages
    /// themselves are NOT searchable through this — only their
    /// descendants. The host typically asks "is the selection
    /// pointing at a real node?" via this helper.
    pub fn find(&self, id: NodeId) -> Option<&Node> {
        for child in &self.children {
            if let Some(hit) = child.find(id) {
                return Some(hit);
            }
        }
        None
    }
}

/// A document — the editor's subject. Holds pages plus a selection
/// sentinel; the rest of the document model (variables, components,
/// styles, artboards) is Step 3+.
#[derive(Debug, Clone)]
pub struct Document {
    pub pages: Vec<Page>,
    /// Currently-selected node id. `NodeId::NONE` = no selection.
    /// Multi-selection (a `Vec<NodeId>`) lands when the canvas
    /// needs it; the editor works with single-selection in Step 2.
    pub selected: NodeId,
}

impl Document {
    /// Empty document with one empty default page named "Page 1".
    /// Used by host smoke fixtures.
    pub fn empty() -> Self {
        Self {
            pages: vec![Page::new(1, "Page 1", Vec::new())],
            selected: NodeId::NONE,
        }
    }

    /// Sample document for the Step 2 editor-UI demo: one page with a
    /// frame containing a title (text) + a button (group of rect +
    /// text). Mirrors the WidgetHost::sample shape from Step 1b but
    /// driven by document data instead of hardcoded TreeWidget
    /// items. Selection is set to the title so PropertyPanel has
    /// something to render.
    pub fn sample() -> Self {
        // Id allocations: page=1, frame=10, title=11, button=12,
        // button_rect=13, button_text=14. Stable across runs so
        // tests can assert specific ids.
        let title = Node::leaf(11, NodeKind::Text, "Title");
        let button_rect = Node::leaf(13, NodeKind::Rect, "Button background");
        let button_text = Node::leaf(14, NodeKind::Text, "Click me");
        let button = Node::with_children(12, NodeKind::Group, "Button", vec![button_rect, button_text]);
        let frame = Node::with_children(10, NodeKind::Frame, "Frame", vec![title, button]);
        Self {
            pages: vec![Page::new(1, "Page 1", vec![frame])],
            selected: NodeId::new(11), // "Title"
        }
    }

    /// Get the currently-selected node, if any. Walks pages in
    /// order and returns the first hit; selection is unique by
    /// design (single-selection in Step 2).
    pub fn selected_node(&self) -> Option<&Node> {
        if !self.selected.is_real() {
            return None;
        }
        for page in &self.pages {
            if let Some(node) = page.find(self.selected) {
                return Some(node);
            }
        }
        None
    }

    /// Convenience: first page (or panic in dev if empty). Chrome
    /// demos render `pages[0]` exclusively in Step 2; multi-page
    /// pickers come later.
    pub fn first_page(&self) -> &Page {
        self.pages
            .first()
            .expect("Document::first_page on empty pages — use Document::empty for a default page")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_sentinel_is_zero() {
        assert_eq!(NodeId::NONE.0, 0);
        assert!(!NodeId::NONE.is_real());
        assert!(NodeId::new(1).is_real());
    }

    #[test]
    fn node_id_to_widget_id_round_trips_inner_value() {
        let nid = NodeId::new(42);
        let wid = nid.to_widget_id();
        assert_eq!(wid.0, 42);
    }

    #[test]
    fn node_find_walks_subtree() {
        let leaf = Node::leaf(3, NodeKind::Rect, "leaf");
        let mid = Node::with_children(2, NodeKind::Group, "mid", vec![leaf]);
        let root = Node::with_children(1, NodeKind::Frame, "root", vec![mid]);
        assert_eq!(root.find(NodeId::new(1)).unwrap().name, "root");
        assert_eq!(root.find(NodeId::new(2)).unwrap().name, "mid");
        assert_eq!(root.find(NodeId::new(3)).unwrap().name, "leaf");
        assert!(root.find(NodeId::new(99)).is_none());
    }

    #[test]
    fn document_sample_has_expected_shape() {
        let doc = Document::sample();
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0].name, "Page 1");
        // Frame > [Title, Button > [bg, text]]
        let frame = &doc.pages[0].children[0];
        assert_eq!(frame.kind, NodeKind::Frame);
        assert_eq!(frame.children.len(), 2);
        assert_eq!(frame.children[0].kind, NodeKind::Text);
        assert_eq!(frame.children[1].kind, NodeKind::Group);
        assert_eq!(frame.children[1].children.len(), 2);
    }

    #[test]
    fn document_selected_node_returns_real_hit() {
        let doc = Document::sample();
        let sel = doc.selected_node().unwrap();
        assert_eq!(sel.id, NodeId::new(11));
        assert_eq!(sel.name, "Title");
        assert_eq!(sel.kind, NodeKind::Text);
    }

    #[test]
    fn document_empty_has_no_selection() {
        let doc = Document::empty();
        assert_eq!(doc.selected, NodeId::NONE);
        assert!(doc.selected_node().is_none());
    }

    #[test]
    fn document_find_unknown_id_is_none() {
        let mut doc = Document::sample();
        doc.selected = NodeId::new(9999);
        assert!(doc.selected_node().is_none());
    }

    #[test]
    fn node_kind_label_matches_variant() {
        assert_eq!(NodeKind::Frame.label(), "Frame");
        assert_eq!(NodeKind::Group.label(), "Group");
        assert_eq!(NodeKind::Rect.label(), "Rect");
        assert_eq!(NodeKind::Text.label(), "Text");
        assert_eq!(NodeKind::Other("Custom".into()).label(), "Custom");
    }
}
