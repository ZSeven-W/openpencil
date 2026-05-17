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
use jian_ops_schema::node::{PathNode, PenNode, PenNodeBase, PenPathAnchor};
use jian_ops_schema::sizing::SizingBehavior;

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

impl EditorState {
    /// Parse `svg` and insert the resulting nodes onto the active page,
    /// translated by `offset` doc-px. Returns the count of nodes
    /// inserted; `0` (no history pushed) when the SVG yields nothing.
    /// One history snapshot is pushed when ≥ 1 node lands.
    pub fn import_svg(&mut self, next_id: &mut u64, svg: &str, offset: (f64, f64)) -> usize {
        let elements = parse_svg_elements(svg);
        if elements.is_empty() {
            return 0;
        }
        // Seed the id allocator past the live id space so imported
        // nodes never collide with existing ones.
        if let Some(safe) = self.max_node_id().checked_add(1) {
            *next_id = (*next_id).max(safe);
        }
        let mut taken = self.collect_node_ids();
        let mut built: Vec<PenNode> = Vec::new();
        for el in &elements {
            let Some(id) = walkers::alloc_n_id(next_id, &mut taken) else {
                break; // id space exhausted — keep whatever was built
            };
            match element_to_node(el, id.clone(), offset) {
                Some(node) => built.push(node),
                None => {
                    // Unsupported / empty element — return the id to
                    // the allocator pool so the next element reuses it.
                    taken.remove(&id);
                }
            }
        }
        if built.is_empty() {
            return 0;
        }
        let pre = self.snapshot_for_history();
        let count = built.len();
        self.active_children_mut().extend(built);
        self.history_push_past(pre);
        count
    }
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
            let d = el.attr("d")?;
            let (anchors, closed) = parse_path_d(d, offset);
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
/// Flat scan of every `<tag …>` / `<tag … />` element in `svg`.
/// Comments, the XML prolog, closing tags and DOCTYPE are skipped;
/// nesting is ignored, so a shape inside a `<g>` is still found.
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

/// Parse an SVG path `d` string into `PenPathAnchor`s. Supports the
/// `M L H V C S Q T Z` commands (absolute + relative). `A` degrades to
/// a straight segment to its endpoint. Returns `(anchors, closed)`.
fn parse_path_d(d: &str, offset: (f64, f64)) -> (Vec<PenPathAnchor>, bool) {
    let tokens = tokenize_path(d);
    let (ox, oy) = offset;
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
    (anchors, closed)
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

/// Sample count when flattening a cubic-Bezier segment to straight
/// anchors. 24 is visually smooth without bloating the anchor list.
const CUBIC_FLATTEN_STEPS: usize = 24;

/// Append a cubic-curve segment as **dense straight-line anchors**.
///
/// The canvas painter, the export renderer and the pen-tool anchor
/// hit-test all model a Path as a straight-segment polyline whose
/// `points` are 1:1 with its anchors. Storing bezier handles instead
/// would either be dropped by the renderer (curve lost) or desync the
/// pen-tool anchor index — so the curve is flattened here, at import
/// time, into `CUBIC_FLATTEN_STEPS` straight anchors that trace it.
/// `c1`/`c2` are the SVG control points, `(x, y)` the endpoint,
/// `(ox, oy)` the document offset; the previous anchor is the start.
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
    // The start point is the last anchor (already offset-applied).
    let Some(&PenPathAnchor { x: p0x, y: p0y, .. }) = anchors.last() else {
        return;
    };
    let (p1x, p1y) = (c1x + ox, c1y + oy);
    let (p2x, p2y) = (c2x + ox, c2y + oy);
    let (p3x, p3y) = (x + ox, y + oy);
    for s in 1..=CUBIC_FLATTEN_STEPS {
        let t = s as f64 / CUBIC_FLATTEN_STEPS as f64;
        anchors.push(PenPathAnchor {
            x: cubic_eval(p0x, p1x, p2x, p3x, t),
            y: cubic_eval(p0y, p1y, p2y, p3y, t),
            handle_in: None,
            handle_out: None,
            point_type: None,
        });
    }
}

/// One axis of a cubic Bezier evaluated at parameter `t`.
fn cubic_eval(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let mt = 1.0 - t;
    mt * mt * mt * p0 + 3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t * p3
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
