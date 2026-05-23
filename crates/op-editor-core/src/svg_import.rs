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

use crate::fills::set_primary_fill_hex;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::{command_node::build_leaf_node, walkers};
use jian_ops_schema::node::path::PenPathHandle;
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
}

impl Default for StyleCtx {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: 1.0,
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
        self.import_svg_impl(next_id, svg, offset, name)
    }

    fn import_svg_impl(
        &mut self,
        next_id: &mut u64,
        svg: &str,
        offset: (f64, f64),
        group_name: Option<&str>,
    ) -> usize {
        // TS-parity pipeline (`packages/pen-engine/src/core/svg-parser.ts`):
        //  1. Extract the root `<svg>` attrs + compute a viewBox-aware
        //     scale so a 24×24 icon doesn't render at 24 px.
        //  2. Walk the body recursively so `<g>` children inherit
        //     style + don't escape into siblings.
        //  3. Build a node tree; `<g>` becomes a Group with the
        //     enclosed children.
        let (body, root_attrs) = match extract_svg_root(svg) {
            Some(p) => p,
            None => return 0,
        };
        let (scale, root_ctx) = compute_root_scale(&root_attrs);
        let tree = parse_svg_tree(body);
        if tree.is_empty() {
            return 0;
        }
        if let Some(safe) = self.max_node_id().checked_add(1) {
            *next_id = (*next_id).max(safe);
        }
        let mut taken = self.collect_node_ids();
        let mut built: Vec<PenNode> = Vec::new();
        for el in &tree {
            if let Some(node) =
                element_to_node_ctx(el, &root_ctx, scale, offset, next_id, &mut taken)
            {
                built.push(node);
            }
        }
        if built.is_empty() {
            return 0;
        }
        let pre = self.snapshot_for_history();
        let count = built.len();
        // Wrap the imported nodes in a Group so the user can move /
        // delete the SVG as a unit instead of `count` flat siblings
        // each picking up its own row in the layer panel. Single-
        // element SVGs (logos, an `<svg>` wrapping one `<path>`) also
        // benefit because future grouping ops then have a stable
        // container to attach to.
        let Some(group_id) = walkers::alloc_n_id(next_id, &mut taken) else {
            // Allocator exhausted — fall back to the flat extend so we
            // don't drop the user's import on the floor.
            self.active_children_mut().extend(built);
            self.history_push_past(pre);
            return count;
        };
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
        self.active_children_mut().push(group);
        self.set_single_selection(group_id);
        self.history_push_past(pre);
        count
    }
}

/// Pull the root `<svg …>` open tag from a document. Returns the
/// body between `<svg …>` and `</svg>` plus the parsed root attrs.
/// `None` when the document lacks a balanced `<svg>` element — fed
/// to the regex-equivalent walker the way `parseSvgRegex` does in
/// the TS port.
fn extract_svg_root(svg: &str) -> Option<(&str, Vec<(String, String)>)> {
    let lower: String = svg.chars().map(|c| c.to_ascii_lowercase()).collect();
    let open = lower.find("<svg")?;
    let after_open = open + 4;
    let bytes = svg.as_bytes();
    let close_of_open = find_tag_end(bytes, after_open)?;
    let body_start = close_of_open + 1;
    // Closing tag is the last `</svg>` in the document.
    let close_marker = lower.rfind("</svg>")?;
    if close_marker < body_start {
        return None;
    }
    let attrs_str = &svg[after_open..close_of_open].trim_end_matches('/');
    let attrs = parse_attrs(attrs_str);
    Some((&svg[body_start..close_marker], attrs))
}

/// Compute the viewBox-aware scale factor + seed style context. The
/// TS port caps the longer side at `maxDim` (400 px) and scales
/// every coord uniformly so an icon authored in a 24-unit viewBox
/// lands at 400 px instead of 24 px on the canvas.
fn compute_root_scale(root_attrs: &[(String, String)]) -> (f64, StyleCtx) {
    let attr = |key: &str| -> Option<&str> {
        root_attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let view_box = attr("viewBox").or_else(|| attr("viewbox"));
    let mut vb_w = 100.0_f64;
    let mut vb_h = 100.0_f64;
    if let Some(vb) = view_box {
        let nums: Vec<f64> = vb
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if nums.len() >= 4 {
            vb_w = nums[2].max(0.001);
            vb_h = nums[3].max(0.001);
        }
    }
    let parse_dim = |raw: &str| -> Option<f64> {
        // Strip a trailing `px` so `width="24px"` round-trips; bail
        // on `%` / `em` / `vh` since they need parent context.
        let trimmed = raw.trim().trim_end_matches("px");
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.chars().any(|c| !c.is_ascii_digit() && c != '.') {
            return None;
        }
        trimmed.parse::<f64>().ok()
    };
    let svg_w = attr("width").and_then(parse_dim).unwrap_or(vb_w);
    let svg_h = attr("height").and_then(parse_dim).unwrap_or(vb_h);
    let mut out_w = svg_w;
    let mut out_h = svg_h;
    if out_w > SVG_MAX_DIM || out_h > SVG_MAX_DIM {
        let s = SVG_MAX_DIM / out_w.max(out_h);
        out_w *= s;
        out_h *= s;
    }
    // Children scale by `out / vb` — matches the TS impl exactly.
    let scale = (out_w / vb_w).min(out_h / vb_h).max(0.001);
    let stroke_w = attr("stroke-width")
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(1.0);
    let ctx = StyleCtx {
        fill: attr("fill").map(|s| s.to_string()),
        stroke: attr("stroke").map(|s| s.to_string()),
        stroke_width: stroke_w,
    };
    (scale, ctx)
}

/// Recursive tree walker — depth-tracking version of
/// `parse_svg_elements`. Each opening tag pairs with its matching
/// `</tag>` so `<g>` children land under the right parent. Skip
/// tags (`defs` / `style` / …) are filtered out before recursion.
fn parse_svg_tree(body: &str) -> Vec<SvgTree> {
    let bytes = body.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        // Skip comments.
        if body[i..].starts_with("<!--") {
            match body[i..].find("-->") {
                Some(rel) => i += rel + 3,
                None => break,
            }
            continue;
        }
        // Prolog / DOCTYPE / stray closing tag — advance to `>`.
        if matches!(bytes.get(i + 1), Some(b'/') | Some(b'?') | Some(b'!')) {
            match body[i..].find('>') {
                Some(rel) => i += rel + 1,
                None => break,
            }
            continue;
        }
        let Some(open_end) = find_tag_end(bytes, i + 1) else {
            break;
        };
        let inner = &body[i + 1..open_end];
        let self_closing = inner.trim_end().ends_with('/');
        let (tag_lower, attrs) = match parse_element(inner) {
            Some(el) => (el.tag, el.attrs),
            None => {
                i = open_end + 1;
                continue;
            }
        };
        if SKIP_TAGS.iter().any(|t| *t == tag_lower) {
            // Skip its body too so child shapes inside `<defs>` don't
            // leak into the import.
            i = if self_closing {
                open_end + 1
            } else {
                skip_until_closing(body, open_end + 1, &tag_lower).unwrap_or(body.len())
            };
            continue;
        }
        if self_closing {
            out.push(SvgTree {
                tag: tag_lower,
                attrs,
                children: Vec::new(),
            });
            i = open_end + 1;
            continue;
        }
        let body_start = open_end + 1;
        let body_end = find_matching_close(body, body_start, &tag_lower).unwrap_or(body.len());
        let inner_body = &body[body_start..body_end];
        out.push(SvgTree {
            tag: tag_lower,
            attrs,
            children: parse_svg_tree(inner_body),
        });
        // Advance past the close tag itself.
        i = match body[body_end..].find('>') {
            Some(rel) => body_end + rel + 1,
            None => body.len(),
        };
    }
    out
}

/// Index just past the `</tag>` that closes the given open tag at
/// `from`. Handles nested same-name tags so a `<g>` inside a `<g>`
/// pairs with its own close. Returns `None` when no matching close
/// exists (malformed SVG); the caller treats the rest of the body
/// as the element's content.
fn find_matching_close(body: &str, from: usize, tag: &str) -> Option<usize> {
    let bytes = body.as_bytes();
    let mut depth = 1usize;
    let mut i = from;
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}");
    while i < bytes.len() {
        let rest = &body[i..];
        let lower = rest.to_ascii_lowercase();
        let next_open = lower.find(&open_pat);
        let next_close = lower.find(&close_pat);
        let (idx, is_close) = match (next_open, next_close) {
            (None, None) => return None,
            (Some(o), None) => (o, false),
            (None, Some(c)) => (c, true),
            (Some(o), Some(c)) => {
                if o < c {
                    (o, false)
                } else {
                    (c, true)
                }
            }
        };
        // Confirm the match is followed by `>` or whitespace (so
        // `<rectangle>` doesn't false-match a `<rect>` close).
        let abs = i + idx;
        let after = abs
            + if is_close {
                close_pat.len()
            } else {
                open_pat.len()
            };
        let next_char = body.as_bytes().get(after).copied();
        let valid = matches!(
            next_char,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') | Some(b'/')
        );
        if !valid {
            i = abs + 1;
            continue;
        }
        if is_close {
            depth -= 1;
            if depth == 0 {
                return Some(abs);
            }
            i = abs + close_pat.len();
        } else {
            depth += 1;
            i = abs + open_pat.len();
        }
    }
    None
}

/// Skip to just past `</tag>` for a skip-tag's body. Returns the
/// index after the closing tag's `>`.
fn skip_until_closing(body: &str, from: usize, tag: &str) -> Option<usize> {
    let close = find_matching_close(body, from, tag)?;
    body[close..].find('>').map(|rel| close + rel + 1)
}

/// Build a `PenNode` for an SVG element with inherited style
/// context + viewBox scaling applied. `<g>` recurses into its
/// children and wraps them in a `PenNode::Group`.
fn element_to_node_ctx(
    el: &SvgTree,
    parent_ctx: &StyleCtx,
    scale: f64,
    offset: (f64, f64),
    next_id: &mut u64,
    taken: &mut std::collections::HashSet<NodeId>,
) -> Option<PenNode> {
    let ctx = merge_style_ctx(parent_ctx, &el.attrs);
    if el.tag == "g" || el.tag == "svg" {
        let mut kids: Vec<PenNode> = Vec::new();
        for child in &el.children {
            if let Some(node) = element_to_node_ctx(child, &ctx, scale, offset, next_id, taken) {
                kids.push(node);
            }
        }
        if kids.is_empty() {
            return None;
        }
        if kids.len() == 1 {
            return Some(kids.into_iter().next().unwrap());
        }
        let id = walkers::alloc_n_id(next_id, taken)?;
        use jian_ops_schema::node::container::ContainerProps;
        use jian_ops_schema::node::GroupNode;
        let name = el
            .attr("id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Group".to_string());
        return Some(PenNode::Group(GroupNode {
            base: PenNodeBase {
                id: id.into(),
                name: Some(name),
                ..Default::default()
            },
            container: ContainerProps::default(),
            children: Some(kids),
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }));
    }
    // Convert via the legacy element builder, then scale + apply
    // inherited fill/stroke.
    let legacy = SvgElement {
        tag: el.tag.clone(),
        attrs: el.attrs.clone(),
    };
    // Scale the geometry by writing scaled values back into the
    // element's attrs (so existing `element_to_node` sees scaled
    // numbers). For `<path>` we tokenise the `d` string + scale
    // every coord — TS parity with `scaleSvgPath`.
    let mut scaled = legacy;
    apply_scale_to_attrs(&mut scaled, scale);
    // Preserve `<path d>` as SVG path data. That keeps arcs and
    // compound subpaths in the same representation the TS renderer
    // sends to CanvasKit.
    let fill_hex = resolve_svg_fill_hex(&scaled.attrs, &ctx);
    let stroke = resolve_svg_stroke(&scaled.attrs, &ctx, scale);
    if scaled.tag == "path" {
        if let Some(d) = scaled.attr("d") {
            let id = walkers::alloc_n_id(next_id, taken)?;
            return path_node_from_svg_d(
                id,
                el.attr("id").unwrap_or("Path"),
                d,
                offset,
                fill_hex,
                stroke,
            );
        }
    }
    let id = walkers::alloc_n_id(next_id, taken)?;
    let mut node = element_to_node(&scaled, id, offset)?;
    if let Some(stroke) = stroke {
        set_node_stroke(&mut node, stroke);
    }
    if fill_hex.is_none() && scaled.tag != "line" && scaled.tag != "polyline" {
        // Explicit fill="none" / fill="transparent" should not leave
        // the old element builder's attribute fill behind.
        clear_node_fill(&mut node);
    } else if let Some(hex) = fill_hex.as_deref() {
        set_primary_fill_hex(&mut node, hex);
    }
    Some(node)
}

/// Merge `parent` with the element's own `fill` / `stroke` /
/// `stroke-width` (inline `style="..."` takes precedence over the
/// matching attribute, matching `extractStyleOrAttr` in TS).
fn merge_style_ctx(parent: &StyleCtx, attrs: &[(String, String)]) -> StyleCtx {
    StyleCtx {
        fill: extract_style_or_attr(attrs, "fill").or_else(|| parent.fill.clone()),
        stroke: extract_style_or_attr(attrs, "stroke").or_else(|| parent.stroke.clone()),
        stroke_width: extract_style_or_attr(attrs, "stroke-width")
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(parent.stroke_width),
    }
}

/// Look up `name` in an inline `style="..."` first, then fall back
/// to the named attribute. Mirrors `extractStyleOrAttr` from the TS
/// regex parser.
fn extract_style_or_attr(attrs: &[(String, String)], name: &str) -> Option<String> {
    if let Some(style) = attrs.iter().find(|(k, _)| k == "style").map(|(_, v)| v) {
        // Naive CSS-ish split — values can't contain `;` because
        // SVG inline styles forbid it.
        for pair in style.split(';') {
            let trimmed = pair.trim();
            if let Some((k, v)) = trimmed.split_once(':') {
                if k.trim().eq_ignore_ascii_case(name) {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.to_string())
}

fn resolve_svg_fill_hex(attrs: &[(String, String)], ctx: &StyleCtx) -> Option<String> {
    let raw = extract_style_or_attr(attrs, "fill").or_else(|| ctx.fill.clone());
    match raw.as_deref().map(str::trim) {
        None => Some("#000000".to_string()),
        Some(v) if v.eq_ignore_ascii_case("none") || v.eq_ignore_ascii_case("transparent") => None,
        Some(v) if v.to_ascii_lowercase().starts_with("url(") => Some("#000000".to_string()),
        Some(v) => parse_svg_color(v),
    }
}

fn resolve_svg_stroke(attrs: &[(String, String)], ctx: &StyleCtx, scale: f64) -> Option<PenStroke> {
    let raw = extract_style_or_attr(attrs, "stroke").or_else(|| ctx.stroke.clone())?;
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("none") || raw.to_ascii_lowercase().starts_with("url(") {
        return None;
    }
    let hex = parse_svg_color(raw).unwrap_or_else(|| "#000000".to_string());
    let width = (ctx.stroke_width * scale).max(0.0) as f32;
    if width <= 0.0 {
        return None;
    }
    Some(PenStroke {
        thickness: StrokeThickness::Uniform(width),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: Some(vec![solid_fill(&hex)]),
    })
}

fn solid_fill(hex: &str) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: hex.to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

fn set_node_stroke(node: &mut PenNode, stroke: PenStroke) {
    match node {
        PenNode::Frame(n) => n.container.stroke = Some(stroke),
        PenNode::Group(n) => n.container.stroke = Some(stroke),
        PenNode::Rectangle(n) => n.container.stroke = Some(stroke),
        PenNode::Ellipse(n) => n.stroke = Some(stroke),
        PenNode::Polygon(n) => n.stroke = Some(stroke),
        PenNode::Path(n) => n.stroke = Some(stroke),
        PenNode::Line(n) => n.stroke = Some(stroke),
        PenNode::TextInput(n) => n.stroke = Some(stroke),
        PenNode::Text(_) | PenNode::IconFont(_) | PenNode::Image(_) | PenNode::Ref(_) => {}
    }
}

fn clear_node_fill(node: &mut PenNode) {
    match node {
        PenNode::Frame(n) => n.container.fill = None,
        PenNode::Group(n) => n.container.fill = None,
        PenNode::Rectangle(n) => n.container.fill = None,
        PenNode::Ellipse(n) => n.fill = None,
        PenNode::Polygon(n) => n.fill = None,
        PenNode::Path(n) => n.fill = None,
        PenNode::Text(n) => n.fill = None,
        PenNode::TextInput(n) => n.fill = None,
        PenNode::IconFont(n) => n.fill = None,
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => {}
    }
}

/// Multiply every numeric coord on the element by `scale` so a
/// 24-unit viewBox shape renders at the same final size as the TS
/// app's viewBox-aware path. For `<path>` we tokenise the `d`
/// string + scale every coord; for shape elements we scale
/// `width` / `height` / `x` / `y` / `r` / `cx` / `cy` / `rx` /
/// `ry` / `x1` / `y1` / `x2` / `y2` in place.
fn apply_scale_to_attrs(el: &mut SvgElement, scale: f64) {
    if (scale - 1.0).abs() < 1e-6 {
        return;
    }
    if el.tag == "path" {
        if let Some(pos) = el.attrs.iter().position(|(k, _)| k == "d") {
            let scaled = scale_svg_path(&el.attrs[pos].1, scale);
            el.attrs[pos].1 = scaled;
        }
        return;
    }
    if el.tag == "polyline" || el.tag == "polygon" {
        if let Some(pos) = el.attrs.iter().position(|(k, _)| k == "points") {
            let scaled = scale_svg_points(&el.attrs[pos].1, scale);
            el.attrs[pos].1 = scaled;
        }
        return;
    }
    let scalable: &[&str] = &[
        "x", "y", "width", "height", "r", "rx", "ry", "cx", "cy", "x1", "y1", "x2", "y2",
    ];
    for (k, v) in &mut el.attrs {
        if !scalable.iter().any(|s| *s == k) {
            continue;
        }
        if let Ok(n) = v.trim().parse::<f64>() {
            *v = format!("{}", n * scale);
        }
    }
}

/// Token-aware scaler — preserves arc `A` flags (rotation +
/// large-arc + sweep are unitless) and scales the rest. Direct port
/// of `scaleSvgPath` from the TS impl.
fn scale_svg_path(d: &str, scale: f64) -> String {
    let mut out = String::with_capacity(d.len());
    let mut cmd: char = ' ';
    let mut param_idx = 0usize;
    let bytes = d.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() {
            cmd = c;
            param_idx = 0;
            out.push(c);
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() || c == ',' {
            out.push(c);
            i += 1;
            continue;
        }
        // Read a number (with optional sign / exponent).
        let start = i;
        if (c == '-' || c == '+') && i + 1 < bytes.len() {
            i += 1;
        }
        while i < bytes.len() {
            let cc = bytes[i] as char;
            if cc.is_ascii_digit() || cc == '.' {
                i += 1;
            } else if (cc == 'e' || cc == 'E')
                && i + 1 < bytes.len()
                && matches!(bytes[i + 1] as char, '-' | '+' | '0'..='9')
            {
                i += 1;
                if matches!(bytes[i] as char, '-' | '+') {
                    i += 1;
                }
            } else {
                break;
            }
        }
        if start == i {
            i += 1;
            continue;
        }
        let tok = &d[start..i];
        let Ok(n) = tok.parse::<f64>() else {
            out.push_str(tok);
            continue;
        };
        let upper = cmd.to_ascii_uppercase();
        let scaled = if upper == 'A' {
            // 7 params: rx ry rotation large-arc sweep x y
            let pos = param_idx % 7;
            let should_scale = pos == 0 || pos == 1 || pos == 5 || pos == 6;
            if should_scale {
                n * scale
            } else {
                n
            }
        } else {
            n * scale
        };
        out.push_str(&format!("{}", scaled));
        param_idx += 1;
    }
    out
}

/// Scale a `points="x1,y1 x2,y2 …"` list for `<polyline>` /
/// `<polygon>`. Returns the same separator style (`x,y x,y …`).
fn scale_svg_points(s: &str, scale: f64) -> String {
    parse_point_list(s)
        .into_iter()
        .map(|(x, y)| format!("{},{}", x * scale, y * scale))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build a `PenNode` from one parsed SVG element. `None` for
/// unsupported tags (`g` / `svg` / `defs` / `style` / …) and for
/// degenerate geometry.
fn element_to_node(el: &SvgElement, id: NodeId, offset: (f64, f64)) -> Option<PenNode> {
    let (ox, oy) = offset;
    let fill = el.attr("fill").and_then(parse_svg_color);
    let mut node = match el.tag.as_str() {
        "rect" => {
            let (w, h) = (el.num("width"), el.num("height"));
            if w <= 0.0 || h <= 0.0 {
                return None;
            }
            build_leaf_node(
                "rect",
                id.as_str(),
                "Rect",
                (el.num("x") + ox).round() as i32,
                (el.num("y") + oy).round() as i32,
                w.round() as i32,
                h.round() as i32,
            )?
        }
        "circle" => {
            let r = el.num("r");
            if r <= 0.0 {
                return None;
            }
            build_leaf_node(
                "ellipse",
                id.as_str(),
                "Ellipse",
                (el.num("cx") - r + ox).round() as i32,
                (el.num("cy") - r + oy).round() as i32,
                (r * 2.0).round() as i32,
                (r * 2.0).round() as i32,
            )?
        }
        "ellipse" => {
            let (rx, ry) = (el.num("rx"), el.num("ry"));
            if rx <= 0.0 || ry <= 0.0 {
                return None;
            }
            build_leaf_node(
                "ellipse",
                id.as_str(),
                "Ellipse",
                (el.num("cx") - rx + ox).round() as i32,
                (el.num("cy") - ry + oy).round() as i32,
                (rx * 2.0).round() as i32,
                (ry * 2.0).round() as i32,
            )?
        }
        "line" => {
            let p0 = (el.num("x1") + ox, el.num("y1") + oy);
            let p1 = (el.num("x2") + ox, el.num("y2") + oy);
            path_node_from_anchors(id, "Line", &[p0, p1], false)?
        }
        "polyline" | "polygon" => {
            let pts: Vec<(f64, f64)> = parse_point_list(el.attr("points").unwrap_or(""))
                .into_iter()
                .map(|(x, y)| (x + ox, y + oy))
                .collect();
            if pts.len() < 2 {
                return None;
            }
            path_node_from_anchors(id, "Path", &pts, el.tag == "polygon")?
        }
        "path" => {
            // Multi-subpath handling lives in `element_to_node_ctx`
            // where we have the id allocator + can build a Group;
            // this legacy single-id path serves only the first subpath
            // so existing callers keep working.
            let d = el.attr("d")?;
            let mut subpaths = parse_path_d(d, offset);
            let (anchors, closed) = subpaths.drain(..).next()?;
            if anchors.len() < 2 {
                return None;
            }
            path_node_from_pen_anchors(id, anchors, closed)?
        }
        // <svg> / <g> / <defs> / <style> / <title> / … — skipped.
        _ => return None,
    };
    if let Some(hex) = fill {
        set_primary_fill_hex(&mut node, &hex);
    }
    Some(node)
}

/// Build a straight-segment `Path` node from doc-space points.
fn path_node_from_anchors(
    id: NodeId,
    name: &str,
    pts: &[(f64, f64)],
    closed: bool,
) -> Option<PenNode> {
    let anchors: Vec<PenPathAnchor> = pts
        .iter()
        .map(|&(x, y)| PenPathAnchor {
            x,
            y,
            handle_in: None,
            handle_out: None,
            point_type: None,
        })
        .collect();
    path_node_from_pen_anchors(id, anchors, closed).map(|mut n| {
        n.base_mut().name = Some(name.to_string());
        n
    })
}

/// Build a `Path` node from ready `PenPathAnchor`s, fitting the base
/// rect to the anchor bounding box.
fn path_node_from_pen_anchors(
    id: NodeId,
    anchors: Vec<PenPathAnchor>,
    closed: bool,
) -> Option<PenNode> {
    if anchors.len() < 2 {
        return None;
    }
    let (min_x, min_y, max_x, max_y) = crate::svg_path_bounds::path_anchor_bounds(&anchors, closed);
    Some(PenNode::Path(PathNode {
        base: PenNodeBase {
            id: id.into(),
            name: Some("Path".to_string()),
            x: Some(min_x),
            y: Some(min_y),
            ..Default::default()
        },
        icon_id: None,
        d: None,
        anchors: Some(anchors),
        closed: Some(closed),
        width: Some(SizingBehavior::Number((max_x - min_x).max(0.0))),
        height: Some(SizingBehavior::Number((max_y - min_y).max(0.0))),
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
    }))
}

fn path_node_from_svg_d(
    id: NodeId,
    name: &str,
    d: &str,
    offset: (f64, f64),
    fill_hex: Option<String>,
    stroke: Option<PenStroke>,
) -> Option<PenNode> {
    let (local_d, bounds) = crate::svg_path_data::localize_svg_path(d)?;
    Some(PenNode::Path(PathNode {
        base: PenNodeBase {
            id: id.into(),
            name: Some(name.to_string()),
            x: Some(bounds.x + offset.0),
            y: Some(bounds.y + offset.1),
            ..Default::default()
        },
        icon_id: None,
        d: Some(local_d.clone()),
        anchors: None,
        closed: Some(local_d.contains('Z') || local_d.contains('z')),
        width: Some(SizingBehavior::Number(bounds.w)),
        height: Some(SizingBehavior::Number(bounds.h)),
        fill: fill_hex.map(|hex| vec![solid_fill(&hex)]),
        stroke,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    }))
}
/// Flat scan of every `<tag …>` / `<tag … />` element in `svg`.
/// Comments, the XML prolog, closing tags and DOCTYPE are skipped;
/// nesting is ignored, so a shape inside a `<g>` is still found.
#[allow(dead_code)] // Kept for tests / fallback callers; the TS-parity tree walker is `parse_svg_tree`.
fn parse_svg_elements(svg: &str) -> Vec<SvgElement> {
    let bytes = svg.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        // Skip comments `<!-- … -->`.
        if svg[i..].starts_with("<!--") {
            match svg[i..].find("-->") {
                Some(rel) => i += rel + 3,
                None => break,
            }
            continue;
        }
        // Closing tag / prolog / DOCTYPE — skip to the matching `>`.
        if matches!(bytes.get(i + 1), Some(b'/') | Some(b'?') | Some(b'!')) {
            match svg[i..].find('>') {
                Some(rel) => i += rel + 1,
                None => break,
            }
            continue;
        }
        // Open / self-closing element: read until the matching `>`,
        // honouring quoted attribute values.
        let Some(end) = find_tag_end(bytes, i + 1) else {
            break;
        };
        let inner = &svg[i + 1..end];
        if let Some(el) = parse_element(inner) {
            out.push(el);
        }
        i = end + 1;
    }
    out
}

/// Index of the `>` that closes a tag started at `from`, skipping any
/// `>` that sits inside a quoted attribute value.
fn find_tag_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                b'"' | b'\'' => quote = Some(c),
                b'>' => return Some(i),
                _ => {}
            },
        }
        i += 1;
    }
    None
}

/// Parse the inside of a tag (`rect x="1" y="2" /`) into tag name +
/// attribute pairs.
fn parse_element(inner: &str) -> Option<SvgElement> {
    let trimmed = inner.trim().trim_end_matches('/').trim();
    // Tag name runs up to the first whitespace.
    let name_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let tag = trimmed[..name_end].to_ascii_lowercase();
    if tag.is_empty() {
        return None;
    }
    Some(SvgElement {
        tag,
        attrs: parse_attrs(&trimmed[name_end..]),
    })
}

/// Parse `key="value"` / `key='value'` pairs from an attribute run.
fn parse_attrs(s: &str) -> Vec<(String, String)> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        // Skip to a key start.
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if key_start == i {
            break;
        }
        let key = s[key_start..i].to_ascii_lowercase();
        // Skip whitespace + the `=`.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            continue; // valueless attribute — ignore
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        i += 1;
        let val_start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        let value = s[val_start..i.min(s.len())].to_string();
        out.push((key, value));
        i += 1;
    }
    out
}

/// Parse an SVG `points` list (`"1,2 3,4"` / `"1 2 3 4"`).
fn parse_point_list(s: &str) -> Vec<(f64, f64)> {
    let nums = scan_numbers(s);
    nums.chunks_exact(2).map(|c| (c[0], c[1])).collect()
}

/// Scan every number out of `s`, treating commas + whitespace as
/// separators. Tolerates the SVG quirks: a leading `.`, a `-` that
/// starts a new number, and scientific `e` notation.
fn scan_numbers(s: &str) -> Vec<f64> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'-' || c == b'+' || c == b'.' || c.is_ascii_digit() {
            let start = i;
            i += 1;
            let mut seen_dot = c == b'.';
            let mut seen_exp = false;
            while i < bytes.len() {
                let d = bytes[i];
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == b'.' && !seen_dot && !seen_exp {
                    seen_dot = true;
                    i += 1;
                } else if (d == b'e' || d == b'E') && !seen_exp {
                    seen_exp = true;
                    i += 1;
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(n) = s[start..i].parse::<f64>() {
                out.push(n);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Parse an SVG path `d` string into a list of subpaths. Each `M`
/// (after the first move) starts a new subpath — SVG's pen-up
/// semantics — so the renderer doesn't draw a stray straight line
/// between disconnected outlines. Returns `Vec<(anchors, closed)>`;
/// supports `M L H V C S Q T Z` (absolute + relative); `A` degrades
/// to a straight segment to its endpoint.
fn parse_path_d(d: &str, offset: (f64, f64)) -> Vec<(Vec<PenPathAnchor>, bool)> {
    let tokens = tokenize_path(d);
    let (ox, oy) = offset;
    let mut subpaths: Vec<(Vec<PenPathAnchor>, bool)> = Vec::new();
    let mut anchors: Vec<PenPathAnchor> = Vec::new();
    let mut closed = false;
    // Current pen position, sub-path start, and the last control point
    // (for the smooth `S` / `T` reflection).
    let (mut cx, mut cy) = (0.0f64, 0.0f64);
    let (mut start_x, mut start_y) = (0.0f64, 0.0f64);
    let mut last_cubic_ctrl: Option<(f64, f64)> = None;
    let mut last_quad_ctrl: Option<(f64, f64)> = None;

    let push_anchor = |anchors: &mut Vec<PenPathAnchor>, x: f64, y: f64| {
        anchors.push(PenPathAnchor {
            x: x + ox,
            y: y + oy,
            handle_in: None,
            handle_out: None,
            point_type: None,
        });
    };

    let mut ti = 0usize;
    let mut cmd = b' ';
    while ti < tokens.len() {
        // A token is either a command letter or (when the previous
        // command repeats) a fresh number run.
        if let PathToken::Cmd(c) = tokens[ti] {
            cmd = c;
            ti += 1;
        }
        let rel = cmd.is_ascii_lowercase();
        let up = cmd.to_ascii_uppercase();
        // Collect the numbers this command consumes.
        let need = match up {
            b'M' | b'L' | b'T' => 2,
            b'H' | b'V' => 1,
            b'C' => 6,
            b'S' | b'Q' => 4,
            b'A' => 7,
            b'Z' => 0,
            _ => {
                ti += 1;
                continue;
            }
        };
        if up == b'Z' {
            closed = true;
            cx = start_x;
            cy = start_y;
            last_cubic_ctrl = None;
            last_quad_ctrl = None;
            continue;
        }
        let mut args = [0.0f64; 7];
        let mut got = 0;
        while got < need && ti < tokens.len() {
            if let PathToken::Num(n) = tokens[ti] {
                args[got] = n;
                got += 1;
                ti += 1;
            } else {
                break;
            }
        }
        if got < need {
            break; // truncated command — stop
        }
        match up {
            b'M' => {
                // Pen-up: a fresh `M` starts a new subpath. Flush the
                // current one (when it has ≥ 2 anchors) before starting
                // — otherwise multiple `M` commands in a single `d`
                // string get fused into one polyline with a stray
                // straight line between subpaths.
                if anchors.len() >= 2 {
                    subpaths.push((std::mem::take(&mut anchors), closed));
                } else {
                    anchors.clear();
                }
                closed = false;
                let (x, y) = abs_pt(rel, cx, cy, args[0], args[1]);
                cx = x;
                cy = y;
                start_x = x;
                start_y = y;
                push_anchor(&mut anchors, x, y);
                cmd = if rel { b'l' } else { b'L' }; // implicit lineto
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            b'L' => {
                let (x, y) = abs_pt(rel, cx, cy, args[0], args[1]);
                cx = x;
                cy = y;
                push_anchor(&mut anchors, x, y);
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            b'H' => {
                let x = if rel { cx + args[0] } else { args[0] };
                cx = x;
                push_anchor(&mut anchors, x, cy);
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            b'V' => {
                let y = if rel { cy + args[0] } else { args[0] };
                cy = y;
                push_anchor(&mut anchors, cx, y);
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            b'C' => {
                let (c1x, c1y) = abs_pt(rel, cx, cy, args[0], args[1]);
                let (c2x, c2y) = abs_pt(rel, cx, cy, args[2], args[3]);
                let (x, y) = abs_pt(rel, cx, cy, args[4], args[5]);
                emit_cubic(&mut anchors, c1x, c1y, c2x, c2y, x, y, ox, oy);
                cx = x;
                cy = y;
                last_cubic_ctrl = Some((c2x, c2y));
                last_quad_ctrl = None;
            }
            b'S' => {
                // Smooth cubic — first control reflects the previous.
                let (c1x, c1y) = match last_cubic_ctrl {
                    Some((px, py)) => (2.0 * cx - px, 2.0 * cy - py),
                    None => (cx, cy),
                };
                let (c2x, c2y) = abs_pt(rel, cx, cy, args[0], args[1]);
                let (x, y) = abs_pt(rel, cx, cy, args[2], args[3]);
                emit_cubic(&mut anchors, c1x, c1y, c2x, c2y, x, y, ox, oy);
                cx = x;
                cy = y;
                last_cubic_ctrl = Some((c2x, c2y));
                last_quad_ctrl = None;
            }
            b'Q' => {
                let (qx, qy) = abs_pt(rel, cx, cy, args[0], args[1]);
                let (x, y) = abs_pt(rel, cx, cy, args[2], args[3]);
                let (c1x, c1y, c2x, c2y) = quad_to_cubic(cx, cy, qx, qy, x, y);
                emit_cubic(&mut anchors, c1x, c1y, c2x, c2y, x, y, ox, oy);
                cx = x;
                cy = y;
                last_quad_ctrl = Some((qx, qy));
                last_cubic_ctrl = None;
            }
            b'T' => {
                // Smooth quadratic — control reflects the previous.
                let (qx, qy) = match last_quad_ctrl {
                    Some((px, py)) => (2.0 * cx - px, 2.0 * cy - py),
                    None => (cx, cy),
                };
                let (x, y) = abs_pt(rel, cx, cy, args[0], args[1]);
                let (c1x, c1y, c2x, c2y) = quad_to_cubic(cx, cy, qx, qy, x, y);
                emit_cubic(&mut anchors, c1x, c1y, c2x, c2y, x, y, ox, oy);
                cx = x;
                cy = y;
                last_quad_ctrl = Some((qx, qy));
                last_cubic_ctrl = None;
            }
            b'A' => {
                // Elliptical arc — v1 degrades to a straight segment to
                // the endpoint (args[5], args[6]).
                let (x, y) = abs_pt(rel, cx, cy, args[5], args[6]);
                cx = x;
                cy = y;
                push_anchor(&mut anchors, x, y);
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            _ => {}
        }
    }
    // Flush the trailing subpath (no terminating `M` to flush it).
    if anchors.len() >= 2 {
        subpaths.push((anchors, closed));
    }
    subpaths
}

/// Resolve a possibly-relative point against the current pen pos.
fn abs_pt(rel: bool, cx: f64, cy: f64, x: f64, y: f64) -> (f64, f64) {
    if rel {
        (cx + x, cy + y)
    } else {
        (x, y)
    }
}

/// Convert a quadratic control point to the two cubic controls.
fn quad_to_cubic(x0: f64, y0: f64, qx: f64, qy: f64, x1: f64, y1: f64) -> (f64, f64, f64, f64) {
    (
        x0 + 2.0 / 3.0 * (qx - x0),
        y0 + 2.0 / 3.0 * (qy - y0),
        x1 + 2.0 / 3.0 * (qx - x1),
        y1 + 2.0 / 3.0 * (qy - y1),
    )
}

/// Append a cubic-curve segment, **preserving the bezier handles**
/// so the canvas painter's `flatten_path` redraws the smooth curve
/// at paint time. The previous anchor gets `handle_out = c1 − p0`;
/// the new anchor (endpoint) gets `handle_in = c2 − p3`. Both stored
/// as anchor-relative deltas — `path_anchor_bounds` + the layout-scene
/// builder agree on the relative convention.
///
/// Earlier this flattened cubics into 24 straight anchors at import
/// time, which dropped curve fidelity entirely. The canvas painter's
/// `flatten_path` already handles handles correctly, so flattening
/// here was both lossy and redundant.
// Each control point + endpoint + offset is its own scalar — bundling
// them into a struct would only obscure a flat geometric signature.
#[allow(clippy::too_many_arguments)]
fn emit_cubic(
    anchors: &mut Vec<PenPathAnchor>,
    c1x: f64,
    c1y: f64,
    c2x: f64,
    c2y: f64,
    x: f64,
    y: f64,
    ox: f64,
    oy: f64,
) {
    let (p0x, p0y) = match anchors.last() {
        Some(a) => (a.x, a.y),
        None => return,
    };
    let p3x = x + ox;
    let p3y = y + oy;
    if let Some(last) = anchors.last_mut() {
        last.handle_out = Some(PenPathHandle {
            x: c1x + ox - p0x,
            y: c1y + oy - p0y,
        });
    }
    anchors.push(PenPathAnchor {
        x: p3x,
        y: p3y,
        handle_in: Some(PenPathHandle {
            x: c2x + ox - p3x,
            y: c2y + oy - p3y,
        }),
        handle_out: None,
        point_type: None,
    });
}

/// A path-`d` lexer token.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PathToken {
    Cmd(u8),
    Num(f64),
}

/// Tokenize a path `d` string into command letters + numbers.
fn tokenize_path(d: &str) -> Vec<PathToken> {
    let bytes = d.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() {
            out.push(PathToken::Cmd(c));
            i += 1;
        } else if c == b'-' || c == b'+' || c == b'.' || c.is_ascii_digit() {
            let start = i;
            i += 1;
            let mut seen_dot = c == b'.';
            let mut seen_exp = false;
            while i < bytes.len() {
                let dch = bytes[i];
                if dch.is_ascii_digit() {
                    i += 1;
                } else if dch == b'.' && !seen_dot && !seen_exp {
                    seen_dot = true;
                    i += 1;
                } else if (dch == b'e' || dch == b'E') && !seen_exp {
                    seen_exp = true;
                    i += 1;
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(n) = d[start..i].parse::<f64>() {
                out.push(PathToken::Num(n));
            }
        } else {
            i += 1; // comma / whitespace separator
        }
    }
    out
}

/// Parse an SVG `fill` value into a `#rrggbb` hex string. `none` /
/// `transparent` and unparseable values return `None` (no fill).
fn parse_svg_color(raw: &str) -> Option<String> {
    let v = raw.trim().to_ascii_lowercase();
    if v.is_empty() || v == "none" || v == "transparent" {
        return None;
    }
    if let Some(hex) = v.strip_prefix('#') {
        return match hex.len() {
            3 => {
                let mut out = String::with_capacity(7);
                out.push('#');
                for ch in hex.chars() {
                    out.push(ch);
                    out.push(ch);
                }
                Some(out)
            }
            6 | 8 => Some(format!("#{}", &hex[..6])),
            _ => None,
        };
    }
    // Minimal named-colour table — the common SVG presentation names.
    let named = match v.as_str() {
        "black" => "#000000",
        "white" => "#ffffff",
        "red" => "#ff0000",
        "green" => "#008000",
        "lime" => "#00ff00",
        "blue" => "#0000ff",
        "yellow" => "#ffff00",
        "cyan" | "aqua" => "#00ffff",
        "magenta" | "fuchsia" => "#ff00ff",
        "gray" | "grey" => "#808080",
        "silver" => "#c0c0c0",
        "orange" => "#ffa500",
        "purple" => "#800080",
        "navy" => "#000080",
        "teal" => "#008080",
        _ => return None,
    };
    Some(named.to_string())
}
