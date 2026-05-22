//! Tree-walk helpers + small leaf utilities for `LayerPanel` —
//! extracted from `layer_panel.rs` so the spine stays under the
//! 800-line cap.
//!
//! Phase 6 migration: these walk the canonical `PenNode` tree owned
//! by `op_editor_core::EditorState` instead of shell-core's old flat
//! `Node`. The widget-facing `LayerItem` still carries a shell-core
//! `document::NodeId` so the hosts' layer hit-test path stays
//! untouched (the two id types are both string newtypes, so the
//! conversion at the walk boundary is lossless).

use crate::widgets::icons::Icon;
use crate::widgets::layer_panel::{
    LAYER_ROW_HEIGHT, PAGE_ROW_HEIGHT, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::Rect;
use op_editor_core::NodeId;

use jian_ops_schema::node::PenNode;
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::ui_draft::LayerContextTarget;
use op_editor_core::EditorState;

use super::layer_panel::{LayerItem, PageItem};

/// Snapshot of the active inline rename (page or layer), pulled off
/// `EditorState.ui.layer_rename` so the panel builders can apply the
/// draft + the `renaming` flag without re-reaching into the draft
/// state on every row.
#[derive(Default)]
pub(super) struct RenameView<'a> {
    page: Option<(usize, &'a str)>,
    layer: Option<(&'a str, &'a str)>,
}

impl<'a> RenameView<'a> {
    /// Read the active inline-rename draft off the editor state.
    pub(super) fn from_state(state: &'a EditorState) -> Self {
        match state.ui.layer_rename.as_ref() {
            Some(s) => match &s.target {
                LayerContextTarget::Page(i) => Self {
                    page: Some((*i, s.draft.as_str())),
                    layer: None,
                },
                LayerContextTarget::Layer(id) => Self {
                    page: None,
                    layer: Some((id.as_str(), s.draft.as_str())),
                },
            },
            None => Self::default(),
        }
    }
}

/// Build the page-row list from the editor's pages, applying the
/// active inline-rename draft (if any). A single-page canonical
/// document (`pages == None`) yields one synthetic "Page 1" row.
pub(super) fn pages_from_state(state: &EditorState, rename: &RenameView<'_>) -> Vec<PageItem> {
    let active = state.ui.active_page_index;
    let hovered = state.editor_ui.hovered_page_index;
    let build = |i: usize, name: &str| -> PageItem {
        let renaming = rename.page.map(|(p, _)| p == i).unwrap_or(false);
        PageItem {
            page_index: i,
            label: match rename.page {
                Some((p, draft)) if p == i => draft.to_string(),
                _ => name.to_string(),
            },
            active: i == active,
            hovered: hovered == Some(i),
            renaming,
        }
    };
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages
            .iter()
            .enumerate()
            .map(|(i, p)| build(i, &p.name))
            .collect(),
        _ => vec![build(0, "Page 1")],
    }
}

/// Apply the active layer-rename draft onto the matching row.
pub(super) fn apply_layer_rename(items: &mut [LayerItem], rename: &RenameView<'_>) {
    if let Some((id, draft)) = rename.layer {
        for item in items.iter_mut() {
            if item.node_id.as_str() == id {
                item.label = draft.to_string();
                item.renaming = true;
            }
        }
    }
}

/// Inputs the walk needs that are not on the node itself — the
/// selection set, the hovered row, and the collapsed-layer set.
/// Bundled so the recursive walk threads one borrow instead of four.
pub(super) struct WalkCx<'a> {
    pub selected: &'a op_editor_core::SelectionState,
    pub hovered: Option<&'a op_editor_core::NodeId>,
    pub ui: &'a EditorUiState,
}

impl<'a> WalkCx<'a> {
    /// Build the walk context from an `EditorState`.
    pub(super) fn from_state(state: &'a EditorState) -> Self {
        Self {
            selected: &state.selection,
            hovered: state.editor_ui.hovered_layer_id.as_ref(),
            ui: &state.editor_ui,
        }
    }
}

/// Convert a canonical id into the widget-facing shell-core `NodeId`.
/// Both are string newtypes, so this never loses information.
fn to_doc_id(id: &str) -> NodeId {
    NodeId::new(id.to_string())
}

/// Build one `LayerItem` from a `PenNode` (without recursing).
fn item_for(node: &PenNode, cx: &WalkCx<'_>, depth: u8) -> LayerItem {
    let base = node.base();
    let canon = op_editor_core::NodeId::new(base.id.clone());
    let has_children = node.children().map(|c| !c.is_empty()).unwrap_or(false);
    LayerItem {
        node_id: to_doc_id(&base.id),
        label: base.name.clone().unwrap_or_default(),
        kind_label: kind_label(node).to_string(),
        icon: icon_for_node(node),
        depth,
        selected: cx.selected.contains(&canon),
        has_children,
        hidden: base.visible == Some(false),
        locked: base.locked.unwrap_or(false),
        collapsed: cx.ui.collapsed_layers.contains(&canon),
        hovered: cx.hovered.map(|h| h.as_str() == base.id).unwrap_or(false),
        is_container: matches!(node, PenNode::Frame(_) | PenNode::Group(_)),
        renaming: false,
    }
}

/// Recursively flatten `node` and its (non-collapsed) subtree into
/// `out`. Collapsed nodes hide their children from the LayerPanel —
/// a tree-view-only concern, canvas paint is unaffected.
pub(super) fn walk(node: &PenNode, cx: &WalkCx<'_>, depth: u8, out: &mut Vec<LayerItem>) {
    let item = item_for(node, cx, depth);
    let collapsed = item.collapsed;
    out.push(item);
    if collapsed {
        return;
    }
    if let Some(children) = node.children() {
        for child in children {
            walk(child, cx, depth.saturating_add(1), out);
        }
    }
}

/// Variant of `walk` that skips `excluded`'s entire subtree. Used by
/// the drag-in-progress panel build so the rendered row stack mirrors
/// the post-commit layout.
pub(super) fn walk_excluding(
    node: &PenNode,
    cx: &WalkCx<'_>,
    excluded: &NodeId,
    depth: u8,
    out: &mut Vec<LayerItem>,
) {
    if node.base().id == excluded.as_str() {
        return;
    }
    let item = item_for(node, cx, depth);
    let collapsed = item.collapsed;
    out.push(item);
    if collapsed {
        return;
    }
    if let Some(children) = node.children() {
        for child in children {
            walk_excluding(child, cx, excluded, depth.saturating_add(1), out);
        }
    }
}

/// Human-readable kind label for the layer row's `kind_label`.
pub(super) fn kind_label(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "Frame",
        PenNode::Group(_) => "Group",
        PenNode::Rectangle(_) => "Rectangle",
        PenNode::Ellipse(_) => "Ellipse",
        PenNode::Line(_) => "Line",
        PenNode::Polygon(_) => "Polygon",
        PenNode::Path(_) => "Path",
        PenNode::Text(_) => "Text",
        PenNode::TextInput(_) => "Text Input",
        PenNode::Image(_) => "Image",
        PenNode::IconFont(_) => "Icon",
        PenNode::Ref(_) => "Component",
    }
}

/// Map a `PenNode` variant onto a LayerPanel row icon.
pub(super) fn icon_for_node(node: &PenNode) -> Icon {
    match node {
        PenNode::Frame(_) => Icon::Hash,
        PenNode::Group(_) => Icon::Square,
        PenNode::Rectangle(_) => Icon::Square,
        PenNode::Ellipse(_) => Icon::Circle,
        PenNode::Polygon(_) => Icon::Triangle,
        PenNode::Line(_) => Icon::Minus,
        PenNode::Path(_) => Icon::PenTool,
        PenNode::Text(_) | PenNode::TextInput(_) => Icon::Type,
        PenNode::Image(_) => Icon::Square,
        PenNode::IconFont(_) => Icon::Square,
        PenNode::Ref(_) => Icon::Square,
    }
}

/// Max height (px) of the LayerPanel's Pages-section row viewport —
/// roughly six rows; a longer page list scrolls within it.
pub const LAYER_PAGES_VIEW_MAX: f32 = PAGE_ROW_HEIGHT * 6.0;

/// Resolved geometry of the LayerPanel's two bounded scroll regions
/// (Pages + Layers). Paint, hit-test and the drop-target walk all
/// derive from this single source so they stay aligned.
#[derive(Debug, Clone, Copy)]
pub struct LayerRegions {
    /// y of the Pages section header.
    pub pages_header_y: f32,
    /// Top y of the clipped page-row viewport.
    pub pages_rows_top: f32,
    /// Height of the page-row viewport.
    pub pages_view_h: f32,
    /// Pages scroll offset, clamped to the scrollable range.
    pub pages_scroll: f32,
    /// Largest valid Pages scroll offset (`content - viewport`).
    pub pages_max_scroll: f32,
    /// y of the Layers section header.
    pub layers_header_y: f32,
    /// Top y of the clipped layer-row viewport.
    pub layers_rows_top: f32,
    /// Height of the layer-row viewport (fills the rail's tail).
    pub layers_view_h: f32,
    /// Layers scroll offset, clamped to the scrollable range.
    pub layers_scroll: f32,
    /// Largest valid Layers scroll offset (`content - viewport`).
    pub layers_max_scroll: f32,
}

/// Compute the bounded Pages / Layers scroll-region geometry for a
/// LayerPanel painted into `rect`. `pages_scroll` / `layers_scroll`
/// are the raw stored offsets; the returned values are clamped to
/// each region's scrollable range.
pub fn layer_regions(
    rect: Rect,
    pages_len: usize,
    items_len: usize,
    pages_scroll: f32,
    layers_scroll: f32,
) -> LayerRegions {
    let pages_header_y = rect.origin.y + 8.0;
    let pages_rows_top = pages_header_y + SECTION_HEADER_HEIGHT;
    let pages_content = pages_len as f32 * PAGE_ROW_HEIGHT;
    let pages_view_h = pages_content.min(LAYER_PAGES_VIEW_MAX);
    let pages_max_scroll = (pages_content - pages_view_h).max(0.0);
    let pages_scroll = pages_scroll.clamp(0.0, pages_max_scroll);

    let layers_header_y = pages_rows_top + pages_view_h + SECTION_GAP;
    let layers_rows_top = layers_header_y + SECTION_HEADER_HEIGHT;
    let layers_view_h = (rect.origin.y + rect.size.y - 8.0 - layers_rows_top).max(0.0);
    let layers_content = items_len.max(1) as f32 * LAYER_ROW_HEIGHT;
    let layers_max_scroll = (layers_content - layers_view_h).max(0.0);
    let layers_scroll = layers_scroll.clamp(0.0, layers_max_scroll);

    LayerRegions {
        pages_header_y,
        pages_rows_top,
        pages_view_h,
        pages_scroll,
        pages_max_scroll,
        layers_header_y,
        layers_rows_top,
        layers_view_h,
        layers_scroll,
        layers_max_scroll,
    }
}
