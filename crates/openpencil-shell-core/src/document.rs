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
/// `NodeId::NONE` (inner value 0) is reserved as the "no node"
/// sentinel — used by [`Document::selected`] to mean "nothing
/// selected". Real ids start at 1.
///
/// The inner `u64` is **private** so the only way to mint an id is
/// via [`NodeId::new`] (panics on 0 in BOTH debug + release) or
/// the [`NodeId::NONE`] constant. Per codex Step 2 R1 BLOCK: the
/// previous `pub u64` field let release-mode callers construct
/// `NodeId(0)` Nodes that would shadow the NONE sentinel and
/// confuse `Document::selected_node`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    /// Sentinel id for "no node" (used by `Document::selected` and
    /// by sample fixtures that need a placeholder).
    pub const NONE: NodeId = NodeId(0);

    /// Returns `true` for non-sentinel ids (id != 0).
    pub const fn is_real(self) -> bool {
        self.0 != 0
    }

    /// Construct a real (non-sentinel) id. Panics in BOTH debug
    /// and release builds if `id == 0` — callers that need the
    /// sentinel use [`NodeId::NONE`] directly. Hard panic (not
    /// `debug_assert`) per codex Step 2 R1 BLOCK fix.
    #[inline]
    pub const fn new(id: u64) -> Self {
        if id == 0 {
            panic!("NodeId::new(0) — id 0 is reserved for NodeId::NONE");
        }
        Self(id)
    }

    /// Inner numeric id. Used by `to_widget_id`, by serde (Step
    /// 4+), and by tests that need to inspect the underlying value.
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Convert to an editor-UI-side `WidgetId`. The mapping is
    /// identity today; both share the `u64` payload and reserve 0
    /// as the "no node / no widget" sentinel. If the editor UI
    /// ever needs a wider numeric range (multiple widgets per
    /// node), this helper becomes the seam to evolve.
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

/// A document — the editor's subject. Holds pages, the active
/// page index, and a selection sentinel; the rest of the document
/// model (variables, components, styles, artboards) is Step 3+.
///
/// Selection is **page-scoped** by design (codex Step 2 R1
/// CONCERN-1): the editor UI only ever shows one page at a time
/// (`active_page_index`), so a "selection" outside that page is
/// effectively no selection. This keeps LayerPanel and
/// PropertyPanel pinned to the same content.
#[derive(Debug, Clone)]
pub struct Document {
    pub pages: Vec<Page>,
    /// Index into `pages` for the page currently shown in the
    /// editor viewport. Defaults to 0; multi-page picker UI lives
    /// in Step 3+.
    pub active_page_index: usize,
    /// Currently-selected node id WITHIN the active page.
    /// `NodeId::NONE` = no selection. Multi-selection (a
    /// `Vec<NodeId>`) lands when the editor needs it.
    pub selected: NodeId,
}

impl Document {
    /// Empty document with one empty default page named "Page 1".
    /// Used by host smoke fixtures.
    pub fn empty() -> Self {
        Self {
            pages: vec![Page::new(1, "Page 1", Vec::new())],
            active_page_index: 0,
            selected: NodeId::NONE,
        }
    }

    /// Sample document for the Step 2 editor-UI demo: one page
    /// with a frame containing a title (text) + a button (group
    /// of rect + text). Driven by document data instead of
    /// hardcoded TreeWidget items. Selection is set to the title
    /// so PropertyPanel has something to render.
    pub fn sample() -> Self {
        // Id allocations: page=1, frame=10, title=11, button=12,
        // button_rect=13, button_text=14. Stable across runs so
        // tests can assert specific ids.
        let title = Node::leaf(11, NodeKind::Text, "Title");
        let button_rect = Node::leaf(13, NodeKind::Rect, "Button background");
        let button_text = Node::leaf(14, NodeKind::Text, "Click me");
        let button = Node::with_children(12, NodeKind::Group, "Button", vec![button_rect, button_text]);
        let frame = Node::with_children(10, NodeKind::Frame, "Frame", vec![title, button]);
        let doc = Self {
            pages: vec![Page::new(1, "Page 1", vec![frame])],
            active_page_index: 0,
            selected: NodeId::new(11), // "Title"
        };
        debug_assert!(
            doc.validate().is_ok(),
            "Document::sample() failed self-validation"
        );
        doc
    }

    /// The page currently shown in the editor viewport. Returns
    /// `None` if `active_page_index` is out of range (only happens
    /// after an external mutation that didn't preserve the
    /// invariant; callers can use `Document::validate` to detect).
    pub fn active_page(&self) -> Option<&Page> {
        self.pages.get(self.active_page_index)
    }

    /// Get the currently-selected node, if any. ONLY searches the
    /// active page (codex Step 2 R1 CONCERN-1). A selection on a
    /// non-active page returns `None` — the host either follows
    /// the selection by switching `active_page_index` first, or
    /// treats it as no-selection-in-view.
    pub fn selected_node(&self) -> Option<&Node> {
        if !self.selected.is_real() {
            return None;
        }
        self.active_page()?.find(self.selected)
    }

    /// Convenience: first page (or panic if pages is empty). Used
    /// by tests + sample fixtures that don't care about
    /// active-page semantics. New code should prefer
    /// `active_page` for the actual rendering target.
    pub fn first_page(&self) -> &Page {
        self.pages
            .first()
            .expect("Document::first_page on empty pages — use Document::empty for a default page")
    }

    /// Walk every node id in every page, returning the first
    /// duplicate id found (or `None` if all ids are unique). Used
    /// by `Document::sample()` debug-asserts and by `validate`.
    /// Codex Step 2 R1 CONCERN-2: previously nothing checked id
    /// uniqueness, so a Document built with duplicate ids would
    /// have `selected_node` returning the first hit while
    /// LayerPanel might mark several rows "selected".
    pub fn find_duplicate_id(&self) -> Option<NodeId> {
        let mut seen = std::collections::HashSet::new();
        for page in &self.pages {
            // Pages share the id namespace with nodes, so include
            // page ids in the uniqueness scan.
            if !seen.insert(page.id) {
                return Some(page.id);
            }
            for child in &page.children {
                if let Some(dup) = find_duplicate_walk(child, &mut seen) {
                    return Some(dup);
                }
            }
        }
        None
    }

    /// Run light invariant checks on the document. Returns `Err`
    /// with a human-readable message on the first violation:
    /// - duplicate node id anywhere in any page
    /// - `active_page_index` out of range (when there are pages)
    pub fn validate(&self) -> Result<(), String> {
        if let Some(dup) = self.find_duplicate_id() {
            return Err(format!("duplicate NodeId: {:?}", dup));
        }
        if !self.pages.is_empty() && self.active_page_index >= self.pages.len() {
            return Err(format!(
                "active_page_index {} out of range (pages.len()={})",
                self.active_page_index,
                self.pages.len()
            ));
        }
        Ok(())
    }
}

fn find_duplicate_walk(
    node: &Node,
    seen: &mut std::collections::HashSet<NodeId>,
) -> Option<NodeId> {
    if !seen.insert(node.id) {
        return Some(node.id);
    }
    for child in &node.children {
        if let Some(dup) = find_duplicate_walk(child, seen) {
            return Some(dup);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_sentinel_is_zero() {
        assert_eq!(NodeId::NONE.raw(), 0);
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

    #[test]
    #[should_panic(expected = "id 0 is reserved for NodeId::NONE")]
    fn node_id_new_zero_panics_in_release_too() {
        // Codex Step 2 R1 BLOCK fix: hard panic, not just
        // debug_assert. `#[should_panic]` test runs in both debug
        // and release configurations.
        let _ = NodeId::new(0);
    }

    #[test]
    fn document_sample_passes_validate() {
        // Codex Step 2 R1 CONCERN-2: sample() must be invariant-
        // clean (no duplicate ids, valid active_page_index).
        let doc = Document::sample();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn document_validate_catches_duplicate_node_id() {
        let doc = Document {
            pages: vec![Page::new(
                1,
                "p",
                vec![
                    Node::leaf(2, NodeKind::Rect, "a"),
                    Node::leaf(2, NodeKind::Rect, "b"), // dup id 2
                ],
            )],
            active_page_index: 0,
            selected: NodeId::NONE,
        };
        let result = doc.validate();
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("duplicate NodeId"),
            "validate should mention duplicate"
        );
    }

    #[test]
    fn document_validate_catches_active_page_index_out_of_range() {
        let mut doc = Document::sample();
        doc.active_page_index = 99;
        let result = doc.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("active_page_index"));
    }

    #[test]
    fn document_selected_node_scopes_to_active_page() {
        // Codex Step 2 R1 CONCERN-1: a selection on page 1 must
        // NOT show up when page 2 is active.
        let page1 = Page::new(
            1,
            "p1",
            vec![Node::leaf(10, NodeKind::Rect, "P1-A")],
        );
        let page2 = Page::new(
            2,
            "p2",
            vec![Node::leaf(20, NodeKind::Rect, "P2-A")],
        );
        let doc = Document {
            pages: vec![page1, page2],
            active_page_index: 1, // page 2 active
            selected: NodeId::new(10), // selection points at page 1's node
        };
        // Selection is on a non-active page → returns None.
        assert!(doc.selected_node().is_none());

        // Switch active page to 0 and the same selection now
        // resolves.
        let doc2 = Document {
            active_page_index: 0,
            ..doc
        };
        assert_eq!(doc2.selected_node().unwrap().name, "P1-A");
    }

    #[test]
    fn document_active_page_returns_indexed_page() {
        let doc = Document {
            pages: vec![
                Page::new(1, "first", vec![]),
                Page::new(2, "second", vec![]),
            ],
            active_page_index: 1,
            selected: NodeId::NONE,
        };
        assert_eq!(doc.active_page().unwrap().name, "second");
    }

    #[test]
    fn document_active_page_returns_none_when_index_out_of_range() {
        let doc = Document {
            pages: vec![Page::new(1, "only", vec![])],
            active_page_index: 5,
            selected: NodeId::NONE,
        };
        assert!(doc.active_page().is_none());
    }

    #[test]
    fn node_id_raw_returns_inner_value() {
        assert_eq!(NodeId::new(42).raw(), 42);
        assert_eq!(NodeId::NONE.raw(), 0);
    }
}
