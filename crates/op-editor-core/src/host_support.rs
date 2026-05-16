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
use jian_ops_schema::node::{PathNode, PenNode, PenNodeBase, PenPathAnchor};

impl EditorState {
    /// Build an editor state seeded with the demo sample document —
    /// a bounded Frame `n10` (`white` fill, 1-px black stroke)
    /// containing a Text `n11` and a Group `n12` (`n13` blue rect +
    /// `n14` text). Selection anchors on `n11`.
    ///
    /// Mirrors `openpencil_shell_core::document::Document::sample()`
    /// so the native host opens with identical content after the
    /// migration onto `EditorState`.
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
        assert!(s
            .replace_paths_with_polyline(&[], &[], &mut next)
            .is_none());
    }
}
