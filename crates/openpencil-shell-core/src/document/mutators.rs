//! `impl Document` mutators + queries.
//!
//! Lives in `document/mutators.rs` so the `document` module file
//! itself stays under the 800-line cap. The impl block is split
//! from `document.rs` purely for file-size hygiene — semantically
//! these methods belong to `Document` and are accessed via the
//! usual `doc.method(...)` paths.

use super::walkers::*;
use super::*;

/// Whether `reorder_relative` drops the source before or after the
/// anchor. Internal helper for `reorder_before` / `reorder_after`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelativePosition {
    Before,
    After,
}

impl Document {
    /// Active theme — driven by `ui.theme_mode`. Widgets call this
    /// instead of hardcoding `Theme::dark()` so the entire chrome
    /// flips together when the user clicks the TopBar Sun icon.
    pub fn theme(&self) -> crate::Theme {
        match self.ui.theme_mode {
            ThemeMode::Dark => crate::Theme::dark(),
            ThemeMode::Light => crate::Theme::light(),
        }
    }

    /// Translate a chrome string by key. Keys are stable English
    /// identifiers; values come from a per-locale table. Unknown
    /// keys fall through to the key itself so callers get a
    /// visible "missing translation" instead of an empty render.
    pub fn t<'a>(&self, key: &'a str) -> &'a str {
        crate::i18n::translate(self.ui.locale, key)
    }

    /// Empty document with one empty default page named "Page 1".
    /// Used by host smoke fixtures.
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
        }
    }

    /// Sample document for the Step 2/3 editor-UI demo: one page
    /// with a frame containing a title (text) + a button (group
    /// of rect + text). Driven by document data instead of
    /// hardcoded TreeWidget items. Selection is set to the title
    /// so PropertyPanel has something to render. Step 3 adds
    /// concrete geometry + fills + strokes + text content so the
    /// CanvasViewport can actually render a recognisable mock.
    pub fn sample() -> Self {
        use crate::{Color, Rect};

        // Id allocations: page=1, frame=10, title=11, button=12,
        // button_rect=13, button_text=14. Stable across runs so
        // tests can assert specific ids.
        //
        // Layout (document coordinates, top-left origin):
        //   Frame    (40, 40)–(360, 240)   white fill, black 1px stroke
        //     Title  (60, 60)–(*, *)       text "Hello OpenPencil", no bg
        //     Button group at (60, 130)
        //       Rect   (60, 130)–(180, 36) blue fill, no stroke
        //       Text   (76, 152)–(*, *)    text "Click me", no bg
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

    /// Append a fresh empty page and switch to it. The page's id
    /// is minted past `max_node_id() + 1` so it can't collide with
    /// any existing node id; the name follows the `"Page N"` pattern
    /// (where N = pages.len() + 1 BEFORE the insert) to match the
    /// existing default-page-name convention. The new selection is
    /// cleared since the freshly-added page has no children.
    ///
    /// Returns the new page's index, or `None` when id allocation
    /// would overflow `u64::MAX`. Mirrors TS `addPage()` (the `+`
    /// button on the LayerPanel Pages header).
    pub fn add_page(&mut self) -> Option<usize> {
        let next_id = self.max_node_id().checked_add(1)?;
        let n = self.pages.len() + 1;
        let page = Page::new(next_id, format!("Page {}", n), Vec::new());
        self.pages.push(page);
        let new_index = self.pages.len() - 1;
        self.active_page_index = new_index;
        self.clear_selection();
        Some(new_index)
    }

    /// Get the anchor-selected node (TS `selectedIds[0]`). ONLY
    /// searches the active page (codex Step 2 R1 CONCERN-1). A
    /// selection on a non-active page returns `None`.
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

    /// Replace the selection with a single node + anchor on it.
    /// TS parity: `setSelection([id], id)`. Idempotent.
    pub fn set_single_selection(&mut self, id: NodeId) {
        if id.is_real() {
            self.selected_set.clear();
            self.selected_set.push(id);
            self.selected = id;
        } else {
            self.clear_selection();
        }
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
            // Anchor needs a new home. Last entry (most-recently
            // added survivor) is the natural choice.
            self.selected = self.selected_set.last().copied().unwrap_or(NodeId::NONE);
        } else {
            self.selected_set.push(id);
            self.selected = id;
        }
    }

    /// Clear both anchor + set. Idempotent.
    pub fn clear_selection(&mut self) {
        self.selected_set.clear();
        self.selected = NodeId::NONE;
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

    /// Whether `id` AND every descendant are editable — stricter
    /// gate than `is_editable`, used by destructive ops
    /// (`delete_selected`) so deleting an editable Frame can't
    /// wipe a locked / hidden child along with it. A locked or
    /// hidden node anywhere in the subtree protects the
    /// ancestor.
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

    /// Copy every node in the selection set into the clipboard
    /// (deep clones, original ids preserved). Returns true when
    /// at least one node was copied; false when nothing was
    /// selected. Mirrors TS `Cmd+C`.
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

    /// Paste every clipboard node into the active page as a
    /// top-level sibling, offset by `(offset_doc_px, offset_doc_px)`,
    /// minting fresh ids from `next_id`. Replaces selection with
    /// the new ids. Returns the new ids in paste order, or empty
    /// when nothing was pasted (empty clipboard or id-allocator
    /// overflow). Mirrors TS `Cmd+V`.
    ///
    /// Anchor-aware insertion (paste-inside-container,
    /// paste-as-sibling) is the TS polish; v1 always pastes at
    /// the top level — matches TS's fallback path when no anchor
    /// is selected.
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

    /// Whether the right-rail property panel should currently
    /// paint. Single source of truth so the host's
    /// `canvas_region` math, the panel's `for_selection_at`
    /// gate, and `apply_press` commit-on-blur all stay in lock-
    /// step. Today: single-select with a resolvable anchor.
    /// Multi-select hides pending an aggregated-properties UI;
    /// stale single anchors (e.g. selection points at a node on
    /// a non-active page, or an id that's been removed) hide too
    /// since the panel itself returns `None` from
    /// `for_selection_at` in that case (codex stop-hook fix:
    /// reserving the rail when the panel won't paint left a
    /// blank strip).
    pub fn property_panel_visible(&self) -> bool {
        self.selection_count() == 1 && self.selected_node().is_some()
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

    /// Hit-test the active page at a document-space point. Returns
    /// the topmost node id whose bounds (or aggregate bounds for
    /// containers) contain `point`. Walks children in reverse z-
    /// order (last child = top-most) so a stack of overlapping
    /// rects resolves to the visually topmost one. `None` if the
    /// click is in canvas dead space or no active page exists.
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

    /// Overwrite the selected leaf node's bounds. Only updates
    /// nodes that carry their own bounds (size > 0); container
    /// nodes (Group / unbounded Frame) are skipped — their
    /// "bounds" are derived from children and resizing them needs
    /// per-child scaling which lands in a later milestone.
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
    /// document px. Container nodes cascade to descendants
    /// (`translate_walk`'s subtree translate). When two selected
    /// nodes have an ancestor-descendant relationship, only the
    /// ancestor is translated so the descendant isn't shifted
    /// twice (TS parity with the dedup in `use-edit-shortcuts.ts`
    /// nudge handler). No-op when nothing is selected or the
    /// active page is missing.
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

    /// Apply a parsed property edit to the selected node. Mirrors
    /// the TS `useDocumentStore` mutation handlers — only this
    /// helper writes back to bounds, so call sites can stay
    /// declarative ("commit X = 120" rather than "find the node,
    /// clone bounds, mutate one axis, write back").
    ///
    /// Returns `true` if the edit landed on a real node; `false`
    /// when there's no selection or the active page can't be
    /// found. Container nodes (Group / unbounded Frame) currently
    /// no-op — their bounds are derived from children — but the
    /// API still returns `true` because the host should still
    /// clear the input draft + focus.
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
        // Lift the allocator past every existing id so we never
        // mint a duplicate even when the document was loaded
        // with ids greater than the host's running counter (codex
        // CONCERN: external docs with ids ≥ next_id would
        // otherwise silently collide).
        //
        // `checked_add(1)` instead of `saturating_add` so a
        // document carrying `NodeId(u64::MAX)` returns None
        // cleanly instead of saturating to u64::MAX and minting
        // a collision (the saturating overflow lane was a
        // theoretical edge but worth being explicit about).
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

    /// Move `source` immediately before `anchor` in the document
    /// tree. Supports cross-parent reparenting. Backs LayerPanel
    /// drag-to-reorder. No-ops on: same id, missing node, locked
    /// or hidden source, or cycle (anchor inside source's subtree).
    pub fn reorder_before(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, RelativePosition::Before)
    }

    /// Same as `reorder_before`, but drops `source` immediately
    /// after `anchor`.
    pub fn reorder_after(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, RelativePosition::After)
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
        let insert_result = match position {
            RelativePosition::Before => insert_before_in_children(&mut page.children, anchor, node),
            RelativePosition::After => insert_after_in_children(&mut page.children, anchor, node),
        };
        debug_assert!(
            insert_result.is_ok(),
            "anchor pre-check should ensure insert"
        );
        insert_result.is_ok()
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

    /// Run light invariant checks on the document. Returns `Err`
    /// with a human-readable message on the first violation:
    /// - `pages` is empty (a document must have at least one
    ///   page; use `Document::empty()` to construct a default
    ///   single-page document)
    /// - duplicate node id anywhere in any page
    /// - `active_page_index` out of range
    ///
    /// Codex Step 2 R2 CONCERN-1: the prior version skipped the
    /// `active_page_index` check when `pages.is_empty()`, leaving
    /// a (Document { pages: vec![], active_page_index: 99, … })
    /// silently valid. Empty pages is itself an invariant
    /// violation; this version rejects it explicitly so the
    /// active_page_index check applies unconditionally.
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
