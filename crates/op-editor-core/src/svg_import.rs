//! SVG import — parse an SVG document into canonical `PenNode`s and
//! insert them onto the active page.
//!
//! TS parity with `apps/web/src/.../svg-parser.ts`. v1 scope:
//!
//!   - Shape elements: `<rect>` / `<circle>` / `<ellipse>` / `<line>` /
//!     `<polyline>` / `<polygon>`.
//!   - `<path>` — the `M L H V C S Q T Z` command subset (absolute +
//!     relative). Cubic / quadratic curves keep their bezier handles
//!     (`Q`/`T` are promoted to cubics); `A` (elliptical arc) degrades
//!     to a straight segment to its endpoint.
//!   - `fill` attribute — `#rgb` / `#rrggbb` + a small named-colour
//!     table; `none` leaves the node unfilled.
//!
//! Out of scope for v1 (skipped, not an error): `<g>` grouping,
//! `transform` attributes, CSS `<style>`, `<defs>` / gradients,
//! `stroke` styling. Elements are scanned flat, so a shape nested in a
//! `<g>` still imports — just without the group's transform.
//!
//! The parser is hand-rolled (no XML / SVG crate) so `op-editor-core`
//! stays dependency-light + wasm32-clean, matching the hand-rolled
//! JSON parser discipline in `op-mcp`.
//!
//! ### Module layout
//!
//! This file is the spine: the shared element/style/tree types plus
//! the public [`EditorState`] entry points. The parsing + node-building
//! internals live in sibling submodules (per the 800-line-per-file
//! ceiling) and are glob-imported back here, so nothing about the
//! module's public surface changes:
//!
//! - `xml` — root `<svg>` extraction, viewBox-aware scale + inherited
//!   style context, recursive element tree walk
//! - `nodes` — element → `PenNode` conversion (containers + leaves)
//! - `style` — fill / stroke resolution + the named-colour table
//! - `scale` — applying the root scale to attrs / path data / points
//! - `lexer` — hand-rolled tag / attribute / number scanning
//! - `path_data` — the `<path d="…">` command reader

mod lexer;
mod nodes;
mod path_data;
mod scale;
mod style;
mod xml;

use lexer::*;
use nodes::*;
use path_data::*;
use scale::*;
use style::*;
use xml::*;

use crate::command_node::build_leaf_node;
use crate::fills::set_primary_fill_hex;
use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use jian_ops_schema::node::path::{PathFillRule, PenPathHandle};
use jian_ops_schema::node::{PathNode, PenNode, PenNodeBase, PenPathAnchor};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{PenFill, PenStroke, SolidFillBody, StrokeThickness};

/// One parsed SVG element — tag name + raw attribute pairs.
struct SvgElement {
    tag: String,
    attrs: Vec<(String, String)>,
}

impl SvgElement {
    /// Attribute lookup (first match wins).
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    /// Attribute parsed as `f64`, defaulting to `0.0` when absent or
    /// unparseable.
    fn num(&self, key: &str) -> f64 {
        self.attr(key)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    }
}

/// Inherited style context — propagates `fill` / `stroke` /
/// `stroke-width` from `<g>` parents to children, mirroring the TS
/// `StyleCtx` (`packages/pen-engine/src/core/svg-parser.ts`). `None`
/// means "no inherited value, use the renderer default".
#[derive(Debug, Clone)]
struct StyleCtx {
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: f64,
    fill_rule: Option<PathFillRule>,
}

impl Default for StyleCtx {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            fill_rule: None,
        }
    }
}

/// Tree element with parsed attrs + child elements (the recursive
/// `<g>` case the old flat `parse_svg_elements` could not handle).
struct SvgTree {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Vec<SvgTree>,
}

impl SvgTree {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    #[allow(dead_code)]
    fn num(&self, key: &str) -> f64 {
        self.attr(key)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    }
}

/// Tags ignored verbatim — definitions, gradients, clipping, scripts.
/// Mirrors the TS `SKIP_TAGS` constant.
const SKIP_TAGS: &[&str] = &[
    "defs",
    "style",
    "title",
    "desc",
    "metadata",
    "clippath",
    "mask",
    "filter",
    "lineargradient",
    "radialgradient",
    "symbol",
    "marker",
    "pattern",
    "script",
    "foreignobject",
    "animate",
    "animatemotion",
    "set",
];

/// Default raster output size — matches the TS parser's `maxDim` so
/// imports land at the same on-canvas size as the TS app.
const SVG_MAX_DIM: f64 = 400.0;

impl EditorState {
    /// Parse `svg` and insert the resulting nodes onto the active page,
    /// translated by `offset` doc-px. Returns the count of nodes
    /// inserted; `0` (no history pushed) when the SVG yields nothing.
    /// One history snapshot is pushed when ≥ 1 node lands.
    pub fn import_svg(&mut self, next_id: &mut u64, svg: &str, offset: (f64, f64)) -> usize {
        self.import_svg_named(next_id, svg, offset, None)
    }

    /// Like [`Self::import_svg`] but uses `name` (typically the file
    /// stem) as the wrapping Group's label so the layer panel shows
    /// the SVG's filename instead of "Imported SVG".
    pub fn import_svg_named(
        &mut self,
        next_id: &mut u64,
        svg: &str,
        offset: (f64, f64),
        name: Option<&str>,
    ) -> usize {
        let Ok(mut allocator) = SequentialIdAllocator::for_document(&self.doc, *next_id) else {
            return 0;
        };
        let result = self.import_svg_named_with_allocator(&mut allocator, svg, offset, name);
        *next_id = allocator.next_counter();
        result.unwrap_or(0)
    }

    /// Parse and import an SVG using a session-owned document allocator.
    pub fn import_svg_with_allocator(
        &mut self,
        allocator: &mut dyn IdAllocator,
        svg: &str,
        offset: (f64, f64),
    ) -> Result<usize, IdAllocError> {
        self.import_svg_named_with_allocator(allocator, svg, offset, None)
    }

    /// Named allocator-aware form of [`Self::import_svg_named`].
    pub fn import_svg_named_with_allocator(
        &mut self,
        allocator: &mut dyn IdAllocator,
        svg: &str,
        offset: (f64, f64),
        group_name: Option<&str>,
    ) -> Result<usize, IdAllocError> {
        // TS-parity pipeline (`packages/pen-engine/src/core/svg-parser.ts`):
        //  1. Extract the root `<svg>` attrs + compute a viewBox-aware
        //     scale so a 24×24 icon doesn't render at 24 px.
        //  2. Walk the body recursively so `<g>` children inherit
        //     style + don't escape into siblings.
        //  3. Build a node tree; `<g>` becomes a Group with the
        //     enclosed children.
        let (body, root_attrs) = match extract_svg_root(svg) {
            Some(p) => p,
            None => return Ok(0),
        };
        let (scale, root_ctx) = compute_root_scale(&root_attrs);
        let tree = parse_svg_tree(body);
        if tree.is_empty() {
            return Ok(0);
        }
        let mut taken = self.collect_node_ids();
        let mut built: Vec<PenNode> = Vec::new();
        for el in &tree {
            if let Some(node) =
                element_to_node_ctx(el, &root_ctx, scale, offset, allocator, &mut taken)?
            {
                built.push(node);
            }
        }
        if built.is_empty() {
            return Ok(0);
        }
        let count = built.len();
        // Wrap the imported nodes in a Group so the user can move /
        // delete the SVG as a unit instead of `count` flat siblings
        // each picking up its own row in the layer panel. Single-
        // element SVGs (logos, an `<svg>` wrapping one `<path>`) also
        // benefit because future grouping ops then have a stable
        // container to attach to.
        let group_id = allocator.allocate(&mut taken)?;
        use jian_ops_schema::node::container::ContainerProps;
        use jian_ops_schema::node::{GroupNode, PenNode};
        let group = PenNode::Group(GroupNode {
            base: jian_ops_schema::node::base::PenNodeBase {
                id: group_id.as_str().to_string(),
                name: Some(
                    group_name
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Imported SVG".to_string()),
                ),
                ..Default::default()
            },
            container: ContainerProps::default(),
            children: Some(built),
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        });
        let pre = self.snapshot_for_history();
        self.active_children_mut().push(group);
        self.set_single_selection(group_id);
        self.history_push_past(pre);
        Ok(count)
    }
}
