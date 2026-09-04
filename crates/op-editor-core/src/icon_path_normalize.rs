//! Deterministic normalization for small, hand-authored icon paths.
//!
//! The editor owns the document mutation, while the runtime icon catalogue
//! stays in `op-editor-ui`. Callers inject the catalogue lookup so this module
//! remains usable by the wasm editor core and by headless hosts.

use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::svg_path_data_bounds;
use jian_ops_schema::node::{IconFontNode, PathNode, PenNode};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{PenFill, PenStroke, StrokeThickness};

const ICON_VIEWBOX: f64 = 24.0;
const MAX_ICON_DIMENSION: f64 = 40.0;
/// A path filling at least this fraction of its own box on both axes is
/// already in local px and is left alone.
const ALREADY_FITS_RATIO: f64 = 0.85;
const REFIT_MARKER: &str = "openpencil:icon-path-refit";

/// Counts the two possible normalizations made by [`normalize_icon_paths`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IconPathNormalizeReport {
    pub converted_to_icon_font: usize,
    pub refit_uniform: usize,
}

/// Normalize every icon-shaped path on the active page.
///
/// The lookup callback receives the path in the ordinary whitespace/comma
/// normalized form first, then in a semantic absolute form when the first
/// lookup misses. It must return the static kebab-case catalogue name.
pub fn normalize_icon_paths<F>(state: &mut EditorState, lookup: F) -> IconPathNormalizeReport
where
    F: Fn(&str) -> Option<&'static str>,
{
    normalize_icon_paths_in_nodes(state.active_children_mut(), &lookup)
}

/// Normalize a detached subtree with the same rules as the active-page pass.
/// Orchestrator sinks use this before sending an `InsertSubtree` over their
/// transport, because a remote sink's editor mirror is intentionally read-only
/// to the worker.
pub fn normalize_icon_paths_in_nodes<F>(nodes: &mut [PenNode], lookup: F) -> IconPathNormalizeReport
where
    F: Fn(&str) -> Option<&'static str>,
{
    let mut report = IconPathNormalizeReport::default();
    normalize_icon_paths_in_nodes_with_report(nodes, &lookup, &mut report);
    report
}

fn normalize_icon_paths_in_nodes_with_report<F>(
    nodes: &mut [PenNode],
    lookup: &F,
    report: &mut IconPathNormalizeReport,
) where
    F: Fn(&str) -> Option<&'static str>,
{
    for node in nodes {
        // The product's own status bar is authored in local px and is never
        // model output; its subtree is skipped wholesale, the same rule every
        // section heuristic follows.
        if is_status_bar_node(node) {
            continue;
        }
        let change = match node {
            PenNode::Path(path) => normalize_path(path, lookup),
            _ => None,
        };
        match change {
            Some(PathNormalization::IconFont(icon)) => {
                *node = PenNode::IconFont(*icon);
                report.converted_to_icon_font += 1;
                continue;
            }
            Some(PathNormalization::Refit { d, scale }) => {
                if let PenNode::Path(path) = node {
                    path.d = Some(d);
                    scale_stroke(path.stroke.as_mut(), scale);
                    path.icon_id = Some(match path.icon_id.take() {
                        Some(icon_id) => format!("{icon_id}|{REFIT_MARKER}"),
                        None => REFIT_MARKER.to_string(),
                    });
                    report.refit_uniform += 1;
                }
            }
            None => {}
        }
        if let Some(children) = node.children_mut() {
            normalize_icon_paths_in_nodes_with_report(children, lookup, report);
        }
    }
}

/// Semantic path canonicalization shared with the UI catalogue reverse map.
/// It makes equivalent relative, implicit, comma-separated, and explicit
/// command spellings compare equal while retaining arc rotation and flags.
pub fn canonicalize_path_d(d: &str) -> Option<String> {
    let segments = absolute_segments(d)?;
    Some(serialize_segments(&segments, 1.0, 0.0, 0.0))
}

enum PathNormalization {
    IconFont(Box<IconFontNode>),
    Refit { d: String, scale: f64 },
}

fn normalize_path<F>(path: &PathNode, lookup: &F) -> Option<PathNormalization>
where
    F: Fn(&str) -> Option<&'static str>,
{
    if path.icon_id.as_deref().is_some_and(is_refit_marker)
        || path.anchors.as_ref().is_some_and(|a| !a.is_empty())
    {
        return None;
    }
    let d = path.d.as_deref()?.trim();
    if d.is_empty() {
        return None;
    }
    let width = numeric_dimension(path.width.as_ref())?;
    let height = numeric_dimension(path.height.as_ref())?;
    if width > MAX_ICON_DIMENSION || height > MAX_ICON_DIMENSION {
        return None;
    }
    let (min_x, min_y, path_width, path_height) = svg_path_data_bounds(d)?;
    let min_x = f64::from(min_x);
    let min_y = f64::from(min_y);
    let path_width = f64::from(path_width);
    let path_height = f64::from(path_height);
    let max_x = min_x + path_width;
    let max_y = min_y + path_height;
    if ![min_x, min_y, path_width, path_height, max_x, max_y]
        .into_iter()
        .all(f64::is_finite)
        || min_x < -1.0
        || min_y < -1.0
        || max_x > 25.0
        || max_y > 25.0
    {
        return None;
    }

    // An exact lucide glyph is unambiguous whatever box it sits in.
    if let Some(name) = lookup_icon_name(d, lookup) {
        return Some(PathNormalization::IconFont(Box::new(icon_font_from_path(
            path, name,
        ))));
    }

    // A path whose tight bounds already fill its own box is authored in
    // local px (the status bar's signal / wifi / battery glyphs, an imported
    // SVG fragment) — refitting it would shrink it by min(w,h)/24. Only a
    // glyph clearly smaller than its box is a pasted 24-unit icon.
    if path_width >= width * ALREADY_FITS_RATIO && path_height >= height * ALREADY_FITS_RATIO {
        return None;
    }

    let scale = width.min(height) / ICON_VIEWBOX;
    let offset_x = (width - width.min(height)) / 2.0;
    let offset_y = (height - width.min(height)) / 2.0;
    let segments = absolute_segments(d)?;
    Some(PathNormalization::Refit {
        d: serialize_segments(&segments, scale, offset_x, offset_y),
        scale,
    })
}

fn lookup_icon_name<F>(d: &str, lookup: &F) -> Option<&'static str>
where
    F: Fn(&str) -> Option<&'static str>,
{
    let normalized = normalize_path_text(d);
    if let Some(name) = lookup(&normalized) {
        return Some(name);
    }
    let canonical = canonicalize_path_d(d)?;
    (canonical != normalized)
        .then(|| lookup(&canonical))
        .flatten()
}

fn normalize_path_text(d: &str) -> String {
    let mut out = String::new();
    for token in d
        .trim()
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
    {
        if token.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(token);
    }
    out
}

fn numeric_dimension(value: Option<&SizingBehavior>) -> Option<f64> {
    match value {
        Some(SizingBehavior::Number(value))
            if value.is_finite() && *value >= 0.0 && *value <= MAX_ICON_DIMENSION =>
        {
            Some(*value)
        }
        _ => None,
    }
}

fn icon_font_from_path(path: &PathNode, name: &str) -> IconFontNode {
    let fill = first_solid_fill(
        path.stroke
            .as_ref()
            .and_then(|stroke| stroke.fill.as_deref()),
    )
    .or_else(|| first_solid_fill(path.fill.as_deref()));
    IconFontNode {
        base: path.base.clone(),
        icon_font_name: name.to_string(),
        icon_font_family: Some("lucide".to_string()),
        width: path.width.clone(),
        height: path.height.clone(),
        limits: path.limits,
        fill,
        stroke: None,
        state: path.state.clone(),
        bindings: path.bindings.clone(),
        events: path.events.clone(),
        lifecycle: path.lifecycle.clone(),
        semantics: path.semantics.clone(),
        gestures: path.gestures.clone(),
        route: path.route.clone(),
    }
}

fn is_status_bar_node(node: &PenNode) -> bool {
    node.base()
        .role
        .as_deref()
        .is_some_and(|role| role.eq_ignore_ascii_case("status-bar"))
}

fn is_refit_marker(icon_id: &str) -> bool {
    icon_id == REFIT_MARKER
        || icon_id
            .strip_suffix(REFIT_MARKER)
            .is_some_and(|prefix| prefix.ends_with('|'))
}

fn first_solid_fill(fills: Option<&[PenFill]>) -> Option<Vec<PenFill>> {
    fills
        .and_then(|fills| fills.iter().find(|fill| matches!(fill, PenFill::Solid(_))))
        .cloned()
        .map(|fill| vec![fill])
}

fn scale_stroke(stroke: Option<&mut PenStroke>, scale: f64) {
    let Some(stroke) = stroke else {
        return;
    };
    match &mut stroke.thickness {
        StrokeThickness::Uniform(value) => *value = scaled_thickness(*value, scale),
        StrokeThickness::PerSide(values) => {
            for value in values {
                *value = scaled_thickness(*value, scale);
            }
        }
        StrokeThickness::Sided(values) => {
            for value in [
                &mut values.top,
                &mut values.right,
                &mut values.bottom,
                &mut values.left,
            ]
            .into_iter()
            .flatten()
            {
                *value = scaled_thickness(*value, scale);
            }
        }
    }
}

fn scaled_thickness(value: f32, scale: f64) -> f32 {
    ((f64::from(value) * scale).max(1.0) * 10.0).round() as f32 / 10.0
}

#[derive(Debug, Clone, Copy)]
enum Token {
    Command(u8),
    Number(f64),
}

#[derive(Debug, Clone, Copy)]
enum Segment {
    Move(f64, f64),
    Line(f64, f64),
    Cubic(f64, f64, f64, f64, f64, f64),
    Quad(f64, f64, f64, f64),
    Arc(f64, f64, f64, f64, f64, f64, f64),
    Close,
}

fn absolute_segments(d: &str) -> Option<Vec<Segment>> {
    let tokens = tokenize(d)?;
    let mut out = Vec::new();
    let mut index = 0;
    let mut command = None;
    let (mut cx, mut cy) = (0.0, 0.0);
    let (mut sx, mut sy) = (0.0, 0.0);
    let mut last_cubic = None;
    let mut last_quad = None;

    while index < tokens.len() {
        if let Token::Command(next) = tokens[index] {
            command = Some(next);
            index += 1;
        }
        let command_byte = command?;
        let upper = command_byte.to_ascii_uppercase();
        if upper == b'Z' {
            out.push(Segment::Close);
            cx = sx;
            cy = sy;
            command = None;
            last_cubic = None;
            last_quad = None;
            continue;
        }
        let count = parameter_count(upper)?;
        let mut args = [0.0; 7];
        for arg in args.iter_mut().take(count) {
            let Token::Number(value) = *tokens.get(index)? else {
                return None;
            };
            *arg = value;
            index += 1;
        }
        let relative = command_byte.is_ascii_lowercase();
        match upper {
            b'M' => {
                let (x, y) = point(relative, cx, cy, args[0], args[1]);
                out.push(Segment::Move(x, y));
                (cx, cy) = (x, y);
                (sx, sy) = (x, y);
                command = Some(if relative { b'l' } else { b'L' });
                last_cubic = None;
                last_quad = None;
            }
            b'L' => {
                let (x, y) = point(relative, cx, cy, args[0], args[1]);
                out.push(Segment::Line(x, y));
                (cx, cy) = (x, y);
                last_cubic = None;
                last_quad = None;
            }
            b'H' => {
                let x = if relative { cx + args[0] } else { args[0] };
                out.push(Segment::Line(x, cy));
                cx = x;
                last_cubic = None;
                last_quad = None;
            }
            b'V' => {
                let y = if relative { cy + args[0] } else { args[0] };
                out.push(Segment::Line(cx, y));
                cy = y;
                last_cubic = None;
                last_quad = None;
            }
            b'C' => {
                let (x1, y1) = point(relative, cx, cy, args[0], args[1]);
                let (x2, y2) = point(relative, cx, cy, args[2], args[3]);
                let (x, y) = point(relative, cx, cy, args[4], args[5]);
                out.push(Segment::Cubic(x1, y1, x2, y2, x, y));
                (cx, cy) = (x, y);
                last_cubic = Some((x2, y2));
                last_quad = None;
            }
            b'S' => {
                let (x1, y1) = last_cubic
                    .map(|(x2, y2)| (2.0 * cx - x2, 2.0 * cy - y2))
                    .unwrap_or((cx, cy));
                let (x2, y2) = point(relative, cx, cy, args[0], args[1]);
                let (x, y) = point(relative, cx, cy, args[2], args[3]);
                out.push(Segment::Cubic(x1, y1, x2, y2, x, y));
                (cx, cy) = (x, y);
                last_cubic = Some((x2, y2));
                last_quad = None;
            }
            b'Q' => {
                let (qx, qy) = point(relative, cx, cy, args[0], args[1]);
                let (x, y) = point(relative, cx, cy, args[2], args[3]);
                out.push(Segment::Quad(qx, qy, x, y));
                (cx, cy) = (x, y);
                last_quad = Some((qx, qy));
                last_cubic = None;
            }
            b'T' => {
                let (qx, qy) = last_quad
                    .map(|(px, py)| (2.0 * cx - px, 2.0 * cy - py))
                    .unwrap_or((cx, cy));
                let (x, y) = point(relative, cx, cy, args[0], args[1]);
                out.push(Segment::Quad(qx, qy, x, y));
                (cx, cy) = (x, y);
                last_quad = Some((qx, qy));
                last_cubic = None;
            }
            b'A' => {
                let (x, y) = point(relative, cx, cy, args[5], args[6]);
                out.push(Segment::Arc(
                    args[0], args[1], args[2], args[3], args[4], x, y,
                ));
                (cx, cy) = (x, y);
                last_cubic = None;
                last_quad = None;
            }
            _ => return None,
        }
    }
    (!out.is_empty()).then_some(out)
}

fn parameter_count(command: u8) -> Option<usize> {
    Some(match command {
        b'M' | b'L' | b'T' => 2,
        b'H' | b'V' => 1,
        b'C' => 6,
        b'S' | b'Q' => 4,
        b'A' => 7,
        _ => return None,
    })
}

fn point(relative: bool, cx: f64, cy: f64, x: f64, y: f64) -> (f64, f64) {
    if relative {
        (cx + x, cy + y)
    } else {
        (x, y)
    }
}

fn tokenize(d: &str) -> Option<Vec<Token>> {
    let bytes = d.as_bytes();
    let mut index = 0;
    let mut tokens = Vec::new();
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() || bytes[index] == b',' {
            index += 1;
        } else if is_path_command(bytes[index]) {
            tokens.push(Token::Command(bytes[index]));
            index += 1;
        } else {
            tokens.push(Token::Number(parse_number(bytes, &mut index)?));
        }
    }
    Some(tokens)
}

fn is_path_command(byte: u8) -> bool {
    matches!(
        byte,
        b'M' | b'm'
            | b'L'
            | b'l'
            | b'H'
            | b'h'
            | b'V'
            | b'v'
            | b'C'
            | b'c'
            | b'S'
            | b's'
            | b'Q'
            | b'q'
            | b'T'
            | b't'
            | b'A'
            | b'a'
            | b'Z'
            | b'z'
    )
}

fn parse_number(bytes: &[u8], index: &mut usize) -> Option<f64> {
    let start = *index;
    if bytes
        .get(*index)
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
    {
        *index += 1;
    }
    let digits_start = *index;
    while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
        *index += 1;
    }
    if bytes.get(*index) == Some(&b'.') {
        *index += 1;
        while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
            *index += 1;
        }
    }
    if *index == digits_start {
        return None;
    }
    if bytes
        .get(*index)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        *index += 1;
        if bytes
            .get(*index)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            *index += 1;
        }
        let exponent_start = *index;
        while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
            *index += 1;
        }
        if *index == exponent_start {
            return None;
        }
    }
    std::str::from_utf8(&bytes[start..*index])
        .ok()?
        .parse()
        .ok()
}

fn serialize_segments(segments: &[Segment], scale: f64, offset_x: f64, offset_y: f64) -> String {
    let mut out = String::new();
    let map = |x: f64, y: f64| (x * scale + offset_x, y * scale + offset_y);
    for segment in segments {
        match *segment {
            Segment::Move(x, y) => {
                let (x, y) = map(x, y);
                append_values(&mut out, "M", &[x, y]);
            }
            Segment::Line(x, y) => {
                let (x, y) = map(x, y);
                append_values(&mut out, "L", &[x, y]);
            }
            Segment::Cubic(x1, y1, x2, y2, x, y) => {
                let (x1, y1) = map(x1, y1);
                let (x2, y2) = map(x2, y2);
                let (x, y) = map(x, y);
                append_values(&mut out, "C", &[x1, y1, x2, y2, x, y]);
            }
            Segment::Quad(qx, qy, x, y) => {
                let (qx, qy) = map(qx, qy);
                let (x, y) = map(x, y);
                append_values(&mut out, "Q", &[qx, qy, x, y]);
            }
            Segment::Arc(rx, ry, rotation, large, sweep, x, y) => {
                let (x, y) = map(x, y);
                append_values(
                    &mut out,
                    "A",
                    &[
                        rx * scale.abs(),
                        ry * scale.abs(),
                        rotation,
                        if large != 0.0 { 1.0 } else { 0.0 },
                        if sweep != 0.0 { 1.0 } else { 0.0 },
                        x,
                        y,
                    ],
                );
            }
            Segment::Close => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push('Z');
            }
        }
    }
    out
}

fn append_values(out: &mut String, command: &str, values: &[f64]) {
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(command);
    for value in values {
        out.push(' ');
        out.push_str(&format_number(*value));
    }
}

fn format_number(value: f64) -> String {
    let value = if value.abs() < 0.0000005 { 0.0 } else { value };
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}
