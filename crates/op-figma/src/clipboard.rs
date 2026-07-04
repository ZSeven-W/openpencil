//! Figma HTML clipboard parsing — ports `pen-figma/figma-clipboard.ts`.
//!
//! Figma's Cmd+C-on-canvas writes an HTML clipboard payload that wraps
//! a base64-encoded `.fig`-kiwi binary buffer + a JSON meta blob in
//! HTML comments / data-attributes. This module decodes that payload
//! into a flat `Vec<PenNode>` matching TS `figmaClipboardToNodes` —
//! including the post-process steps that swap unresolvable
//! `__blob:` / `__hash:` references for grey placeholder fills and
//! enrich text nodes with inline CSS hints recovered from the HTML.
//!
//! The `.fig.json` clipboard format (JSON wrapper, distinct from this
//! HTML format) lives in `lib.rs::extract_clipboard_nodes` and is
//! unrelated.
//!
//! Host integration: a desktop paste handler can call
//! [`is_figma_clipboard_html`] on the system clipboard's HTML payload,
//! then [`figma_clipboard_to_nodes`] to obtain insertable PenNodes.

use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::{
    EllipseNode, FrameNode, GroupNode, ImageNode, LineNode, PathNode, PenNode, PenNodeBase,
    PolygonNode, RectangleNode, RefNode, TextContent, TextInputNode, TextNode,
};
use jian_ops_schema::style::{PenFill, SolidFillBody};
use std::collections::HashMap;

use crate::common::FigLayoutMode;
use crate::figma_types;
use crate::image_resolver::resolve_image_blobs;
use crate::node_mapper::{figma_node_changes_to_pen_nodes, FigmaClipboardResult};

/// Decoded Figma HTML clipboard payload — the JSON `meta` text plus
/// the binary `.fig`-kiwi `buffer`. The caller can opt into parsing
/// `meta` with `serde_json` (kept as `String` here so the crate
/// doesn't pull serde_json at runtime).
#[derive(Debug, Clone)]
pub struct FigmaClipboardData {
    /// Raw JSON text of the figmeta block (caller parses if needed).
    pub meta: String,
    /// The decoded `.fig`-kiwi binary buffer.
    pub buffer: Vec<u8>,
}

/// Quick HTML-prefix check for the Figma clipboard. Matches TS
/// `isFigmaClipboardHtml` — looks for either the `figmeta` comment
/// marker or the `data-buffer` attribute, whichever wrapper Figma is
/// using on the platform.
pub fn is_figma_clipboard_html(html: &str) -> bool {
    html.contains("figmeta") || html.contains("data-buffer")
}

/// Extract Figma clipboard data from an HTML payload. Mirrors TS
/// `extractFigmaClipboardData` — tries three wrapper formats in order:
///   1. `<!--(figmeta)-->BASE64<!--(figmeta)-->` comment blocks
///   2. `data-metadata="…"` / `data-buffer="…"` attribute pairs
///   3. HTML-entity-encoded comment markers
///
/// Returns `None` on any unrecognised / malformed input.
pub fn extract_figma_clipboard_data(html: &str) -> Option<FigmaClipboardData> {
    let (mut meta_b64, mut buf_b64) = (None::<String>, None::<String>);

    // Strategy 1: bare comment-wrapped form.
    if let (Some(m), Some(b)) = (
        scan_between(html, "<!--(figmeta)", "<!--(figmeta)-->"),
        scan_between(html, "<!--(figma)", "<!--(figma)-->"),
    ) {
        meta_b64 = Some(strip_opening_marker(m).trim().to_string());
        buf_b64 = Some(strip_opening_marker(b).trim().to_string());
    }

    // Strategy 2: `data-metadata` / `data-buffer` attributes — comment
    // markers may live INSIDE the attribute value.
    if meta_b64.is_none() || buf_b64.is_none() {
        if let (Some(m), Some(b)) = (
            scan_attr(html, "data-metadata"),
            scan_attr(html, "data-buffer"),
        ) {
            meta_b64 = Some(strip_embedded_markers(m, "figmeta").trim().to_string());
            buf_b64 = Some(strip_embedded_markers(b, "figma").trim().to_string());
        }
    }

    // Strategy 3: HTML-entity-encoded comment markers.
    if meta_b64.is_none() || buf_b64.is_none() {
        if let (Some(m), Some(b)) = (
            scan_between(html, "&lt;!--(figmeta)--&gt;", "&lt;!--(figmeta)--&gt;"),
            scan_between(html, "&lt;!--(figma)--&gt;", "&lt;!--(figma)--&gt;"),
        ) {
            meta_b64 = Some(m.trim().to_string());
            buf_b64 = Some(b.trim().to_string());
        }
    }

    let meta_b64 = meta_b64?;
    let buf_b64 = buf_b64?;

    let meta_raw = String::from_utf8(decode_base64(&meta_b64)?).ok()?;
    // Trim any trailing padding-induced junk past the JSON object.
    let json_end = meta_raw.rfind('}').map(|i| i + 1).unwrap_or(meta_raw.len());
    let meta = meta_raw[..json_end].to_string();
    let buffer = decode_base64(&buf_b64)?;
    Some(FigmaClipboardData { meta, buffer })
}

/// Drive a clipboard buffer through the same binary parser used for
/// `.fig` files, plus the two clipboard-specific post-process passes:
/// `fix_unresolved_images` (swap `__blob:` / `__hash:` references for
/// grey placeholders) and — when HTML is supplied — `enrich_*` (copy
/// missing color/font hints from inline CSS in the HTML). Mirrors TS
/// `figmaClipboardToNodes`.
pub fn figma_clipboard_to_nodes(buffer: &[u8], html: Option<&str>) -> FigmaClipboardResult {
    let decoded = match figma_types::parse_fig_file(buffer) {
        Ok(d) => d,
        Err(e) => {
            return FigmaClipboardResult {
                nodes: Vec::new(),
                warnings: vec![format!("Clipboard parse failed: {e}")],
                image_blobs: HashMap::new(),
            };
        }
    };
    let image_files = decoded.image_files.clone();
    // Clipboard uses the same `preserve` mode the TS clipboard path
    // uses — keeps auto-layout flow + numeric pixel sizes on images.
    let mut out = figma_node_changes_to_pen_nodes(decoded, FigLayoutMode::Preserve);

    if !out.image_blobs.is_empty() || !image_files.is_empty() {
        let mut doc = build_doc_for_resolve(out.nodes.clone());
        resolve_image_blobs(&mut doc, &out.image_blobs, &image_files);
        out.nodes = extract_doc_children(doc);
    }

    fix_unresolved_images(&mut out.nodes);

    if let Some(html) = html {
        let hints = parse_clipboard_html_styles(html);
        if !hints.is_empty() {
            enrich_nodes_from_html_hints(&mut out.nodes, &hints);
        }
    }

    out
}

// ── Base64 ──────────────────────────────────────────────────────────

/// Tolerant base64 decode — strips non-alphabet bytes (`\n`, `\r`,
/// `\t`, ` `), normalises URL-safe `-` / `_`, accepts missing or
/// extra `=` padding. Matches TS `decodeBase64ToBytes` accept-set.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut cleaned = String::with_capacity(input.len());
    for c in input.chars() {
        if matches!(
            c,
            'A'..='Z' | 'a'..='z' | '0'..='9' | '+' | '/' | '-' | '_' | '='
        ) {
            // Normalise URL-safe.
            cleaned.push(match c {
                '-' => '+',
                '_' => '/',
                other => other,
            });
        }
    }
    // Strip trailing padding so STANDARD_NO_PAD accepts; the engine
    // handles missing padding fine. Manual trim avoids the decoder
    // bailing on `==` count mismatches.
    while cleaned.ends_with('=') {
        cleaned.pop();
    }
    STANDARD_NO_PAD.decode(cleaned.as_bytes()).ok()
}

// ── HTML scanning helpers ───────────────────────────────────────────

/// Find the first slice between `start` (anywhere) and a subsequent
/// `end`. Returns the inner slice without trimming.
fn scan_between<'a>(haystack: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = haystack.find(start)?;
    let after = &haystack[i + start.len()..];
    let j = after.find(end)?;
    Some(&after[..j])
}

/// Find the first `<attr>="VALUE"` slice. The attribute name comes
/// verbatim (no escaping needed for the names we handle).
fn scan_attr<'a>(haystack: &'a str, attr: &str) -> Option<&'a str> {
    let needle = format!("{attr}=\"");
    let i = haystack.find(&needle)?;
    let after = &haystack[i + needle.len()..];
    let j = after.find('"')?;
    Some(&after[..j])
}

/// Drop the leading `-->` from a comment-wrapped opener — Figma
/// sometimes emits `<!--(figmeta)BASE64<!--(figmeta)-->` (opening
/// lacks the `-->`), so we accept both forms by stripping at most one
/// occurrence.
fn strip_opening_marker(s: &str) -> &str {
    s.strip_prefix("-->").unwrap_or(s)
}

/// Strip embedded comment markers from inside an attribute value —
/// e.g. `data-metadata="<!--(figmeta)BASE64<!--(figmeta)-->"` becomes
/// the bare BASE64 payload.
fn strip_embedded_markers(s: &str, kind: &str) -> String {
    let opener = format!("<!--({kind})");
    let opener_full = format!("<!--({kind})-->");
    let mut work = s.to_string();
    work = work.replace(&opener_full, "");
    work = work.replace(&opener, "");
    work
}

// ── CSS color + entities ────────────────────────────────────────────

/// Normalize a CSS color literal to `#rrggbb` (or `#rrggbbaa` when
/// `rgba(...)` carries a non-`1` alpha). Returns None for shapes we
/// don't recognise (named colors, hsl(), etc.).
fn css_color_to_hex(css: &str) -> Option<String> {
    let c = css.trim();
    if let Some(rest) = c.strip_prefix('#') {
        return Some(match rest.len() {
            3 => {
                let b = rest.as_bytes();
                format!(
                    "#{0}{0}{1}{1}{2}{2}",
                    b[0] as char, b[1] as char, b[2] as char
                )
            }
            _ => c.to_string(),
        });
    }
    let rest = c.strip_prefix("rgba(").or_else(|| c.strip_prefix("rgb("))?;
    let rest = rest.strip_suffix(')')?;
    let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return None;
    }
    let r: u32 = parts[0].parse().ok()?;
    let g: u32 = parts[1].parse().ok()?;
    let b: u32 = parts[2].parse().ok()?;
    if let Some(a_str) = parts.get(3) {
        let a_f: f32 = a_str.parse().ok()?;
        let a = (a_f * 255.0).round() as u32;
        if a < 255 {
            return Some(format!("#{r:02x}{g:02x}{b:02x}{a:02x}"));
        }
    }
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

/// Decode HTML entities present in the styled text portions of Figma
/// clipboard markup: numeric `&#NN;` / `&#xHH;` and the common named
/// entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&nbsp;`).
fn decode_html_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // Numeric entity: &#NN; or &#xHH;
        if i + 2 < bytes.len() && bytes[i + 1] == b'#' {
            let hex = bytes[i + 2] == b'x' || bytes[i + 2] == b'X';
            let start = if hex { i + 3 } else { i + 2 };
            let Some(semi_off) = bytes[start..].iter().position(|&b| b == b';') else {
                out.push('&');
                i += 1;
                continue;
            };
            let end = start + semi_off;
            let body = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
            let code = if hex {
                u32::from_str_radix(body, 16).ok()
            } else {
                body.parse::<u32>().ok()
            };
            if let Some(c) = code.and_then(char::from_u32) {
                out.push(c);
                i = end + 1;
                continue;
            }
        }
        // Named entity.
        let rest = &text[i..];
        let mut matched = false;
        for (name, repl) in [
            ("&amp;", '&'),
            ("&lt;", '<'),
            ("&gt;", '>'),
            ("&quot;", '"'),
            ("&nbsp;", ' '),
        ] {
            if rest.starts_with(name) {
                out.push(repl);
                i += name.len();
                matched = true;
                break;
            }
        }
        if !matched {
            out.push('&');
            i += 1;
        }
    }
    out
}

// ── HTML style-hint scanning ────────────────────────────────────────

/// Style hints harvested from the inline CSS inside a Figma clipboard
/// HTML payload. Field shapes mirror TS `HtmlStyleHint`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HtmlStyleHint {
    pub color: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u32>,
    pub background_color: Option<String>,
}

/// Walk an HTML payload, extract `style="…"` + adjacent text content,
/// and build a map keyed by trimmed text → style hint. Used by
/// `enrich_nodes_from_html_hints` to recover style information that
/// got lost in the binary path (commonly: shared-style references the
/// clipboard didn't include the resolved value of).
pub fn parse_clipboard_html_styles(html: &str) -> HashMap<String, HtmlStyleHint> {
    // Strip the binary blocks first so we don't accidentally match
    // base64 payloads.
    let clean = strip_binary_blocks(html);

    let mut hints = HashMap::new();
    let mut cursor = 0usize;
    let bytes = clean.as_bytes();
    while let Some(rel) = clean[cursor..].find("style=\"") {
        let start = cursor + rel + "style=\"".len();
        let Some(end_off) = clean[start..].find('"') else {
            break;
        };
        let end = start + end_off;
        let style_attr = &clean[start..end];

        // Find the immediately following `>TEXT<` pair.
        let after = &clean[end + 1..];
        let Some(gt) = after.find('>') else {
            cursor = end + 1;
            continue;
        };
        let text_start = end + 1 + gt + 1;
        let Some(lt) = clean[text_start..].find('<') else {
            cursor = end + 1;
            continue;
        };
        let text_end = text_start + lt;
        let raw_text = decode_html_entities(&clean[text_start..text_end])
            .trim()
            .to_string();
        cursor = text_end;

        if raw_text.is_empty() || raw_text.len() > 200 {
            continue;
        }

        let mut hint = HtmlStyleHint::default();
        for (key, slot) in [
            ("color:", 0u8),
            ("font-family:", 1),
            ("font-size:", 2),
            ("font-weight:", 3),
            ("background-color:", 4),
        ] {
            if let Some(val) = scan_css_value(style_attr, key) {
                match slot {
                    0 => hint.color = css_color_to_hex(&val),
                    1 => {
                        let family = val
                            .trim()
                            .trim_matches(|c: char| c == '\'' || c == '"')
                            .split(',')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if !family.is_empty() {
                            hint.font_family = Some(family);
                        }
                    }
                    2 => {
                        if let Some(px) = val.strip_suffix("px") {
                            if let Ok(n) = px.trim().parse::<f64>() {
                                hint.font_size = Some(n);
                            }
                        }
                    }
                    3 => {
                        if let Ok(n) = val.trim().parse::<u32>() {
                            hint.font_weight = Some(n);
                        }
                    }
                    4 => hint.background_color = css_color_to_hex(&val),
                    _ => {}
                }
            }
        }

        if hint != HtmlStyleHint::default() {
            hints.entry(raw_text).or_insert(hint);
        }
    }
    // Silence the dead-store warning when no entities matched.
    let _ = bytes;
    hints
}

fn strip_binary_blocks(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    for marker in ["<!--(figmeta)", "<!--(figma)"] {
        while let Some(i) = rest.find(marker) {
            out.push_str(&rest[..i]);
            // Find end-marker after the start.
            let after = &rest[i + marker.len()..];
            let Some(j) = after.find("-->") else {
                rest = "";
                break;
            };
            // Skip from `i` through the end marker (`-->` length 3).
            rest = &after[j + 3..];
            // Second marker may follow for the closing comment; skip
            // until we've consumed the second occurrence too.
            if let Some(k) = rest.find(marker) {
                let after2 = &rest[k + marker.len()..];
                if let Some(l) = after2.find("-->") {
                    rest = &after2[l + 3..];
                }
            }
        }
    }
    out.push_str(rest);
    out
}

/// Pull `<key> <value>;` (or to end-of-attr) from an inline CSS
/// style string. Stops at the first `;` after the value start. The
/// key must start at attribute-start or after `;` + optional
/// whitespace, so that `color:` doesn't false-match
/// `background-color:`.
fn scan_css_value(attr: &str, key: &str) -> Option<String> {
    let mut search_from = 0usize;
    while let Some(rel) = attr[search_from..].find(key) {
        let abs = search_from + rel;
        let prev_ok = if abs == 0 {
            true
        } else {
            let prev = &attr[..abs];
            prev.trim_end_matches([' ', '\t']).ends_with(';')
                || prev.trim_end_matches([' ', '\t']).is_empty()
        };
        let after = abs + key.len();
        if prev_ok {
            let tail = &attr[after..];
            let end = tail.find(';').unwrap_or(tail.len());
            return Some(tail[..end].trim().to_string());
        }
        search_from = abs + key.len();
    }
    None
}

// ── PenNode tree walkers ────────────────────────────────────────────

/// Replace `__blob:` / `__hash:` image refs with placeholder fills /
/// rectangles. Mirrors TS `fixUnresolvedImages`.
pub fn fix_unresolved_images(nodes: &mut [PenNode]) {
    for slot in nodes.iter_mut() {
        if let PenNode::Image(img) = slot {
            if img.src.starts_with("__blob:") || img.src.starts_with("__hash:") {
                // Preserve cornerRadius (and width/height) on the
                // placeholder rectangle so the visual frame stays put.
                // TS `figma-clipboard.ts:192-202` carries
                // `cornerRadius` + `opacity` over; `opacity` lives on
                // `PenNodeBase` and is already inherited via
                // `img.base.clone()`.
                *slot = PenNode::Rectangle(RectangleNode {
                    base: img.base.clone(),
                    container: ContainerProps {
                        width: img.width.clone(),
                        height: img.height.clone(),
                        corner_radius: img.corner_radius.clone(),
                        fill: Some(vec![placeholder_fill()]),
                        ..ContainerProps::default()
                    },
                    children: None,
                    state: None,
                    bindings: None,
                    events: None,
                    lifecycle: None,
                    semantics: None,
                    gestures: None,
                    route: None,
                });
                continue;
            }
        }
        // Fix in-place image fills on non-image variants.
        replace_unresolved_image_fills(slot);
        recurse_children_mut(slot, fix_unresolved_images);
    }
}

fn placeholder_fill() -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: "#E5E7EB".to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

fn replace_unresolved_image_fills(node: &mut PenNode) {
    let fill_slot = node_fill_mut(node);
    if let Some(fill) = fill_slot {
        for slot in fill.iter_mut() {
            if let PenFill::Image(img) = slot {
                if img.url.starts_with("__blob:") || img.url.starts_with("__hash:") {
                    *slot = placeholder_fill();
                }
            }
        }
    }
}

fn node_fill_mut(node: &mut PenNode) -> Option<&mut Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => n.container.fill.as_mut(),
        PenNode::Group(n) => n.container.fill.as_mut(),
        PenNode::Rectangle(n) => n.container.fill.as_mut(),
        PenNode::Ellipse(n) => n.fill.as_mut(),
        PenNode::Polygon(n) => n.fill.as_mut(),
        PenNode::Path(n) => n.fill.as_mut(),
        PenNode::Text(n) => n.fill.as_mut(),
        PenNode::TextInput(n) => n.fill.as_mut(),
        PenNode::IconFont(n) => n.fill.as_mut(),
        _ => None,
    }
}

fn recurse_children_mut(node: &mut PenNode, f: fn(&mut [PenNode])) {
    match node {
        PenNode::Frame(n) => {
            if let Some(c) = n.children.as_mut() {
                f(c);
            }
        }
        PenNode::Group(n) => {
            if let Some(c) = n.children.as_mut() {
                f(c);
            }
        }
        PenNode::Rectangle(n) => {
            if let Some(c) = n.children.as_mut() {
                f(c);
            }
        }
        PenNode::Ref(n) => {
            if let Some(c) = n.children.as_mut() {
                f(c);
            }
        }
        _ => {}
    }
}

/// Copy missing color / font hints from HTML-derived style hints onto
/// `Text` nodes, and `background-color` onto `Frame` / `Rectangle`
/// nodes that lack a fill. Mirrors TS `enrichNodesFromHtmlHints`.
pub fn enrich_nodes_from_html_hints(nodes: &mut [PenNode], hints: &HashMap<String, HtmlStyleHint>) {
    for node in nodes.iter_mut() {
        match node {
            PenNode::Text(t) => enrich_text(t, hints),
            PenNode::Frame(f) => enrich_container_bg(&f.base, &mut f.container, hints),
            PenNode::Rectangle(r) => enrich_container_bg(&r.base, &mut r.container, hints),
            _ => {}
        }
        // Recurse into children.
        let kids: Option<&mut Vec<PenNode>> = match node {
            PenNode::Frame(n) => n.children.as_mut(),
            PenNode::Group(n) => n.children.as_mut(),
            PenNode::Rectangle(n) => n.children.as_mut(),
            PenNode::Ref(n) => n.children.as_mut(),
            _ => None,
        };
        if let Some(c) = kids {
            enrich_nodes_from_html_hints(c, hints);
        }
    }
}

fn enrich_text(t: &mut TextNode, hints: &HashMap<String, HtmlStyleHint>) {
    let content_str = match &t.content {
        TextContent::Plain(s) => s.clone(),
        TextContent::Styled(segs) => segs.iter().map(|s| s.text.clone()).collect::<String>(),
    };
    let trimmed = content_str.trim();
    if trimmed.is_empty() {
        return;
    }
    let hint = hints
        .get(trimmed)
        .or_else(|| find_partial_hint(trimmed, hints));
    let Some(hint) = hint else { return };

    if t.fill.is_none() {
        if let Some(color) = &hint.color {
            t.fill = Some(vec![PenFill::Solid(SolidFillBody {
                color: color.clone(),
                explain: None,
                opacity: None,
                blend_mode: None,
            })]);
        }
    }
    if t.font_family.is_none() {
        if let Some(family) = &hint.font_family {
            t.font_family = Some(family.clone());
        }
    }
    if t.font_size.is_none() {
        if let Some(size) = hint.font_size {
            t.font_size = Some(size);
        }
    }
    if t.font_weight.is_none() {
        if let Some(w) = hint.font_weight {
            t.font_weight = Some(jian_ops_schema::node::text::FontWeight::Number(w));
        }
    }
}

fn enrich_container_bg(
    base: &PenNodeBase,
    container: &mut ContainerProps,
    hints: &HashMap<String, HtmlStyleHint>,
) {
    if container.fill.is_some() {
        return;
    }
    let Some(name) = base.name.as_ref() else {
        return;
    };
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Some(hint) = hints.get(trimmed) {
        if let Some(bg) = &hint.background_color {
            container.fill = Some(vec![PenFill::Solid(SolidFillBody {
                color: bg.clone(),
                explain: None,
                opacity: None,
                blend_mode: None,
            })]);
        }
    }
}

fn find_partial_hint<'a>(
    text: &str,
    hints: &'a HashMap<String, HtmlStyleHint>,
) -> Option<&'a HtmlStyleHint> {
    for (k, v) in hints.iter() {
        if text.starts_with(k) || k.starts_with(text) {
            return Some(v);
        }
    }
    None
}

// ── Internal: image-blob resolve wrapper ────────────────────────────

fn build_doc_for_resolve(children: Vec<PenNode>) -> jian_ops_schema::document::PenDocument {
    jian_ops_schema::document::PenDocument {
        version: "1".to_string(),
        name: None,
        themes: None,
        variables: None,
        pages: None,
        children,
        format_version: None,
        id: None,
        app: None,
        routes: None,
        state: None,
        lifecycle: None,
        logic_modules: None,
        design_md: None,
        conversion: None,
    }
}

fn extract_doc_children(doc: jian_ops_schema::document::PenDocument) -> Vec<PenNode> {
    doc.children
}

// Keep `EllipseNode` / `LineNode` / `PolygonNode` / `PathNode` /
// `TextInputNode` / `ImageNode` / `RefNode` / `FrameNode` /
// `GroupNode` / `RectangleNode` / `TextNode` in scope so future
// post-process passes don't have to re-import.
const _: () = {
    let _: Option<EllipseNode> = None;
    let _: Option<LineNode> = None;
    let _: Option<PolygonNode> = None;
    let _: Option<PathNode> = None;
    let _: Option<TextInputNode> = None;
    let _: Option<ImageNode> = None;
    let _: Option<RefNode> = None;
    let _: Option<FrameNode> = None;
    let _: Option<GroupNode> = None;
};

#[cfg(test)]
mod tests;
