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
use crate::walkers::{find_node, find_node_mut};
use jian_ops_schema::node::{IconFontNode, PathNode, PenNode, PenNodeBase};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{PenEffect, PenFill, PenStroke};

#[derive(Default)]
struct BooleanPathStyle {
    fill: Option<Vec<PenFill>>,
    stroke: Option<PenStroke>,
    effects: Option<Vec<PenEffect>>,
}

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
    /// starter Frame `n10` matching the TypeScript app's blank
    /// document geometry, selected so the user can immediately resize
    /// / move it or drop nodes inside. No demo decoration.
    pub fn starter() -> Self {
        let src = r##"{
            "version": "0.8.0",
            "children": [
              {"type":"frame","id":"n10","name":"Frame",
               "x":0,"y":0,"width":1200,"height":800,
               "fill":[{"type":"solid","color":"#FFFFFF"}],
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
        let icon_name = icon_name.trim();
        if icon_name.is_empty() {
            return None;
        }
        let Some(d) = svg_path_d.map(str::trim).filter(|d| !d.is_empty()) else {
            return self.insert_icon_font_node_at(icon_name, family, center_x, center_y);
        };
        const SIZE: f64 = 32.0;
        let safe = self.max_node_id().checked_add(1)?;
        let id = NodeId::new(format!("n{}", safe));
        let icon_id = format!("{}:{}", family.trim(), icon_name);
        self.commit_history();
        let node = PenNode::Path(PathNode {
            base: PenNodeBase {
                id: id.as_str().to_string(),
                name: Some(icon_id.clone()),
                x: Some(center_x - SIZE / 2.0),
                y: Some(center_y - SIZE / 2.0),
                ..Default::default()
            },
            icon_id: Some(icon_id),
            d: Some(d.to_string()),
            anchors: None,
            closed: None,
            width: Some(SizingBehavior::Number(SIZE)),
            height: Some(SizingBehavior::Number(SIZE)),
            fill: Some(vec![jian_ops_schema::style::PenFill::Solid(
                jian_ops_schema::style::SolidFillBody {
                    color: "#111827".to_string(),
                    explain: None,
                    opacity: None,
                    blend_mode: None,
                },
            )]),
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
        self.set_single_selection(id.clone());
        Some(id)
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
        if let PenNode::Image(image) = node {
            image.src = src.to_string();
            return true;
        }
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
        // Keep only contours with real extent (≥2 points).
        let usable: Vec<&Vec<(f64, f64)>> = contours.iter().filter(|c| c.len() >= 2).collect();
        if usable.is_empty() {
            return None;
        }
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let id = NodeId::new(format!("n{}", *next_id));
        *next_id = (*next_id).checked_add(1)?;
        let style = source_ids
            .iter()
            .find_map(|src| find_node(self.active_children(), src).map(boolean_path_style))
            .unwrap_or_default();
        let (min_x, min_y, max_x, max_y) = usable.iter().flat_map(|c| c.iter()).fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(min_x, min_y, max_x, max_y), (x, y)| {
                (min_x.min(*x), min_y.min(*y), max_x.max(*x), max_y.max(*y))
            },
        );
        let width = (max_x - min_x).max(0.0);
        let height = (max_y - min_y).max(0.0);
        // Remove every source shape from the active page.
        for src in source_ids {
            crate::walkers::remove_from_children(self.active_children_mut(), src);
        }
        // Compound `d` in node-local coordinates (origin at the bbox min):
        // one `M … L … Z` subpath per contour.
        let mut d = String::new();
        for contour in &usable {
            for (i, (x, y)) in contour.iter().enumerate() {
                let lx = *x - min_x;
                let ly = *y - min_y;
                if i == 0 {
                    d.push_str(&format!("M {:.2} {:.2}", lx, ly));
                } else {
                    d.push_str(&format!(" L {:.2} {:.2}", lx, ly));
                }
            }
            d.push_str(" Z");
        }
        let node = PenNode::Path(PathNode {
            base: PenNodeBase {
                id: id.as_str().to_string(),
                name: Some("Boolean Result".to_string()),
                x: Some(min_x),
                y: Some(min_y),
                ..Default::default()
            },
            icon_id: None,
            d: Some(d),
            anchors: None,
            closed: Some(true),
            width: Some(SizingBehavior::Number(width)),
            height: Some(SizingBehavior::Number(height)),
            fill: style.fill,
            stroke: style.stroke,
            effects: style.effects,
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

fn boolean_path_style(node: &PenNode) -> BooleanPathStyle {
    match node {
        PenNode::Frame(n) => BooleanPathStyle {
            fill: n.container.fill.clone(),
            stroke: n.container.stroke.clone(),
            effects: n.container.effects.clone(),
        },
        PenNode::Rectangle(n) => BooleanPathStyle {
            fill: n.container.fill.clone(),
            stroke: n.container.stroke.clone(),
            effects: n.container.effects.clone(),
        },
        PenNode::Ellipse(n) => BooleanPathStyle {
            fill: n.fill.clone(),
            stroke: n.stroke.clone(),
            effects: n.effects.clone(),
        },
        PenNode::Polygon(n) => BooleanPathStyle {
            fill: n.fill.clone(),
            stroke: n.stroke.clone(),
            effects: n.effects.clone(),
        },
        PenNode::Path(n) => BooleanPathStyle {
            fill: n.fill.clone(),
            stroke: n.stroke.clone(),
            effects: n.effects.clone(),
        },
        PenNode::Line(n) => BooleanPathStyle {
            fill: None,
            stroke: n.stroke.clone(),
            effects: n.effects.clone(),
        },
        _ => BooleanPathStyle::default(),
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
            "version": "0.8.0",
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
