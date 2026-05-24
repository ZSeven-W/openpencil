//! Host-facing `EditorState` helpers for the native editor host.
//!
//! The native widget host (`openpencil-shell-native`) drives
//! `EditorState` as its single source of truth. A handful of host
//! operations have no 1:1 mutator on `EditorState` yet — node spawn
//! for the active tool, a non-test sample document for the host
//! constructor, and committing a path-boolean result. These are
//! collected here as additive, tested `EditorState` methods so the
//! host never forks tree-mutation logic of its own.

use crate::command_node::build_leaf_node;
use crate::fills::{set_primary_fill_hex, set_primary_stroke_hex};
use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::tool::Tool;
use jian_ops_schema::node::{IconFontNode, PathNode, PenNode, PenNodeBase, PenPathAnchor};

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
            "version": "0.8.0",
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
        let doc = jian_ops_schema::load_str(src)
            .expect("EditorState::sample() fixture parses")
            .value;
        let mut state = Self::from_document(doc);
        state.set_single_selection(NodeId::new("n11"));
        state
    }

    /// Build the document a fresh launch opens with — a single empty
    /// starter Frame `n10` (white fill, 1-px black stroke), selected
    /// so the user can immediately resize / move it or drop nodes
    /// inside. No demo decoration.
    pub fn starter() -> Self {
        let src = r##"{
            "version": "0.8.0",
            "children": [
              {"type":"frame","id":"n10","name":"Frame",
               "x":40,"y":40,"width":360,"height":240,
               "fill":[{"type":"solid","color":"#FFFFFF"}],
               "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#000000"}]},
               "children":[]}
            ]
        }"##;
        let doc = jian_ops_schema::load_str(src)
            .expect("EditorState::starter() fixture parses")
            .value;
        let mut state = Self::from_document(doc);
        state.set_single_selection(NodeId::new("n10"));
        state
    }

    /// Spawn a fresh leaf node for the active shape / frame / text
    /// tool at `(doc_x, doc_y)`, sized `init_w × init_h`. Returns the
    /// new node's id on success; `None` for `Select` / `Hand` (no
    /// creatable kind) or when the id allocator is exhausted.
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
        // `(canonical kind, display name)` for each creatable tool.
        // The Pen tool spawns a `path` named "Path"; other tools map
        // straight through. `Select` / `Hand` are not creatable.
        let (kind, name): (&str, &str) = match tool {
            Tool::Rect => ("rect", "Rectangle"),
            Tool::Ellipse => ("ellipse", "Ellipse"),
            Tool::Polygon => ("polygon", "Polygon"),
            Tool::Line => ("line", "Line"),
            Tool::Pen => ("path", "Path"),
            Tool::Frame => ("frame", "Frame"),
            Tool::Text => ("text", "Text"),
            Tool::Select | Tool::Hand => return None,
        };
        // Allocator-collision guard — lift the counter past the
        // document's id space before minting.
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let id = NodeId::new(format!("n{}", *next_id));
        *next_id = (*next_id).checked_add(1)?;
        let mut node = build_leaf_node(
            kind,
            id.as_str(),
            name,
            doc_x.round() as i32,
            doc_y.round() as i32,
            init_w.round().max(0.0) as i32,
            init_h.round().max(0.0) as i32,
        )?;
        // Default paints — match the shell-core host's create path:
        // shape tools get a light-grey body fill; Line / Pen get a
        // 2-px black stroke; Frame gets a white fill.
        match tool {
            Tool::Rect | Tool::Ellipse | Tool::Polygon => {
                set_primary_fill_hex(&mut node, "#BDC7D9");
            }
            Tool::Line | Tool::Pen => {
                set_primary_stroke_hex(&mut node, "#000000");
            }
            Tool::Frame => {
                set_primary_fill_hex(&mut node, "#FFFFFF");
            }
            Tool::Text | Tool::Select | Tool::Hand => {}
        }
        self.active_children_mut().push(node);
        Some(id)
    }

    /// Append an Image node centred on the current viewport with a
    /// default 300×200 box. `src` is typically a `data:` URL the host
    /// already produced from a picked file. Returns the new node id on
    /// success; `None` when the id allocator is exhausted.
    pub fn insert_image_node_at_viewport(&mut self, name: &str, src: &str) -> Option<NodeId> {
        use jian_ops_schema::node::image::ImageNode;
        use jian_ops_schema::node::PenNode;
        use jian_ops_schema::sizing::SizingBehavior;
        const W: f64 = 300.0;
        const H: f64 = 200.0;
        let pan_x = self.viewport.pan_x as f64;
        let pan_y = self.viewport.pan_y as f64;
        let zoom = self.viewport.zoom.max(0.001) as f64;
        let centre_x = -pan_x / zoom;
        let centre_y = -pan_y / zoom;
        let safe = self.max_node_id().checked_add(1)?;
        let id = NodeId::new(format!("n{}", safe));
        let mut next_id = safe.checked_add(1)?;
        let _ = &mut next_id;
        self.commit_history();
        let node = PenNode::Image(ImageNode {
            base: jian_ops_schema::node::base::PenNodeBase {
                id: id.as_str().to_string(),
                name: Some(name.to_string()),
                x: Some(centre_x - W / 2.0),
                y: Some(centre_y - H / 2.0),
                ..Default::default()
            },
            src: src.to_string(),
            object_fit: None,
            width: Some(SizingBehavior::Number(W)),
            height: Some(SizingBehavior::Number(H)),
            corner_radius: None,
            effects: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
            image_prompt: None,
            image_search_query: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        });
        self.active_children_mut().push(node);
        self.set_single_selection(id.clone());
        Some(id)
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
        let icon_name = icon_name.trim();
        if icon_name.is_empty() {
            return None;
        }
        const SIZE: f64 = 32.0;
        let safe = self.max_node_id().checked_add(1)?;
        let id = NodeId::new(format!("n{}", safe));
        self.commit_history();
        let node = PenNode::IconFont(IconFontNode {
            base: PenNodeBase {
                id: id.as_str().to_string(),
                name: Some(icon_name.to_string()),
                x: Some(center_x - SIZE / 2.0),
                y: Some(center_y - SIZE / 2.0),
                ..Default::default()
            },
            icon_font_name: icon_name.to_string(),
            icon_font_family: Some(family.to_string()),
            width: Some(jian_ops_schema::sizing::SizingBehavior::Number(SIZE)),
            height: Some(jian_ops_schema::sizing::SizingBehavior::Number(SIZE)),
            fill: Some(vec![jian_ops_schema::style::PenFill::Solid(
                jian_ops_schema::style::SolidFillBody {
                    color: "#111827".to_string(),
                    explain: None,
                    opacity: None,
                    blend_mode: None,
                },
            )]),
            stroke: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        });
        self.active_children_mut().push(node);
        self.set_single_selection(id.clone());
        Some(id)
    }

    /// Replace the selected node's primary fill with an Image fill
    /// rooted at `src` (typically a `data:` URL). Existing colour /
    /// gradient is overwritten; non-fillable variants reject silently.
    /// Returns `true` on success.
    pub fn set_selected_fill_image_url(&mut self, src: &str) -> bool {
        use jian_ops_schema::style::{ImageFillBody, PenFill};
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = crate::walkers::find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        let Some(fills) = crate::fills::node_fills_mut(node) else {
            return false;
        };
        let body = PenFill::Image(ImageFillBody {
            url: src.to_string(),
            mode: None,
            original_size: None,
            transform: None,
            explain: None,
            opacity: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
        });
        if fills.is_empty() {
            fills.push(body);
        } else {
            fills[0] = body;
        }
        true
    }

    /// Replace the path nodes `source_ids` on the active page with a
    /// single new `Path` node whose anchors trace `points`. Used by
    /// the host's path-boolean op: the skia `Path::op` math lives in
    /// `openpencil-shell-native`, but the resulting polyline is
    /// committed back through this `EditorState` mutator so the host
    /// never edits the canonical tree directly.
    ///
    /// Returns the new node's id on success; `None` when `points` is
    /// empty or the id allocator is exhausted. Caller is responsible
    /// for the surrounding history snapshot + selection update.
    pub fn replace_paths_with_polyline(
        &mut self,
        source_ids: &[NodeId],
        points: &[(f64, f64)],
        next_id: &mut u64,
    ) -> Option<NodeId> {
        if points.is_empty() {
            return None;
        }
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let id = NodeId::new(format!("n{}", *next_id));
        *next_id = (*next_id).checked_add(1)?;
        // Remove every source path from the active page.
        for src in source_ids {
            crate::walkers::remove_from_children(self.active_children_mut(), src);
        }
        let first = points[0];
        let anchors: Vec<PenPathAnchor> = points
            .iter()
            .map(|(x, y)| PenPathAnchor {
                x: *x,
                y: *y,
                handle_in: None,
                handle_out: None,
                point_type: None,
            })
            .collect();
        let node = PenNode::Path(PathNode {
            base: PenNodeBase {
                id: id.as_str().to_string(),
                name: Some("Boolean Result".to_string()),
                x: Some(first.0),
                y: Some(first.1),
                ..Default::default()
            },
            icon_id: None,
            d: None,
            anchors: Some(anchors),
            closed: Some(true),
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
        });
        self.active_children_mut().push(node);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_has_the_demo_tree_and_anchors_selection() {
        let s = EditorState::sample();
        assert_eq!(s.doc.children.len(), 1);
        assert_eq!(s.selection.anchor, NodeId::new("n11"));
        // The frame + its two children are present.
        assert_eq!(s.max_node_id(), 14);
    }

    #[test]
    fn starter_is_a_single_empty_frame_with_it_selected() {
        let s = EditorState::starter();
        // Exactly one top-level node — the starter Frame. The
        // `max_node_id() == 10` check proves no demo children:
        // the n11..n14 sample tree would lift it to 14.
        assert_eq!(s.doc.children.len(), 1);
        assert_eq!(s.max_node_id(), 10);
        assert_eq!(s.selection.anchor, NodeId::new("n10"));
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
                &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
                &mut next,
            )
            .expect("polyline committed");
        assert!(result.is_real());
        // Both sources gone, one result node remains.
        assert_eq!(s.active_children().len(), 1);
    }

    #[test]
    fn replace_paths_with_polyline_rejects_empty_points() {
        let mut s = EditorState::new();
        let mut next = 100u64;
        assert!(s.replace_paths_with_polyline(&[], &[], &mut next).is_none());
    }
}
