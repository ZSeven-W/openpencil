//! Host-facing `EditorState` helpers for the native editor host.
//!
//! The native widget host (`openpencil-shell-native`) drives
//! `EditorState` as its single source of truth. A handful of host
//! operations have no 1:1 mutator on `EditorState` yet — node spawn
//! for the active tool, a non-test sample document for the host
//! constructor, and committing a path-boolean result. These are
//! collected here as additive, tested `EditorState` methods so the
//! host never forks tree-mutation logic of its own.

use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::tool::Tool;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::PenNode;

impl EditorState {
    /// Build an editor state seeded with the demo sample document —
    /// a bounded Frame `n10` (`white` fill, 1-px black stroke)
    /// containing a Text `n11` and a Group `n12` (`n13` blue rect +
    /// `n14` text). Selection anchors on `n11`.
    ///
    /// This is the widget-test fixture. The native host opens with
    /// [`EditorState::starter`] (a single empty Frame) instead.
    pub fn sample() -> Self {
        let src = r##"{
            "version": "__OPENPENCIL_VERSION__",
            "children": [
              {"type":"frame","id":"n10","name":"Frame",
               "x":40,"y":40,"width":360,"height":240,
               "fill":[{"type":"solid","color":"#FFFFFF"}],
               "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#000000"}]},
               "children":[
                 {"type":"text","id":"n11","name":"Title",
                  "x":60,"y":60,"width":240,"height":28,"content":"Hello OpenPencil"},
                 {"type":"group","id":"n12","name":"Button",
                  "x":60,"y":130,"width":180,"height":36,
                  "children":[
                    {"type":"rectangle","id":"n13","name":"Button background",
                     "x":60,"y":130,"width":180,"height":36,
                     "fill":[{"type":"solid","color":"#2563EB"}]},
                    {"type":"text","id":"n14","name":"Click me",
                     "x":76,"y":152,"width":160,"height":16,"content":"Click me"}
                  ]}
               ]}
            ]
        }"##;
        let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));
        let doc = jian_ops_schema::load_str(&src)
            .expect("EditorState::sample() fixture parses")
            .value;
        let mut state = Self::from_document(doc);
        state.set_single_selection(NodeId::new("n11"));
        state
    }

    /// Build the document a fresh launch opens with — a single empty
    /// starter Frame `n10` matching the TypeScript app's blank
    /// document geometry. No default selection and no demo decoration.
    pub fn starter() -> Self {
        let src = r##"{
            "version": "__OPENPENCIL_VERSION__",
            "children": [
              {"type":"frame","id":"n10","name":"Frame",
               "x":0,"y":0,"width":1200,"height":800,
               "fill":[{"type":"solid","color":"#FFFFFF"}],
               "children":[]}
            ]
        }"##;
        let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));
        let doc = jian_ops_schema::load_str(&src)
            .expect("EditorState::starter() fixture parses")
            .value;
        Self::from_document(doc)
    }

    /// Spawn a fresh leaf node for the active shape / frame / text /
    /// form-widget tool at `(doc_x, doc_y)`, sized `init_w × init_h`.
    /// Returns the new node's id on success; `None` for `Select` /
    /// `Hand` (no creatable kind) or when the id allocator is
    /// exhausted.
    ///
    /// Form-widget tools ([`Tool::widget_kind`]) ignore the caller's
    /// `init_w` / `init_h` and use the widget's spec default box
    /// ([`crate::widget_default_size`]) so a single click drops a
    /// correctly-sized widget; non-widget tools keep their existing
    /// drag-init sizing.
    ///
    /// `next_id` is the host's monotonic id counter — bumped past the
    /// document's highest id so a loaded document with ids near the
    /// counter cannot mint a colliding id.
    pub fn create_node_for_tool(
        &mut self,
        tool: Tool,
        next_id: &mut u64,
        doc_x: f64,
        doc_y: f64,
        init_w: f64,
        init_h: f64,
    ) -> Option<NodeId> {
        let mut allocator = crate::SequentialIdAllocator::for_document(&self.doc, *next_id).ok()?;
        let result = self
            .create_node_for_tool_with_allocator(tool, &mut allocator, doc_x, doc_y, init_w, init_h)
            .ok()
            .flatten();
        if result.is_some() {
            *next_id = allocator.next_counter();
        }
        result
    }

    /// Insert `node` at the page root, directly above the selection's top-level
    /// ancestor — one row up in the LayerPanel, which is one step toward the
    /// front in z-order (the canvas paints children back-to-front via
    /// `children.iter().rev()`, so a lower index renders in front).
    ///
    /// The new node is free-positioned (explicit `x`/`y` at the viewport
    /// centre), so it is kept at the page root rather than nested into the
    /// selection's parent: nesting it into a flex/auto-layout frame would
    /// reflow it away from the cursor (and could detach the selected flow
    /// child), and nesting into a clipped frame could hide it. Falls back to
    /// appending at the page root (back) when nothing is selected. Callers
    /// select the new node afterwards.
    pub(crate) fn insert_node_above_selection(&mut self, node: PenNode) {
        let sel = self.selection.anchor.clone();
        if sel.is_real() {
            if let Some(idx) = self
                .active_children()
                .iter()
                .position(|n| crate::walkers::descendant_contains(n, &sel))
            {
                self.active_children_mut().insert(idx, node);
                return;
            }
        }
        self.active_children_mut().push(node);
    }

    /// Insert an Image node centred on the current viewport with a
    /// default 300×200 box, directly above the current selection. `src` is
    /// typically a `data:` URL the host already produced from a picked file.
    /// Returns the new node id on success; `None` when the id allocator is
    /// exhausted.
    pub fn insert_image_node_at_viewport(&mut self, name: &str, src: &str) -> Option<NodeId> {
        let mut allocator = crate::DocumentIdAllocator::sequential_for_document(&self.doc).ok()?;
        self.insert_image_node_at_viewport_with_allocator(name, src, &mut allocator)
            .ok()
            .flatten()
    }

    /// Insert an Image node centred on the current viewport, preserving the
    /// source bitmap's aspect ratio. The largest side is capped at 300
    /// document pixels and smaller bitmaps are not enlarged.
    pub fn insert_image_node_at_viewport_sized(
        &mut self,
        name: &str,
        src: &str,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Option<NodeId> {
        let mut allocator = crate::DocumentIdAllocator::sequential_for_document(&self.doc).ok()?;
        self.insert_image_node_at_viewport_sized_with_allocator(
            name,
            src,
            pixel_width,
            pixel_height,
            &mut allocator,
        )
        .ok()
        .flatten()
    }

    /// Like [`Self::insert_image_node_at_viewport_sized`], but centred on an
    /// explicit DOC-space point — the drop point of a dragged image file, so
    /// the node lands where the user released it instead of mid-viewport.
    pub fn insert_image_node_at_doc_point_sized(
        &mut self,
        name: &str,
        src: &str,
        pixel_width: u32,
        pixel_height: u32,
        centre: (f64, f64),
    ) -> Option<NodeId> {
        let mut allocator = crate::DocumentIdAllocator::sequential_for_document(&self.doc).ok()?;
        self.insert_image_node_at_doc_point_sized_with_allocator(
            name,
            src,
            pixel_width,
            pixel_height,
            centre,
            &mut allocator,
        )
        .ok()
        .flatten()
    }

    /// Insert a Lucide `icon_font` node centered at the given document
    /// point. The native icon picker only offers names the renderer can
    /// resolve, so this mutator validates only that the chosen name is
    /// non-empty.
    pub fn insert_icon_font_node_at(
        &mut self,
        icon_name: &str,
        family: &str,
        center_x: f64,
        center_y: f64,
    ) -> Option<NodeId> {
        let mut allocator = crate::DocumentIdAllocator::sequential_for_document(&self.doc).ok()?;
        self.insert_icon_font_node_at_with_allocator(
            icon_name,
            family,
            center_x,
            center_y,
            &mut allocator,
        )
        .ok()
        .flatten()
    }

    /// Insert a remote icon with baked SVG path data, centered at the
    /// given document point (GAP #26). When `svg_path_d` is `Some`,
    /// this builds a `Path` node carrying the fetched `d` plus an
    /// `iconId` — mirroring the TS toolbar's `parseSvgToNodes` insert
    /// (`toolbar.tsx::handleIconSelect`) and the shape the REPLACE
    /// path already writes — so a remote Iconify glyph renders its
    /// real geometry instead of the fallback dot. Without path data
    /// it falls back to [`EditorState::insert_icon_font_node_at`].
    pub fn insert_icon_node_at(
        &mut self,
        icon_name: &str,
        family: &str,
        svg_path_d: Option<&str>,
        center_x: f64,
        center_y: f64,
    ) -> Option<NodeId> {
        let mut allocator = crate::DocumentIdAllocator::sequential_for_document(&self.doc).ok()?;
        self.insert_icon_node_at_with_allocator(
            icon_name,
            family,
            svg_path_d,
            center_x,
            center_y,
            &mut allocator,
        )
        .ok()
        .flatten()
    }

    /// Replace the selected icon node with another Lucide glyph.
    /// `icon_font` nodes update their name/family directly. Path
    /// icons update `iconId` and optionally replace their SVG `d`
    /// data when the host can provide local path data.
    pub fn replace_selected_icon(
        &mut self,
        icon_name: &str,
        family: &str,
        svg_path_d: Option<&str>,
    ) -> bool {
        let icon_name = icon_name.trim();
        let family = family.trim();
        let sel = self.selection.anchor.clone();
        if icon_name.is_empty() || family.is_empty() || !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let can_replace = match self.selected_node() {
            Some(PenNode::IconFont(_)) => true,
            Some(PenNode::Path(n)) => n.icon_id.is_some(),
            _ => false,
        };
        if !can_replace {
            return false;
        }
        self.commit_history();
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        let icon_id = format!("{family}:{icon_name}");
        match node {
            PenNode::IconFont(n) => {
                n.icon_font_name = icon_name.to_string();
                n.icon_font_family = Some(family.to_string());
                n.base.name = Some(icon_id);
                true
            }
            PenNode::Path(n) if n.icon_id.is_some() => {
                n.icon_id = Some(icon_id.clone());
                n.base.name = Some(icon_id);
                if let Some(d) = svg_path_d {
                    n.d = Some(d.to_string());
                }
                true
            }
            _ => false,
        }
    }

    /// Replace the path nodes `source_ids` on the active page with a
    /// single new `Path` node whose geometry is the boolean `contours`
    /// (one closed polyline per subpath). Used by the host's path-boolean
    /// op: the skia `Path::op` math lives in the host layer, but the
    /// result is committed back through this `EditorState` mutator so the
    /// host never edits the canonical tree directly.
    ///
    /// The contours are emitted as a compound SVG `d` string (one
    /// `M … L … Z` per contour) rather than the single-contour `anchors`
    /// form, so holes / disjoint regions (Subtract / Exclude) survive:
    /// a `d` with ≥2 `Z` commands makes the renderer apply even-odd
    /// winding. Mirrors TS `boolean-ops.ts`, which also stores the result
    /// as `d` only.
    ///
    /// Returns the new node's id on success; `None` when no contour has
    /// ≥2 points or the id allocator is exhausted. Caller is responsible
    /// for the surrounding history snapshot + selection update.
    pub fn replace_paths_with_polyline(
        &mut self,
        source_ids: &[NodeId],
        contours: &[Vec<(f64, f64)>],
        next_id: &mut u64,
    ) -> Option<NodeId> {
        let mut allocator = crate::SequentialIdAllocator::for_document(&self.doc, *next_id).ok()?;
        let result = self
            .replace_paths_with_polyline_with_allocator(source_ids, contours, &mut allocator)
            .ok()
            .flatten();
        if result.is_some() {
            *next_id = allocator.next_counter();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::sizing::SizingBehavior;

    #[test]
    fn sample_has_the_demo_tree_and_anchors_selection() {
        let s = EditorState::sample();
        assert_eq!(s.doc.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(s.doc.children.len(), 1);
        assert_eq!(s.selection.anchor, NodeId::new("n11"));
        // The frame + its two children are present.
        assert_eq!(s.max_node_id(), 14);
    }

    #[test]
    fn starter_is_a_single_empty_frame_with_nothing_selected() {
        let s = EditorState::starter();
        assert_eq!(s.doc.version, env!("CARGO_PKG_VERSION"));
        // Exactly one top-level node — the starter Frame. The
        // `max_node_id() == 10` check proves no demo children:
        // the n11..n14 sample tree would lift it to 14.
        assert_eq!(s.doc.children.len(), 1);
        assert_eq!(s.max_node_id(), 10);
        assert!(s.selection.is_empty());
        let frame = match &s.doc.children[0] {
            jian_ops_schema::node::PenNode::Frame(frame) => frame,
            other => panic!("starter should be a frame, got {:?}", other),
        };
        assert_eq!(frame.base.x, Some(0.0));
        assert_eq!(frame.base.y, Some(0.0));
        assert!(matches!(
            frame.container.width,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(1200.0))
        ));
        assert!(matches!(
            frame.container.height,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(800.0))
        ));
        assert!(frame.container.stroke.is_none());
    }

    #[test]
    fn create_node_for_tool_spawns_a_rect_and_returns_its_id() {
        let mut s = EditorState::new();
        let mut next = 100u64;
        let id = s
            .create_node_for_tool(Tool::Rect, &mut next, 10.0, 20.0, 30.0, 40.0)
            .expect("rect created");
        assert!(id.is_real());
        assert_eq!(s.active_children().len(), 1);
    }

    #[test]
    fn create_node_for_tool_rejects_select_and_hand() {
        let mut s = EditorState::new();
        let mut next = 100u64;
        assert!(s
            .create_node_for_tool(Tool::Select, &mut next, 0.0, 0.0, 1.0, 1.0)
            .is_none());
        assert!(s
            .create_node_for_tool(Tool::Hand, &mut next, 0.0, 0.0, 1.0, 1.0)
            .is_none());
        assert!(s.active_children().is_empty());
    }

    #[test]
    fn replace_paths_with_polyline_swaps_sources_for_one_node() {
        let mut s = EditorState::new();
        let mut next = 100u64;
        // Two throwaway source paths.
        let a = s
            .create_node_for_tool(Tool::Pen, &mut next, 0.0, 0.0, 1.0, 1.0)
            .unwrap();
        let b = s
            .create_node_for_tool(Tool::Pen, &mut next, 5.0, 5.0, 1.0, 1.0)
            .unwrap();
        assert_eq!(s.active_children().len(), 2);
        let result = s
            .replace_paths_with_polyline(
                &[a, b],
                &[vec![(10.0, 10.0), (0.0, 0.0), (10.0, 0.0)]],
                &mut next,
            )
            .expect("path committed");
        assert!(result.is_real());
        // Both sources gone, one result node remains.
        assert_eq!(s.active_children().len(), 1);
        let PenNode::Path(path) = &s.active_children()[0] else {
            panic!("boolean result should be a Path");
        };
        assert_eq!(path.base.x, Some(0.0));
        assert_eq!(path.base.y, Some(0.0));
        assert_eq!(path.width, Some(SizingBehavior::Number(10.0)));
        assert_eq!(path.height, Some(SizingBehavior::Number(10.0)));
        // Committed as a compound `d` (node-local coords), not anchors.
        assert!(
            path.anchors.is_none(),
            "result uses a compound d, not anchors"
        );
        assert_eq!(
            path.d.as_deref(),
            Some("M 10.00 10.00 L 0.00 0.00 L 10.00 0.00 Z")
        );
    }

    #[test]
    fn replace_paths_with_polyline_rejects_empty_points() {
        let mut s = EditorState::new();
        let mut next = 100u64;
        assert!(s.replace_paths_with_polyline(&[], &[], &mut next).is_none());
        // A contour with <2 points is degenerate and also rejected.
        assert!(s
            .replace_paths_with_polyline(&[], &[vec![(1.0, 1.0)]], &mut next)
            .is_none());
    }

    #[test]
    fn replace_paths_with_polyline_inherits_first_source_style() {
        let src = r##"{
            "version": "1.0.0",
            "children": [
              {"type":"rectangle","id":"n10","x":0,"y":0,"width":20,"height":20,
               "fill":[{"type":"solid","color":"#ff0000"}]},
              {"type":"rectangle","id":"n11","x":10,"y":0,"width":20,"height":20}
            ]
        }"##;
        let doc = jian_ops_schema::load_str(src)
            .expect("fixture parses")
            .value;
        let mut s = EditorState::from_document(doc);
        let mut next = 100u64;
        s.replace_paths_with_polyline(
            &[NodeId::new("n10"), NodeId::new("n11")],
            &[vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)]],
            &mut next,
        )
        .expect("path committed");
        let PenNode::Path(path) = &s.active_children()[0] else {
            panic!("boolean result should be a Path");
        };
        assert!(path.fill.is_some(), "result should keep first source fill");
    }
}
