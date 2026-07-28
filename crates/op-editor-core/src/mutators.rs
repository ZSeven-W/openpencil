//! `impl EditorState` editor mutators — ported from
//! `openpencil-shell-core::document`'s `impl Document`, retargeted
//! onto the canonical `jian_ops_schema::PenDocument`.
//!
//! ## Page model
//!
//! `PenDocument` carries `pages: Option<Vec<PenPage>>` plus a root
//! `children: Vec<PenNode>`. When `pages` is `Some`, the editor
//! works on the active page's `children`; when `pages` is `None`,
//! the document is single-page and the editor works on the root
//! `children` directly. [`EditorState::active_children`] /
//! [`EditorState::active_children_mut`] hide that fork so the
//! mutators stay page-model-agnostic.

use crate::geometry::{union_aggregate_bounds, DocRect};
use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::selection::SelectionState;
use crate::state::EditorState;
use crate::walkers::{self, find_node, find_node_mut, reorder_in_children, ReorderDirection};
use jian_ops_schema::node::PenNode;
use std::collections::HashSet;

impl EditorState {
    // --- Active-page node access -------------------------------------

    /// The active page's children — `doc.pages[active]` when the
    /// document is multi-page, else the root `doc.children`. A
    /// `pages: Some([])` document (empty multi-page list) and an
    /// out-of-range `active_page_index` both fall back to
    /// `doc.children`, matching [`active_children_mut`] so an insert
    /// lands where the read side will find it.
    pub fn active_children(&self) -> &[PenNode] {
        let idx = self.ui.active_page_index;
        match self.doc.pages.as_ref() {
            Some(pages) if !pages.is_empty() => {
                let i = idx.min(pages.len() - 1);
                &pages[i].children
            }
            _ => &self.doc.children,
        }
    }

    /// Mutable form of [`EditorState::active_children`].
    ///
    /// A `pages: Some([])` document (empty multi-page list — legal
    /// on the wire even if the page-mutator layer normally
    /// normalizes it away) falls back to `doc.children` instead of
    /// panicking on an out-of-bounds index.
    pub fn active_children_mut(&mut self) -> &mut Vec<PenNode> {
        let idx = self.ui.active_page_index;
        match self.doc.pages.as_mut() {
            Some(pages) if !pages.is_empty() => {
                let i = idx.min(pages.len() - 1);
                &mut pages[i].children
            }
            _ => &mut self.doc.children,
        }
    }

    /// Number of pages — 1 when the document uses the single-page
    /// fallback (no `pages` list).
    pub fn page_count(&self) -> usize {
        self.doc.pages.as_ref().map(|p| p.len().max(1)).unwrap_or(1)
    }

    // --- Queries -----------------------------------------------------

    /// The anchor-selected node on the active page, or `None`.
    pub fn selected_node(&self) -> Option<&PenNode> {
        if !self.selection.anchor.is_real() {
            return None;
        }
        find_node(self.active_children(), &self.selection.anchor)
    }

    /// True when `id` is in the active selection set.
    pub fn is_selected(&self, id: &NodeId) -> bool {
        self.selection.contains(id)
    }

    /// Number of nodes in the active selection set.
    pub fn selection_count(&self) -> usize {
        self.selection.len()
    }

    /// Whether `id` resolves to an editable node on the active page.
    /// Hidden (`visible == Some(false)`) + locked nodes are not
    /// editable; everything else is.
    pub fn is_editable(&self, id: &NodeId) -> bool {
        if self.instance_write_virtual_anchor.as_ref() == Some(id) {
            return true;
        }
        if let Some(node) = find_node(self.active_children(), id) {
            return node_editable(node);
        }
        let Some((ref_id, _child_id)) =
            crate::instance_override::split_instance_child_anchor(id, &self.doc)
        else {
            return false;
        };
        find_node(self.active_children(), &ref_id)
            .filter(|node| matches!(node, PenNode::Ref(_)))
            .is_some_and(node_editable)
    }

    /// Stricter form of [`EditorState::is_editable`] — every
    /// descendant must also be editable. Gates destructive ops so a
    /// locked / hidden child protects its ancestor.
    pub fn is_subtree_editable(&self, id: &NodeId) -> bool {
        let Some(node) = find_node(self.active_children(), id) else {
            return false;
        };
        subtree_all_editable(node)
    }

    /// Largest editor-minted `n{N}` id suffix anywhere in the
    /// document (0 when no `n{N}` id exists). Seeds the new-node-id
    /// allocator.
    pub fn max_node_id(&self) -> u64 {
        let mut max = 0u64;
        if let Some(pages) = self.doc.pages.as_ref() {
            for page in pages {
                max = max.max(walkers::parse_n_id(&page.id).unwrap_or(0));
                for child in &page.children {
                    max = max.max(walkers::max_id_walk(child));
                }
            }
        }
        for child in &self.doc.children {
            max = max.max(walkers::max_id_walk(child));
        }
        max
    }

    /// Every node + page id live in the document. Backs the
    /// allocator's collision check.
    pub fn collect_node_ids(&self) -> HashSet<NodeId> {
        let mut out = HashSet::new();
        if let Some(pages) = self.doc.pages.as_ref() {
            for page in pages {
                if let Some(id) = NodeId::new_opt(page.id.as_str()) {
                    out.insert(id);
                }
                walkers::collect_ids(&page.children, &mut out);
            }
        }
        walkers::collect_ids(&self.doc.children, &mut out);
        out
    }

    /// Right-rail PropertyPanel visibility gate. Design and Interact require
    /// at least one live selected node. Code remains selection-independent:
    /// it falls back to the active page's children.
    pub fn property_panel_visible(&self) -> bool {
        if self.editor_ui.property_tab == crate::PropertyTab::Code {
            return true;
        }
        if self.selection.set.is_empty() {
            return false;
        }
        let children = self.active_children();
        self.selection.set.iter().any(|id| {
            find_node(children, id).is_some()
                || crate::instance_override::split_instance_child_anchor(id, &self.doc).is_some()
        })
    }

    /// True when a selection inspector or selection-independent Code panel
    /// occupies the right rail.
    pub fn right_rail_visible(&self) -> bool {
        self.property_panel_visible()
    }

    /// Union of `aggregate_bounds` across the selected nodes.
    pub fn selection_bounds(&self) -> Option<DocRect> {
        union_aggregate_bounds(self.active_children(), &self.selection.set)
    }

    /// First duplicate id found in the document, or `None`.
    pub fn find_duplicate_id(&self) -> Option<NodeId> {
        let mut seen: HashSet<String> = HashSet::new();
        if let Some(pages) = self.doc.pages.as_ref() {
            for page in pages {
                if !seen.insert(page.id.clone()) {
                    return NodeId::new_opt(page.id.as_str());
                }
                if let Some(dup) = walkers::find_duplicate(&page.children, &mut seen) {
                    return Some(dup);
                }
            }
        }
        walkers::find_duplicate(&self.doc.children, &mut seen)
    }

    // --- Selection ---------------------------------------------------

    /// Replace the selection with `id` + anchor on it. A NONE id
    /// clears the selection. Idempotent.
    pub fn set_single_selection(&mut self, id: NodeId) {
        let next = if id.is_real() {
            SelectionState {
                anchor: id.clone(),
                set: vec![id],
            }
        } else {
            SelectionState::empty()
        };
        if self.selection != next {
            self.editor_ui.image_panel.close_popovers();
            self.editor_ui.image_crop_editing = None;
            self.selection = next;
        }
    }

    /// Shift-click semantics: toggle `id` in / out of the set,
    /// keeping the anchor invariant (anchor == last entry).
    pub fn toggle_selection(&mut self, id: NodeId) {
        if !id.is_real() {
            return;
        }
        self.editor_ui.image_panel.close_popovers();
        self.editor_ui.image_crop_editing = None;
        if let Some(pos) = self.selection.set.iter().position(|n| *n == id) {
            self.selection.set.remove(pos);
            self.selection.anchor = self.selection.set.last().cloned().unwrap_or(NodeId::NONE);
        } else {
            self.selection.anchor = id.clone();
            self.selection.set.push(id);
        }
    }

    /// Clear both anchor + set. Idempotent.
    pub fn clear_selection(&mut self) {
        if !self.selection.is_empty() {
            self.editor_ui.image_panel.close_popovers();
        }
        self.editor_ui.image_crop_editing = None;
        self.selection = SelectionState::empty();
    }

    /// Alias for [`EditorState::clear_selection`] — Escape's last
    /// tier in the editor host.
    pub fn deselect_all(&mut self) {
        self.clear_selection();
    }

    /// Select every top-level node on the active page. Anchor is the
    /// last node. False when the page has no children.
    pub fn select_all_top_level(&mut self) -> bool {
        let ids: Vec<NodeId> = self
            .active_children()
            .iter()
            .filter_map(|n| NodeId::new_opt(n.id_str()))
            .collect();
        if ids.is_empty() {
            return false;
        }
        let next_anchor = ids.last().cloned().unwrap_or(NodeId::NONE);
        if self.selection.anchor != next_anchor || self.selection.set != ids {
            self.editor_ui.image_panel.close_popovers();
        }
        self.selection.anchor = next_anchor;
        self.selection.set = ids;
        true
    }

    // --- Node flag toggles -------------------------------------------

    /// Toggle the `visible` flag on the node. True on success.
    /// `visible == None` is treated as visible, so the first toggle
    /// hides it.
    pub fn toggle_node_hidden(&mut self, id: &NodeId) -> bool {
        let Some(node) = find_node_mut(self.active_children_mut(), id) else {
            return false;
        };
        let base = node.base_mut();
        let now_visible = base.visible.unwrap_or(true);
        base.visible = Some(!now_visible);
        true
    }

    /// Toggle the `locked` flag on the node. True on success.
    pub fn toggle_node_locked(&mut self, id: &NodeId) -> bool {
        let Some(node) = find_node_mut(self.active_children_mut(), id) else {
            return false;
        };
        let base = node.base_mut();
        base.locked = Some(!base.locked.unwrap_or(false));
        true
    }

    /// Toggle the LayerPanel collapse flag for `id`. Collapse is
    /// editor-only UI state — the canonical `PenNodeBase` has no
    /// `collapsed` field, so it lives in
    /// `EditorState.editor_ui.collapsed_layers` rather than on the
    /// node. `true` is returned when `id` is collapsed AFTER the call
    /// (it was just collapsed); `false` when it is now expanded. A
    /// `NONE` id is a no-op returning `false`.
    pub fn toggle_node_collapsed(&mut self, id: &NodeId) -> bool {
        if !id.is_real() {
            return false;
        }
        if self.editor_ui.collapsed_layers.contains(id) {
            self.editor_ui.collapsed_layers.remove(id);
            false
        } else {
            self.editor_ui.collapsed_layers.insert(id.clone());
            true
        }
    }

    /// Whether `id`'s children are collapsed in the LayerPanel. Reads
    /// the editor-only `collapsed_layers` set — see
    /// [`EditorState::toggle_node_collapsed`].
    pub fn is_node_collapsed(&self, id: &NodeId) -> bool {
        self.editor_ui.collapsed_layers.contains(id)
    }

    // --- Geometry ----------------------------------------------------

    /// Overwrite the anchor node's rotation (radians, clockwise).
    /// The canonical schema stores rotation in degrees, so this
    /// converts on write. No-op on a locked / hidden / missing node.
    pub fn set_selected_rotation(&mut self, radians: f32) {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return;
        }
        if let Some(node) = find_node_mut(self.active_children_mut(), &sel) {
            node.base_mut().rotation = Some((radians as f64).to_degrees());
            self.mark_document_changed();
        }
    }

    // Selection-handle resize lives in `drag_mutators.rs` — split out to
    // keep this file under the 800-line ceiling. It changes only the selected
    // node; descendants adapt through normal layout resolution.

    /// Translate every node in the selection set by `(dx, dy)` doc
    /// px. Containers carry their subtree (child coords are parent-
    /// relative); an ancestor-already-in-set dedup stops descendants
    /// shifting twice. A child of an auto-layout (flex) parent is
    /// engine-positioned; materializing x/y here flips it to
    /// `Position::Absolute` in jian-core and detaches it from flex
    /// flow, so such a node is skipped (a free drag of it is a no-op).
    ///
    /// Single recursive walk over `active_children` (Task 10):
    /// previously this ran three full-tree walks (`is_flow_child_of_flex`
    /// / `is_ancestor_in_set` / `find_node_mut`) PER selected id —
    /// O(|selection| × nodes), near-quadratic on select-all. The editable
    /// set is collected once into a `HashSet<&str>` and
    /// [`walkers::translate_editable_subtree`] threads both skip
    /// conditions down the recursion as it descends, so the whole tree
    /// is visited exactly once regardless of selection size.
    pub fn translate_selected(&mut self, dx: f64, dy: f64) -> bool {
        if self.selection.set.is_empty() || (dx == 0.0 && dy == 0.0) {
            return false;
        }
        // Owned, not borrowed from `self.selection` — the `HashSet<&str>`
        // built below borrows these local `NodeId`s instead, so it can
        // coexist with the `&mut self` borrow `active_children_mut()`
        // needs.
        let editable: Vec<NodeId> = self
            .selection
            .set
            .iter()
            .filter(|id| self.is_editable(id))
            .cloned()
            .collect();
        if editable.is_empty() {
            return false;
        }
        let editable_set: HashSet<&str> = editable.iter().map(NodeId::as_str).collect();
        let children = self.active_children_mut();
        let translated =
            walkers::translate_editable_subtree(children, &editable_set, dx, dy, false, false);
        if translated {
            self.mark_document_changed();
        }
        translated
    }

    // --- Tree ops ----------------------------------------------------

    /// Remove every editable node in the selection set from its
    /// parent. Locked / hidden subtrees are protected. True on
    /// success; selection collapses to the kept (protected) ids.
    pub fn delete_selected(&mut self) -> bool {
        if self.selection.set.is_empty() {
            return false;
        }
        let (deletable, kept): (Vec<NodeId>, Vec<NodeId>) = self
            .selection
            .set
            .iter()
            .cloned()
            .partition(|id| self.is_subtree_editable(id));
        if deletable.is_empty() {
            return false;
        }
        let children = self.active_children_mut();
        let mut removed_any = false;
        for id in &deletable {
            if walkers::remove_from_children(children, id) {
                removed_any = true;
            }
        }
        if removed_any {
            self.selection.set = kept;
            self.selection.anchor = self.selection.set.last().cloned().unwrap_or(NodeId::NONE);
            true
        } else {
            false
        }
    }

    /// Deep-clone every selected node with fresh ids, inserting each
    /// clone as the next sibling offset by `offset_doc_px`. Returns
    /// the new anchor id (last clone) on success.
    pub fn duplicate_selected(&mut self, next_id: &mut u64, offset_doc_px: f64) -> Option<NodeId> {
        let mut allocator = SequentialIdAllocator::for_document(&self.doc, *next_id).ok()?;
        let result = self
            .duplicate_selected_with_allocator(&mut allocator, offset_doc_px)
            .ok()
            .flatten();
        if result.is_some() {
            *next_id = allocator.next_counter();
        }
        result
    }

    /// Allocator-aware form of [`Self::duplicate_selected`].
    pub fn duplicate_selected_with_allocator(
        &mut self,
        allocator: &mut dyn IdAllocator,
        offset_doc_px: f64,
    ) -> Result<Option<NodeId>, IdAllocError> {
        if self.selection.set.is_empty() {
            return Ok(None);
        }
        let mut taken = self.collect_node_ids();
        let targets = self.selection.set.clone();
        let mut staged_children = self.active_children().to_vec();
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(targets.len());
        for target in &targets {
            if let Some(new_id) = duplicate_in_children_with_allocator(
                &mut staged_children,
                target,
                allocator,
                &mut taken,
                offset_doc_px,
            )? {
                new_ids.push(new_id);
            }
        }
        if new_ids.is_empty() {
            return Ok(None);
        }
        *self.active_children_mut() = staged_children;
        self.selection.anchor = new_ids.last().cloned().unwrap();
        self.selection.set = new_ids;
        Ok(Some(self.selection.anchor.clone()))
    }

    /// Bump the anchor node up / down one position among its
    /// siblings. True on success.
    pub fn reorder_selected(&mut self, direction: ReorderDirection) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() {
            return false;
        }
        reorder_in_children(self.active_children_mut(), &sel, direction)
    }

    /// Move `source` to be a sibling immediately before `anchor`.
    /// Cross-parent reparenting supported. Cycle / lock / missing
    /// guarded.
    pub fn reorder_before(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, true)
    }

    /// Move `source` to be a sibling immediately after `anchor`.
    pub fn reorder_after(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, false)
    }

    /// Move `source` so it becomes the FIRST child of `parent` (TS parity:
    /// a layer-panel drop into a container inserts at index 0, the top of
    /// the child list — `layer-dnd-utils.ts`).
    pub fn reorder_into(&mut self, source: NodeId, parent: NodeId) -> bool {
        if source == parent || !source.is_real() || !parent.is_real() {
            return false;
        }
        if !self.is_subtree_editable(&source) {
            return false;
        }
        let children = self.active_children();
        let Some(source_ref) = find_node(children, &source) else {
            return false;
        };
        // The target must be a container that accepts children, verified
        // BEFORE extracting the source. Otherwise a non-container target would
        // make `prepend_into` bounce the payload (`Err`) AFTER the node was
        // already removed, silently dropping it from the document.
        let parent_accepts = find_node(children, &parent).is_some_and(PenNodeExt::is_container);
        if walkers::descendant_contains(source_ref, &parent) || !parent_accepts {
            return false;
        }
        let children = self.active_children_mut();
        let Some(node) = walkers::extract_node(children, &source) else {
            return false;
        };
        walkers::prepend_into(children, &parent, node).is_ok()
    }

    fn reorder_relative(&mut self, source: NodeId, anchor: NodeId, before: bool) -> bool {
        if source == anchor || !source.is_real() || !anchor.is_real() {
            return false;
        }
        if !self.is_subtree_editable(&source) {
            return false;
        }
        let children = self.active_children();
        let Some(source_ref) = find_node(children, &source) else {
            return false;
        };
        if walkers::descendant_contains(source_ref, &anchor) {
            return false;
        }
        if !walkers::contains_node(children, &anchor) {
            return false;
        }
        let children = self.active_children_mut();
        let Some(node) = walkers::extract_node(children, &source) else {
            return false;
        };
        let r = if before {
            walkers::insert_before_in_children(children, &anchor, node)
        } else {
            walkers::insert_after_in_children(children, &anchor, node)
        };
        r.is_ok()
    }

    /// Select the chat model at `idx` in `chat.available_models` and
    /// close the picker. Also re-syncs `editor_ui.chat_selected_agent`
    /// to the model's provider so the desktop chat transport targets
    /// the matching CLI. A bad index is ignored (the picker still
    /// closes). Faithful port of shell-core's
    /// `Document::select_chat_model`.
    pub fn select_chat_model(&mut self, idx: usize) {
        // Extract everything we need from the borrowed entry before
        // releasing it, then apply mutations — required because
        // `self.chat` is a `ChatSessions` Deref wrapper and the borrow
        // checker cannot reason about field-level disjointness through
        // the Deref impl.
        let update = self.chat.available_models.get(idx).map(|entry| {
            (
                entry.provider,
                entry.builtin_provider_id.is_none() && entry.acp_agent_id().is_none(),
            )
        });
        if let Some((provider, use_native_agent)) = update {
            self.chat.selected_model = idx;
            if use_native_agent {
                if let Some(pidx) = crate::AgentProvider::ALL
                    .iter()
                    .position(|p| *p == provider)
                {
                    self.editor_ui.chat_selected_agent = pidx;
                }
            }
        }
        self.editor_ui.close_chat_model_picker();
    }

    /// Recompute the chat model-picker's `available_models` from the
    /// discovered catalog filtered by the providers connected in
    /// Settings → Agents. Thin wrapper over
    /// [`ChatState::rebuild_available_models`] that reads the
    /// connected mask off `editor_ui.agent_settings`. Hosts call this
    /// after discovery finishes and after every connect toggle.
    pub fn rebuild_chat_models(&mut self) {
        let prev = self
            .chat
            .available_models
            .get(self.chat.selected_model)
            .cloned();
        let connected = self.editor_ui.agent_settings.verified_connected_mask();
        self.chat.rebuild_available_models(&connected);
        self.chat.available_models.extend(
            self.editor_ui
                .agent_settings
                .builtin_agents
                .iter()
                .filter(|agent| agent.ready())
                .map(|agent| {
                    crate::ModelEntry::builtin_with_display_name(
                        agent.kind.model_provider(),
                        agent.id.clone(),
                        agent.display_name.clone(),
                        format!("builtin:{}:{}", agent.id, agent.model),
                        agent.model.clone(),
                    )
                }),
        );
        let agent_settings = &self.editor_ui.agent_settings;
        self.chat.available_models.extend(
            agent_settings
                .acp_agents
                .iter()
                .filter(|agent| {
                    agent.ready() && agent_settings.acp_agent_verified_connected(&agent.id)
                })
                .map(|agent| crate::ModelEntry::acp(agent.id.clone(), agent.display_name.clone())),
        );
        if let Some(prev) = prev {
            if let Some(idx) = self.chat.available_models.iter().position(|entry| {
                entry.provider == prev.provider
                    && entry.value == prev.value
                    && entry.builtin_provider_id == prev.builtin_provider_id
            }) {
                self.chat.selected_model = idx;
            }
        }
        if let Some(entry) = self.chat.selected_model_entry() {
            if entry.builtin_provider_id.is_none() && entry.acp_agent_id().is_none() {
                if let Some(pidx) = crate::AgentProvider::ALL
                    .iter()
                    .position(|p| *p == entry.provider)
                {
                    self.editor_ui.chat_selected_agent = pidx;
                }
            }
        }
    }

    /// Cycle the active tab's ⚡Nx (`chat.agent_team_size`) AND mirror the
    /// new value into `editor_ui.preferred_agent_team_size` — the sticky
    /// app preference `settings_io` persists and every future launch's
    /// tab 0 seeds from (see that field's doc comment). A thin
    /// `EditorState`-level wrapper over `ChatState::cycle_agent_team_size`
    /// so the two fields can never drift apart: every host (native + web)
    /// calls THIS, never the bare `chat` method, when the user changes
    /// the setting.
    pub fn cycle_agent_team_size(&mut self) {
        self.chat.cycle_agent_team_size();
        self.editor_ui.preferred_agent_team_size = self.chat.agent_team_size;
    }

    /// Set the active tab's ⚡Nx to an explicit value (the Parallel Agents
    /// picker's direct choice) and mirror it into `editor_ui.preferred_
    /// agent_team_size`, same as [`Self::cycle_agent_team_size`].
    pub fn set_agent_team_size(&mut self, n: u32) {
        self.chat.agent_team_size = n;
        self.editor_ui.preferred_agent_team_size = n;
    }

    /// Light invariant check — Err on first violation: out-of-range
    /// active page index, or a duplicate node id.
    pub fn validate(&self) -> Result<(), EditorStateInvariant> {
        if let Some(dup) = self.find_duplicate_id() {
            return Err(EditorStateInvariant::DuplicateNodeId(dup));
        }
        if self.ui.active_page_index >= self.page_count() {
            return Err(EditorStateInvariant::ActivePageIndexOutOfRange {
                index: self.ui.active_page_index,
                page_count: self.page_count(),
            });
        }
        Ok(())
    }
}

/// A violated [`EditorState`] invariant reported by
/// [`EditorState::validate`]. `Display` reproduces the previous ad-hoc
/// `String` messages byte for byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorStateInvariant {
    /// The same node id appears more than once in the tree.
    DuplicateNodeId(NodeId),
    /// `ui.active_page_index` points past the last page.
    ActivePageIndexOutOfRange { index: usize, page_count: usize },
}

impl std::fmt::Display for EditorStateInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorStateInvariant::DuplicateNodeId(dup) => write!(f, "duplicate NodeId: {dup:?}"),
            EditorStateInvariant::ActivePageIndexOutOfRange { index, page_count } => write!(
                f,
                "active_page_index {index} out of range (page_count={page_count})"
            ),
        }
    }
}

impl std::error::Error for EditorStateInvariant {}

impl From<EditorStateInvariant> for String {
    fn from(error: EditorStateInvariant) -> String {
        error.to_string()
    }
}

// --- Free helpers ----------------------------------------------------

/// True when a node is editable — not hidden, not locked.
fn node_editable(node: &PenNode) -> bool {
    let base = node.base();
    base.visible.unwrap_or(true) && !base.locked.unwrap_or(false)
}

/// True when `node` and every descendant are editable.
fn subtree_all_editable(node: &PenNode) -> bool {
    if !node_editable(node) {
        return false;
    }
    match node.children() {
        Some(children) => children.iter().all(subtree_all_editable),
        None => true,
    }
}

/// Recursive helper for `duplicate_selected` — finds the target,
/// deep-clones it with fresh ids, offsets the clone, and inserts it
/// as the next sibling. Returns the clone's id. Also the paste path
/// for reusable components (`clipboard.rs`), which mirrors TS
/// `duplicateNode`: the minted instance lands next to the live
/// component, not at the paste anchor.
pub(crate) fn duplicate_in_children_with_allocator(
    children: &mut Vec<PenNode>,
    target: &NodeId,
    allocator: &mut dyn IdAllocator,
    taken: &mut HashSet<NodeId>,
    offset: f64,
) -> Result<Option<NodeId>, IdAllocError> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == target.as_str()) {
        // TS parity: duplicating a reusable component mints a Ref
        // INSTANCE pointing at it, not a deep clone — the duplicate
        // tracks future component edits. Position offsets like a
        // clone; size/name/props inherit through the render-time
        // ref expansion.
        if let PenNode::Frame(frame) = &children[idx] {
            if frame.reusable == Some(true) {
                let new_id = allocator.allocate(taken)?;
                let x = frame.base.x.unwrap_or(0.0) + offset;
                let y = frame.base.y.unwrap_or(0.0) + offset;
                let instance: Option<PenNode> = serde_json::from_value(serde_json::json!({
                    "type": "ref",
                    "id": new_id.as_str(),
                    "ref": frame.base.id,
                    "x": x,
                    "y": y,
                }))
                .ok();
                let Some(instance) = instance else {
                    return Ok(None);
                };
                children.insert(idx + 1, instance);
                return Ok(Some(new_id));
            }
        }
        let mut clone = walkers::deep_clone_with_allocator(&children[idx], allocator, taken)?;
        walkers::translate_subtree(&mut clone, offset, offset);
        let Some(new_id) = NodeId::new_opt(clone.id_str()) else {
            return Ok(None);
        };
        children.insert(idx + 1, clone);
        return Ok(Some(new_id));
    }
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            if let Some(new_id) =
                duplicate_in_children_with_allocator(grand, target, allocator, taken, offset)?
            {
                return Ok(Some(new_id));
            }
        }
    }
    Ok(None)
}
