//! `impl Document` mutators + queries. Split from `document.rs`
//! for file-size hygiene — methods are reached via `doc.method()`.

use super::walkers::*;
use super::*;

/// Whether `reorder_relative` drops the source before or after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelativePosition {
    Before,
    After,
}

impl Document {
    /// Active theme driven by `ui.theme_mode`.
    pub fn theme(&self) -> crate::Theme {
        match self.ui.theme_mode {
            ThemeMode::Dark => crate::Theme::dark(),
            ThemeMode::Light => crate::Theme::light(),
        }
    }

    /// Translate `key` via active locale; falls through to `key`. `'static` to skip per-frame clones.
    pub fn t(&self, key: &'static str) -> &'static str {
        crate::i18n::translate(self.ui.locale, key)
    }

    /// Empty document with one default page; used by host smoke fixtures.
    pub fn empty() -> Self {
        Self {
            pages: vec![Page::new(1, "Page 1", Vec::new())],
            active_page_index: 0,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
            history: History::default(),
            var_table: VariableTable::default(),
            components: ComponentLibrary::default(),
        }
    }

    /// Sample document for the editor-UI demo. Stable ids: page=1,
    /// frame=10, title=11, button=12, button_rect=13, button_text=14.
    pub fn sample() -> Self {
        use crate::{Color, Rect};
        let title = Node::leaf(11, NodeKind::Text, "Title")
            .with_bounds(Rect::xywh(60.0, 60.0, 240.0, 28.0))
            .with_text("Hello OpenPencil");
        let button_rect = Node::leaf(13, NodeKind::Rect, "Button background")
            .with_bounds(Rect::xywh(60.0, 130.0, 180.0, 36.0))
            .with_fill(Color::BLUE);
        let button_text = Node::leaf(14, NodeKind::Text, "Click me")
            .with_bounds(Rect::xywh(76.0, 152.0, 160.0, 16.0))
            .with_text("Click me");
        let button = Node::with_children(
            12,
            NodeKind::Group,
            "Button",
            vec![button_rect, button_text],
        );
        let frame = Node::with_children(10, NodeKind::Frame, "Frame", vec![title, button])
            .with_bounds(Rect::xywh(40.0, 40.0, 360.0, 240.0))
            .with_fill(Color::WHITE)
            .with_stroke(Color::BLACK, 1.0);
        let doc = Self {
            pages: vec![Page::new(1, "Page 1", vec![frame])],
            active_page_index: 0,
            selected: NodeId::new(11), // "Title"
            selected_set: vec![NodeId::new(11)],
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
            history: History::default(),
            var_table: VariableTable::default(),
            components: ComponentLibrary::default(),
        };
        debug_assert!(
            doc.validate().is_ok(),
            "Document::sample() failed self-validation"
        );
        doc
    }

    /// The page currently shown in the editor viewport.
    pub fn active_page(&self) -> Option<&Page> {
        self.pages.get(self.active_page_index)
    }

    fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            pages: self.pages.clone(),
            active_page_index: self.active_page_index,
            selected: self.selected,
            selected_set: self.selected_set.clone(),
            var_table: self.var_table.clone(),
        }
    }

    fn restore(&mut self, snap: DocumentSnapshot) {
        self.pages = snap.pages;
        self.active_page_index = snap.active_page_index;
        self.selected = snap.selected;
        self.selected_set = snap.selected_set;
        self.var_table = snap.var_table;
    }

    /// Capture without pushing; use with `history_push_past`.
    pub fn snapshot_for_history(&self) -> DocumentSnapshot {
        self.snapshot()
    }

    /// Push snapshot to undo + clear redo. Cap = 100 (O(1) drop).
    pub fn history_push_past(&mut self, snap: DocumentSnapshot) {
        self.history.past.push_back(snap);
        if self.history.past.len() > 100 {
            self.history.past.pop_front();
        }
        self.history.future.clear();
    }

    /// Push current state to undo + clear redo. Call BEFORE a
    /// transactional change so undo reverts to here.
    pub fn commit_history(&mut self) {
        let snap = self.snapshot();
        self.history_push_past(snap);
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.history.past.pop_back() else {
            return false;
        };
        let cur = self.snapshot();
        self.history.future.push_back(cur);
        self.restore(prev);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.history.future.pop_back() else {
            return false;
        };
        let cur = self.snapshot();
        self.history.past.push_back(cur);
        self.restore(next);
        true
    }
    /// Get the anchor-selected node — last entry in `selected_set`.
    /// ONLY searches the active page; a selection on a non-active
    /// page returns `None`.
    pub fn selected_node(&self) -> Option<&Node> {
        if !self.selected.is_real() {
            return None;
        }
        self.active_page()?.find(self.selected)
    }

    /// True iff `id` is in the active selection set.
    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selected_set.contains(&id)
    }

    /// Number of nodes in the active selection set.
    pub fn selection_count(&self) -> usize {
        self.selected_set.len()
    }

    /// Replace selection with `id` + anchor on it. Idempotent.
    pub fn set_single_selection(&mut self, id: NodeId) {
        if id.is_real() {
            self.selected_set.clear();
            self.selected_set.push(id);
            self.selected = id;
        } else {
            self.clear_selection();
        }
        self.ui.align_toolbar_hover = None;
    }

    /// Shift-click semantics: if `id` is already in the set,
    /// remove it (and pick a new anchor); otherwise add it as
    /// the new anchor. TS parity: `toggleSelection(id)`.
    pub fn toggle_selection(&mut self, id: NodeId) {
        if !id.is_real() {
            return;
        }
        if let Some(pos) = self.selected_set.iter().position(|n| *n == id) {
            self.selected_set.remove(pos);
            self.selected = self.selected_set.last().copied().unwrap_or(NodeId::NONE);
        } else {
            self.selected_set.push(id);
            self.selected = id;
        }
        if self.selected_set.len() < 2 { self.ui.align_toolbar_hover = None; }
    }

    /// Clear both anchor + set. Idempotent.
    pub fn clear_selection(&mut self) {
        self.selected_set.clear();
        self.selected = NodeId::NONE;
        self.ui.align_toolbar_hover = None;
    }

    /// Whether `id` resolves to a node that can be mutated via
    /// selection-aware helpers (`translate_selected`,
    /// `set_selected_bounds`, etc.). Hidden + locked nodes are
    /// non-editable; everything else is. Mirrors TS
    /// `isNodeEditable(id)` from `document-store`.
    pub fn is_editable(&self, id: NodeId) -> bool {
        let Some(node) = self.active_page().and_then(|p| p.find(id)) else {
            return false;
        };
        !node.hidden && !node.locked
    }

    /// Stricter form of `is_editable` — every descendant must also
    /// be editable. Gates destructive ops so a locked/hidden child
    /// protects its ancestor from deletion.
    pub fn is_subtree_editable(&self, id: NodeId) -> bool {
        let Some(node) = self.active_page().and_then(|p| p.find(id)) else {
            return false;
        };
        subtree_all_editable(node)
    }

    /// Toggle the `hidden` flag on the node with this id. Returns
    /// true on success. Mirrors TS `useDocumentStore.toggleVisible`.
    pub fn toggle_node_hidden(&mut self, id: NodeId) -> bool {
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        toggle_hidden_walk(&mut page.children, id)
    }

    /// Toggle the `collapsed` flag on the node with this id —
    /// LayerPanel-only state, doesn't affect canvas paint or
    /// hit-test.
    pub fn toggle_node_collapsed(&mut self, id: NodeId) -> bool {
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        toggle_collapsed_walk(&mut page.children, id)
    }

    /// Toggle the `locked` flag on the node with this id.
    pub fn toggle_node_locked(&mut self, id: NodeId) -> bool {
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        toggle_locked_walk(&mut page.children, id)
    }

    /// Cmd+C: deep-clone selected nodes (ids preserved) into the
    /// clipboard. True iff anything was copied.
    pub fn copy_selected(&mut self) -> bool {
        if self.selected_set.is_empty() {
            return false;
        }
        let Some(page) = self.active_page() else {
            return false;
        };
        let mut buf: Vec<Node> = Vec::with_capacity(self.selected_set.len());
        for id in &self.selected_set {
            if let Some(node) = page.find(*id) {
                buf.push(node.clone());
            }
        }
        if buf.is_empty() {
            return false;
        }
        self.clipboard = buf;
        true
    }

    /// Copy the selection into the clipboard then delete it.
    /// Returns true when both steps succeeded. Mirrors TS `Cmd+X`.
    pub fn cut_selected(&mut self) -> bool {
        if !self.copy_selected() {
            return false;
        }
        self.delete_selected()
    }

    /// Paste clipboard nodes into the active page as top-level
    /// siblings offset by `offset_doc_px`. Mints fresh ids from
    /// `next_id`; replaces selection with the new ids. Returns
    /// the new ids in paste order, or empty on no-op (empty
    /// clipboard or id-allocator overflow). Mirrors TS `Cmd+V`.
    pub fn paste_clipboard(&mut self, next_id: &mut u64, offset_doc_px: f32) -> Vec<NodeId> {
        if self.clipboard.is_empty() {
            return Vec::new();
        }
        let Some(safe) = self.max_node_id().checked_add(1) else {
            return Vec::new();
        };
        *next_id = (*next_id).max(safe);
        // Verify total subtree headroom before any mint so a
        // partially-pasted document is impossible on overflow.
        let total: u64 = self.clipboard.iter().map(subtree_size).sum();
        if next_id.checked_add(total).is_none() {
            return Vec::new();
        }
        // Clone clipboard out so `pages.get_mut` doesn't alias
        // `self.clipboard`.
        let originals = self.clipboard.clone();
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return Vec::new();
        };
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(originals.len());
        for original in &originals {
            let mut clone = deep_clone_with_new_ids(original, next_id);
            shift_subtree(&mut clone, offset_doc_px, offset_doc_px);
            new_ids.push(clone.id);
            page.children.push(clone);
        }
        if !new_ids.is_empty() {
            self.selected = *new_ids.last().unwrap();
            self.selected_set = new_ids.clone();
        }
        new_ids
    }

    /// Top-level node ids on the active page whose aggregate
    /// bounds intersect `rect` (doc space). Used by the marquee
    /// rect-select on release. Mirrors TS
    /// `SpatialIndex::searchRect`. Descends only into top-level
    /// children — same as the click hit-test, so the result set
    /// can be selected as a unit.
    pub fn nodes_intersecting_doc_rect(&self, rect: crate::Rect) -> Vec<NodeId> {
        let Some(page) = self.active_page() else {
            return Vec::new();
        };
        let nx = rect.origin.x.min(rect.origin.x + rect.size.x);
        let ny = rect.origin.y.min(rect.origin.y + rect.size.y);
        let nw = rect.size.x.abs();
        let nh = rect.size.y.abs();
        let mut out = Vec::new();
        for child in &page.children {
            let b = child.aggregate_bounds();
            if b.size.x <= 0.0 && b.size.y <= 0.0 {
                continue;
            }
            let bx = b.origin.x.min(b.origin.x + b.size.x);
            let by = b.origin.y.min(b.origin.y + b.size.y);
            let bw = b.size.x.abs();
            let bh = b.size.y.abs();
            // AABB intersection test.
            if bx + bw < nx || nx + nw < bx || by + bh < ny || ny + nh < by {
                continue;
            }
            out.push(child.id);
        }
        out
    }

    /// Right-rail visibility gate. Visible when at least one id
    /// in `selected_set` resolves on the active page. Shared
    /// source of truth for canvas_region math + panel build +
    /// commit-on-blur.
    pub fn property_panel_visible(&self) -> bool {
        // Single + multi treat 0x0 nodes identically: panel shows
        // as long as at least one id resolves on the active page.
        match self.selection_count() {
            0 => false,
            _ => self
                .active_page()
                .is_some_and(|p| self.selected_set.iter().any(|id| p.find(*id).is_some())),
        }
    }

    /// True when ANY widget occupies the right rail today —
    /// PropertyPanel (gated on selection) or VariablesPanel
    /// (gated on a non-empty var table). `canvas_region` uses
    /// this to size the canvas so it doesn't paint over the
    /// rail content (codex BLOCK: `no-selection VariablesPanel
    /// is painted under the canvas` — without this gate the
    /// canvas extended full-width when nothing was selected,
    /// hiding the Variables panel).
    pub fn right_rail_visible(&self) -> bool {
        self.property_panel_visible() || !self.var_table.variables.is_empty()
    }

    /// Union of `aggregate_bounds` across selected nodes on the
    /// active page. Backs the multi-select panel's X/Y/W/H.
    pub fn selection_bounds(&self) -> Option<crate::Rect> {
        union_aggregate_bounds(self.active_page()?, &self.selected_set)
    }

    /// Cmd/Ctrl+A — select every top-level node on the active
    /// page (TS parity with `getActivePageChildren(...).map(id)`).
    /// Anchor is the last node so subsequent edits read the
    /// top-of-stack as "primary".
    pub fn select_all_top_level(&mut self) -> bool {
        let Some(page) = self.active_page() else {
            return false;
        };
        if page.children.is_empty() {
            return false;
        }
        self.selected_set = page.children.iter().map(|n| n.id).collect();
        self.selected = self.selected_set.last().copied().unwrap_or(NodeId::NONE);
        true
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

    /// Topmost node id whose bounds contain `point` on the active
    /// page. Walks children in reverse z-order. None on dead space.
    pub fn node_at_doc_point(&self, point: crate::Point2D) -> Option<NodeId> {
        let zoom = self.viewport.zoom.max(0.0001);
        let page = self.active_page()?;
        for child in page.children.iter().rev() {
            if let Some(hit) = hit_test_walk(child, point, zoom) {
                return Some(hit);
            }
        }
        None
    }

    /// Overwrite the selected node's rotation (radians, clockwise).
    pub fn set_selected_rotation(&mut self, radians: f32) {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return;
        };
        for child in &mut page.children {
            if set_rotation_walk(child, sel, radians) {
                return;
            }
        }
    }

    /// Overwrite the selected leaf node's bounds. Container nodes
    /// (Group / unbounded Frame) no-op — child-derived bounds need
    /// per-child scaling (later milestone).
    pub fn set_selected_bounds(&mut self, bounds: crate::Rect) {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return;
        };
        for child in &mut page.children {
            if set_bounds_walk(child, sel, bounds) {
                return;
            }
        }
    }

    /// Translate every node in the selection set by `(dx, dy)`
    /// document px. Containers cascade; ancestor-descendant
    /// dedup so descendants aren't shifted twice.
    pub fn translate_selected(&mut self, dx: f32, dy: f32) {
        if self.selected_set.is_empty() {
            return;
        }
        // Filter out hidden + locked nodes — those aren't
        // mutable. Done up-front because the page borrow below
        // is mutable.
        let editable: Vec<NodeId> = self
            .selected_set
            .iter()
            .copied()
            .filter(|id| self.is_editable(*id))
            .collect();
        if editable.is_empty() {
            return;
        }
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return;
        };
        for target in &editable {
            // Skip if any ancestor (within the active page tree)
            // is also in the selection — that ancestor's cascade
            // already shifted this descendant.
            if !is_ancestor_in_set(&page.children, *target, &editable) {
                for child in page.children.iter_mut() {
                    if translate_walk(child, *target, dx, dy) {
                        break;
                    }
                }
            }
        }
    }

    /// Apply a parsed property edit to the selected node. Returns
    /// `true` on a real selection (callers clear input draft +
    /// focus on `true`); container nodes silently no-op since
    /// their bounds are child-derived.
    pub fn commit_property_edit(&mut self, focus: PropertyFocus, value: f32) -> bool {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return false;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        for child in &mut page.children {
            if commit_property_walk(child, sel, focus, value) {
                return true;
            }
        }
        false
    }

    /// Set the fill / stroke colour on the selected node. Used by
    /// the hex inputs in the property panel — split from
    /// `commit_property_edit` because Color isn't a single f32.
    /// Write the picker's fill-type choice to the selected
    /// node's `fill_type`. Editable-gated so locked / hidden
    /// nodes can't be mutated. Returns true when the edit
    /// lands. No-op when nothing is selected.
    pub fn set_selected_fill_type(&mut self, fill_type: FillType) -> bool {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return false;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        for child in &mut page.children {
            if set_fill_type_walk(child, sel, fill_type) {
                return true;
            }
        }
        false
    }

    pub fn set_selected_color(&mut self, is_fill: bool, color: crate::Color) -> bool {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return false;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        for child in &mut page.children {
            if set_color_walk(child, sel, is_fill, color) {
                return true;
            }
        }
        false
    }

    /// Remove every node in the selection set from its parent's
    /// children. Returns true on success (selection cleared
    /// after). No-op when nothing is selected.
    ///
    /// TS parity: `for id in selectedIds: removeNode(id)` from
    /// `use-edit-shortcuts.ts`. Used by Delete / Backspace.
    pub fn delete_selected(&mut self) -> bool {
        if self.selected_set.is_empty() {
            return false;
        }
        // Filter out hidden + locked nodes — those aren't
        // removable via the user-facing Delete shortcut. Use the
        // SUBTREE-editable gate so deleting an editable Frame
        // can't take a locked / hidden child down with it (codex
        // stop-hook BLOCK: "nested protected selections can
        // still be deleted via selected ancestor"). TS parity:
        // locked rows ignore Delete in `use-edit-shortcuts.ts`.
        let (deletable, kept): (Vec<NodeId>, Vec<NodeId>) = self
            .selected_set
            .iter()
            .copied()
            .partition(|id| self.is_subtree_editable(*id));
        if deletable.is_empty() {
            return false;
        }
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        let mut removed_any = false;
        for id in &deletable {
            if remove_from_children(&mut page.children, *id) {
                removed_any = true;
            }
        }
        if removed_any {
            // Anchor + set survive the protected ids (locked /
            // hidden nodes). If everything got deleted the set
            // collapses to empty.
            self.selected_set = kept;
            self.selected = self.selected_set.last().copied().unwrap_or(NodeId::NONE);
            true
        } else {
            false
        }
    }

    /// Clone every node in the selection set (deep-clone with
    /// fresh ids), insert each as the next sibling at
    /// `offset_doc_px` from the original, and replace the
    /// selection with the new ids. Returns the new anchor id
    /// (last clone) on success.
    ///
    /// TS parity: `selectedIds.map(duplicateNode)` from
    /// `use-clipboard-shortcuts.ts`. Used by Cmd/Ctrl+D.
    pub fn duplicate_selected(&mut self, next_id: &mut u64, offset_doc_px: f32) -> Option<NodeId> {
        if self.selected_set.is_empty() {
            return None;
        }
        // Lift allocator past every existing id. checked_add so
        // u64::MAX returns None cleanly (no overflow → collision).
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let targets: Vec<NodeId> = self.selected_set.clone();
        let page = self.pages.get_mut(self.active_page_index)?;
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(targets.len());
        for target in targets {
            if let Some(new_id) =
                duplicate_in_children(&mut page.children, target, next_id, offset_doc_px)
            {
                new_ids.push(new_id);
            }
        }
        if new_ids.is_empty() {
            return None;
        }
        self.selected = *new_ids.last().unwrap();
        self.selected_set = new_ids;
        Some(self.selected)
    }

    /// Largest `NodeId` (by raw value) anywhere in the document,
    /// across all pages. Used as a one-shot guard so the duplicate
    /// allocator can never collide with a real id.
    pub fn max_node_id(&self) -> u64 {
        let mut max = 0u64;
        for page in &self.pages {
            max = max.max(page.id.raw());
            for child in &page.children {
                max = max.max(max_id_walk(child));
            }
        }
        max
    }

    /// Bump the selected node up or down by one position in its
    /// parent's children vec, which changes its paint order
    /// (children paint earlier-to-later, so later index = on top).
    /// Returns true on success.
    ///
    /// TS parity: `reorderNode(id, 'up' | 'down')`. Bound to `]`
    /// (`Up` → towards front) and `[` (`Down` → towards back).
    pub fn reorder_selected(&mut self, direction: ReorderDirection) -> bool {
        if !self.selected.is_real() {
            return false;
        }
        let target = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        reorder_in_children(&mut page.children, target, direction)
    }

    /// Move `source` to be a sibling immediately before/after
    /// `anchor`. Cross-parent reparenting supported. No-ops on
    /// same id, missing node, locked/hidden source, or cycle.
    pub fn reorder_before(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, RelativePosition::Before)
    }

    pub fn reorder_after(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, RelativePosition::After)
    }

    /// Move `source` so it becomes the LAST child of `parent`.
    /// Same guards as `reorder_before/after`.
    pub fn reorder_into(&mut self, source: NodeId, parent: NodeId) -> bool {
        if source == parent || !source.is_real() || !parent.is_real() {
            return false;
        }
        if !self.is_subtree_editable(source) {
            return false;
        }
        let Some(page) = self.pages.get(self.active_page_index) else {
            return false;
        };
        let Some(source_ref) = page.find(source) else {
            return false;
        };
        if descendant_contains(source_ref, parent) || page.find(parent).is_none() {
            return false;
        }
        let page = self.pages.get_mut(self.active_page_index).unwrap();
        let Some(node) = extract_node(&mut page.children, source) else {
            return false;
        };
        append_into(&mut page.children, parent, node).is_ok()
    }

    fn reorder_relative(
        &mut self,
        source: NodeId,
        anchor: NodeId,
        position: RelativePosition,
    ) -> bool {
        if source == anchor || !source.is_real() || !anchor.is_real() {
            return false;
        }
        if !self.is_subtree_editable(source) {
            return false;
        }
        let Some(page) = self.pages.get(self.active_page_index) else {
            return false;
        };
        let Some(source_ref) = page.find(source) else {
            return false;
        };
        if descendant_contains(source_ref, anchor) {
            return false;
        }
        if !children_contain_descendant(&page.children, anchor) {
            return false;
        }
        let page = self.pages.get_mut(self.active_page_index).unwrap();
        let Some(node) = extract_node(&mut page.children, source) else {
            return false;
        };
        let r = match position {
            RelativePosition::Before => insert_before_in_children(&mut page.children, anchor, node),
            RelativePosition::After => insert_after_in_children(&mut page.children, anchor, node),
        };
        debug_assert!(r.is_ok(), "anchor pre-check should ensure insert");
        r.is_ok()
    }

    /// Clear the active selection. Distinct from
    /// `ui.property_focus` clear — Escape calls both. Alias for
    /// `clear_selection` kept for readability at call sites.
    pub fn deselect_all(&mut self) {
        self.clear_selection();
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

    /// Run light invariant checks; Err on first violation: empty
    /// `pages`, duplicate node id, or `active_page_index` out of
    /// range. The empty-pages check fires BEFORE the index check
    /// so a document with `pages: vec![]` + `active_page_index:
    /// 99` is consistently rejected.
    pub fn validate(&self) -> Result<(), String> {
        if self.pages.is_empty() {
            return Err("Document::pages is empty (use Document::empty() for the default single-page shape)".to_string());
        }
        if let Some(dup) = self.find_duplicate_id() {
            return Err(format!("duplicate NodeId: {:?}", dup));
        }
        if self.active_page_index >= self.pages.len() {
            return Err(format!(
                "active_page_index {} out of range (pages.len()={})",
                self.active_page_index,
                self.pages.len()
            ));
        }
        Ok(())
    }
}
