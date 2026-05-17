//! Figma `.fig` import — file recognition + the full binary parsing
//! pipeline. The Rust port of `packages/pen-figma`: a `.fig`
//! container is unzipped + chunk-split ([`container`]), decoded
//! through the Kiwi schema ([`kiwi`]), built into a node tree
//! ([`tree`]) and converted to a `PenDocument` ([`converters`] /
//! [`node_mapper`]). Clipboard-JSON (`.fig.json`) keeps the
//! hand-rolled shallow extractor below.

mod color;
mod common;
mod container;
mod converters;
mod figma_types;
mod image_resolver;
mod instance;
mod kiwi;
mod mappers;
mod node_build;
mod node_mapper;
mod text_mapper;
mod tree;
mod vector_decoder;
mod zip_reader;

#[cfg(test)]
mod binary_e2e_tests;

pub use common::FigLayoutMode;
pub use node_mapper::{
    figma_all_pages_to_pen_document, figma_node_changes_to_pen_nodes, figma_to_pen_document,
    get_figma_pages, FigmaImportResult, FigmaPageInfo,
};

use common::FigLayoutMode as LayoutMode;
use image_resolver::resolve_image_blobs;
use jian_ops_schema::document::PenDocument;
use jian_ops_schema::node::{
    ContainerProps, EllipseNode, FrameNode, GroupNode, LineNode, PathNode, PenNode, PenNodeBase,
    PolygonNode, RectangleNode, TextContent, TextNode,
};

/// Recognised file kinds. The TS app also accepts `.fig.json`
/// (Figma's clipboard export) — that path lands once a real
/// parser is wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigFileKind {
    /// Binary `.fig` with the `fig-kiwi` magic prefix.
    Binary,
    /// JSON clipboard export (`{"type":"FIGMA_DOCUMENT"...}`).
    ClipboardJson,
    /// Unknown — caller should reject + surface an error.
    Unknown,
}

/// Sniff the first ~16 bytes / chars to decide which import path
/// applies. Returns `Unknown` for empty input.
pub fn detect_kind(bytes: &[u8]) -> FigFileKind {
    // Figma's binary container starts with the literal "fig-kiwi" — a
    // 8-byte ASCII magic followed by the chunk payload.
    if bytes.len() >= 8 && &bytes[0..8] == b"fig-kiwi" {
        return FigFileKind::Binary;
    }
    // The common export form wraps the container in a ZIP archive
    // (`canvas.fig` + `images/*`); the ZIP local-file-header magic
    // `PK\x03\x04` marks it. A non-fig ZIP simply fails later with a
    // "no canvas.fig" error.
    if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4b, 0x03, 0x04] {
        return FigFileKind::Binary;
    }
    // Clipboard JSON starts with `{` and contains the FIGMA_DOCUMENT
    // marker within the first KB. Cheap prefix check first.
    if let Some(first_non_ws) = bytes.iter().find(|b| !b.is_ascii_whitespace()) {
        if *first_non_ws == b'{' {
            let slice_end = bytes.len().min(2048);
            if let Ok(s) = std::str::from_utf8(&bytes[..slice_end]) {
                if s.contains("FIGMA_DOCUMENT") || s.contains("\"type\":\"FIGMA\"") {
                    return FigFileKind::ClipboardJson;
                }
            }
        }
    }
    FigFileKind::Unknown
}

/// A fully imported `.fig` file — the converted document plus any
/// non-fatal conversion warnings.
#[derive(Debug, Clone)]
pub struct FigImport {
    pub document: PenDocument,
    pub warnings: Vec<String>,
}

/// Parse a binary `.fig` file into a `PenDocument`. Runs the whole
/// pipeline: container split → Kiwi decode → tree build → node
/// conversion → image-blob resolution. `file_name` names the
/// document; `layout_mode` selects auto-layout resolution vs verbatim
/// Figma geometry.
pub fn parse_fig_binary(
    bytes: &[u8],
    file_name: &str,
    layout_mode: FigLayoutMode,
) -> Result<FigImport, FigParseError> {
    if detect_kind(bytes) != FigFileKind::Binary {
        return Err(FigParseError::UnknownFormat);
    }
    let decoded =
        figma_types::parse_fig_file(bytes).map_err(|e| FigParseError::Binary(e.to_string()))?;
    let image_files = decoded.image_files.clone();
    let result = figma_all_pages_to_pen_document(decoded, file_name, layout_mode);
    let mut document = result.document;
    resolve_image_blobs(&mut document, &result.image_blobs, &image_files);
    Ok(FigImport {
        document,
        warnings: result.warnings,
    })
}

/// Convert a `.fig` byte stream to a `ParsedFigStub`. For clipboard
/// JSON, extracts the top-level shape (`type` + children count); for
/// binary `.fig`, runs the full parser and reports the top-level node
/// count. Returns `UnknownFormat` on unrecognised bytes.
pub fn parse_fig(bytes: &[u8]) -> Result<ParsedFigStub, FigParseError> {
    match detect_kind(bytes) {
        FigFileKind::Binary => {
            let import = parse_fig_binary(bytes, "Figma Import", LayoutMode::OpenPencil)?;
            let top_level_children = import
                .document
                .pages
                .as_ref()
                .map(|pages| pages.iter().map(|p| p.children.len()).sum())
                .unwrap_or(0);
            Ok(ParsedFigStub {
                kind: FigFileKind::Binary,
                top_level_children,
                clipboard_nodes: Vec::new(),
            })
        }
        FigFileKind::ClipboardJson => {
            let s = std::str::from_utf8(bytes).map_err(|_| FigParseError::UnknownFormat)?;
            let nodes = extract_clipboard_nodes(s);
            Ok(ParsedFigStub {
                kind: FigFileKind::ClipboardJson,
                top_level_children: nodes.len(),
                clipboard_nodes: nodes,
            })
        }
        FigFileKind::Unknown => Err(FigParseError::UnknownFormat),
    }
}

/// Walk the clipboard-JSON children array and pull the `type` +
/// `name` fields off each top-level entry into a
/// `FigmaClipboardNode`. Returns an empty Vec when the children
/// key is absent or the brace tracker can't find a matching
/// close. Hand-rolled parser (no serde) — finds each top-level
/// `{` at depth 1, scans the matching `}` block for `"type"` +
/// `"name"`, returns the pair.
fn extract_clipboard_nodes(s: &str) -> Vec<FigmaClipboardNode> {
    let mut out = Vec::new();
    let Some(top_blocks) = collect_top_level_blocks(s) else {
        return out;
    };
    for block in top_blocks {
        let kind = extract_string_field(block, "type").unwrap_or_default();
        let name = extract_string_field(block, "name").unwrap_or_default();
        out.push(FigmaClipboardNode { kind, name });
    }
    out
}

fn extract_string_field(block: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let key_at = block.find(&needle)?;
    let after = &block[key_at + needle.len()..];
    let colon = after.find(':')? + 1;
    let val = after[colon..].trim_start();
    let val = val.strip_prefix('"')?;
    // Find closing quote, honouring `\"` escapes.
    let mut end = 0;
    let mut escape = false;
    for (i, c) in val.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '"' {
            end = i;
            break;
        }
    }
    Some(val[..end].to_string())
}

fn collect_top_level_blocks(s: &str) -> Option<Vec<&str>> {
    let needle = "\"children\"";
    let key_at = s.find(needle)?;
    let after = &s[key_at + needle.len()..];
    let bracket_rel = after.find('[')?;
    let arr_start = key_at + needle.len() + bracket_rel + 1;
    let body = &s[arr_start..];
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut current_start: Option<usize> = None;
    let mut blocks: Vec<&str> = Vec::new();
    for (i, c) in body.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match c {
            '{' => {
                if depth == 0 {
                    current_start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = current_start.take() {
                        blocks.push(&body[start..=i]);
                    }
                }
            }
            ']' if depth == 0 => return Some(blocks),
            _ => {}
        }
    }
    None
}

/// The canonical `PenNode` variant a Figma clipboard kind maps onto.
/// `PenNode` is a closed 12-variant enum with no `Other` escape
/// hatch, so unrecognised Figma kinds round-trip as `Frame` (the
/// neutral container) with the original kind string preserved in
/// `base.role` — that keeps a layer-panel entry to display and lets
/// a follow-up converter recover the source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigmaNodeVariant {
    Frame,
    Group,
    Rectangle,
    Ellipse,
    Line,
    Polygon,
    Path,
    Text,
}

/// One node extracted from a clipboard-JSON `children` array.
/// `type` is the Figma node kind string (`RECTANGLE`, `ELLIPSE`,
/// `TEXT`, `FRAME`, ...); `to_node` maps it to a canonical
/// `jian_ops_schema::PenNode` variant. Empty arrays / missing
/// fields yield empty defaults so the caller can `unwrap_or`
/// without panics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FigmaClipboardNode {
    pub kind: String,
    pub name: String,
}

impl FigmaClipboardNode {
    /// Build a canonical `PenNode` from this clipboard entry. Mints a
    /// fresh id from `next_id`, picks the closest `PenNode` variant
    /// via `to_variant`, copies the name into `base.name`. Geometry /
    /// fill / stroke all stay at schema defaults (`None`) — that's
    /// what the per-kind specialised mappers (`figma-fill-mapper`
    /// etc) port over in follow-ups. Returns the constructed node so
    /// the caller can append it to a `PenPage.children` /
    /// `PenDocument.children` list.
    pub fn to_node(&self, next_id: &mut u64) -> PenNode {
        let raw = *next_id;
        *next_id = next_id.checked_add(1).unwrap_or(raw);
        let name = if self.name.is_empty() {
            self.kind.to_lowercase()
        } else {
            self.name.clone()
        };
        let variant = self.to_variant();
        let mut base = PenNodeBase {
            id: format!("n{raw}"),
            name: Some(name),
            ..Default::default()
        };
        // `PenNode` has no `Other` variant: stash the original Figma
        // kind in `base.role` for unmapped types so a follow-up
        // converter can still recover what Figma authored.
        if !self.is_known_kind() && !self.kind.is_empty() {
            base.role = Some(self.kind.clone());
        }
        build_node(variant, base)
    }

    /// Map this clipboard node's Figma kind string onto the closest
    /// canonical `PenNode` variant. Covers the 8 most-common Figma
    /// kinds — the remaining specialised types (BOOLEAN_OPERATION,
    /// SLICE, COMPONENT, INSTANCE, STAR, ...) fall back to `Frame`
    /// and ship dedicated mappings in follow-up commits as the
    /// converters need them.
    pub fn to_variant(&self) -> FigmaNodeVariant {
        match self.kind.as_str() {
            "RECTANGLE" => FigmaNodeVariant::Rectangle,
            "ELLIPSE" => FigmaNodeVariant::Ellipse,
            "LINE" => FigmaNodeVariant::Line,
            "POLYGON" | "REGULAR_POLYGON" => FigmaNodeVariant::Polygon,
            "VECTOR" => FigmaNodeVariant::Path,
            "TEXT" => FigmaNodeVariant::Text,
            "GROUP" => FigmaNodeVariant::Group,
            "FRAME" | "SECTION" => FigmaNodeVariant::Frame,
            // Unknown / not-yet-mapped kinds become a neutral Frame;
            // the original kind is preserved in `base.role`.
            _ => FigmaNodeVariant::Frame,
        }
    }

    /// True when this kind has an explicit `to_variant` mapping (not
    /// the catch-all `Frame` fallback).
    fn is_known_kind(&self) -> bool {
        matches!(
            self.kind.as_str(),
            "RECTANGLE"
                | "ELLIPSE"
                | "LINE"
                | "POLYGON"
                | "REGULAR_POLYGON"
                | "VECTOR"
                | "TEXT"
                | "GROUP"
                | "FRAME"
                | "SECTION"
        )
    }
}

/// Construct a bare `PenNode` of the given variant with `base`
/// already filled in. Geometry / fill / stroke stay `None` so the
/// canonical schema serializes them away — the specialised Figma
/// mappers fill them in follow-up commits.
fn build_node(variant: FigmaNodeVariant, base: PenNodeBase) -> PenNode {
    match variant {
        FigmaNodeVariant::Frame => PenNode::Frame(FrameNode {
            base,
            container: ContainerProps::default(),
            children: None,
            reusable: None,
            slot: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        FigmaNodeVariant::Group => PenNode::Group(GroupNode {
            base,
            container: ContainerProps::default(),
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        FigmaNodeVariant::Rectangle => PenNode::Rectangle(RectangleNode {
            base,
            container: ContainerProps::default(),
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        FigmaNodeVariant::Ellipse => PenNode::Ellipse(EllipseNode {
            base,
            width: None,
            height: None,
            corner_radius: None,
            inner_radius: None,
            start_angle: None,
            sweep_angle: None,
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
        }),
        FigmaNodeVariant::Line => PenNode::Line(LineNode {
            base,
            x2: None,
            y2: None,
            stroke: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        FigmaNodeVariant::Polygon => PenNode::Polygon(PolygonNode {
            base,
            // Figma `POLYGON` / `REGULAR_POLYGON` default to a
            // triangle; the real point count ports with the
            // geometry mapper.
            polygon_count: 3,
            width: None,
            height: None,
            corner_radius: None,
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
        }),
        FigmaNodeVariant::Path => PenNode::Path(PathNode {
            base,
            icon_id: None,
            d: None,
            anchors: None,
            closed: None,
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
        }),
        FigmaNodeVariant::Text => PenNode::Text(TextNode {
            base,
            width: None,
            height: None,
            // Empty body until the text mapper ports real runs.
            content: TextContent::Plain(String::new()),
            font_family: None,
            font_size: None,
            font_weight: None,
            font_style: None,
            letter_spacing: None,
            line_height: None,
            text_align: None,
            text_align_vertical: None,
            text_growth: None,
            underline: None,
            strikethrough: None,
            fill: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
    }
}

/// Successful parse result. Carries the file kind, top-level
/// children count, and (for clipboard JSON) a minimal-fields
/// extraction of each top-level child. Placeholder until the full
/// Figma → PenNode mapping ports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFigStub {
    pub kind: FigFileKind,
    pub top_level_children: usize,
    pub clipboard_nodes: Vec<FigmaClipboardNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FigParseError {
    /// Input is neither a binary `.fig` nor a Figma clipboard JSON.
    UnknownFormat,
    /// Binary `.fig` parsing failed — carries the underlying reason.
    Binary(String),
}

impl std::fmt::Display for FigParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FigParseError::UnknownFormat => write!(f, "unrecognised .fig format"),
            FigParseError::Binary(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::pen_node_ext::PenNodeExt;

    #[test]
    fn detect_kind_recognises_binary_magic() {
        let mut bytes = b"fig-kiwi".to_vec();
        bytes.extend_from_slice(&[0u8; 32]);
        assert_eq!(detect_kind(&bytes), FigFileKind::Binary);
    }

    #[test]
    fn detect_kind_recognises_clipboard_json() {
        let payload = br#"{"type":"FIGMA_DOCUMENT","children":[]}"#;
        assert_eq!(detect_kind(payload), FigFileKind::ClipboardJson);
    }

    #[test]
    fn detect_kind_rejects_random_bytes() {
        assert_eq!(detect_kind(b"hello world"), FigFileKind::Unknown);
        assert_eq!(detect_kind(b"\xff\xfe\x00\x00"), FigFileKind::Unknown);
        assert_eq!(detect_kind(&[]), FigFileKind::Unknown);
    }

    #[test]
    fn parse_fig_binary_rejects_malformed_container() {
        // `fig-kiwi` magic but no decodable chunks → a Binary error,
        // no longer NotYetImplemented.
        let mut bin = b"fig-kiwi".to_vec();
        bin.extend_from_slice(&[0u8; 4]);
        assert!(matches!(parse_fig(&bin), Err(FigParseError::Binary(_))));
    }

    #[test]
    fn parse_fig_clipboard_returns_children_count() {
        let json = br#"{"type":"FIGMA_DOCUMENT","children":[{"id":"1"},{"id":"2"},{"id":"3"}]}"#;
        let parsed = parse_fig(json).unwrap();
        assert_eq!(parsed.kind, FigFileKind::ClipboardJson);
        assert_eq!(parsed.top_level_children, 3);
    }

    #[test]
    fn parse_fig_clipboard_handles_nested_objects_correctly() {
        // 2 top-level entries, each with a nested object — only top-
        // level entries should be counted.
        let json = br#"{"type":"FIGMA_DOCUMENT","children":[{"id":"a","child":{"id":"a1"}},{"id":"b","child":{"id":"b1"}}]}"#;
        let parsed = parse_fig(json).unwrap();
        assert_eq!(parsed.top_level_children, 2);
    }

    #[test]
    fn parse_fig_clipboard_handles_quoted_braces() {
        let json =
            br#"{"type":"FIGMA_DOCUMENT","children":[{"name":"has \"{\" in name"},{"id":"b"}]}"#;
        let parsed = parse_fig(json).unwrap();
        // Quoted `{` must NOT bump the count.
        assert_eq!(parsed.top_level_children, 2);
    }

    #[test]
    fn parse_fig_clipboard_extracts_type_and_name_per_top_level() {
        let json = br#"{"type":"FIGMA_DOCUMENT","children":[
            {"type":"RECTANGLE","name":"Hero Bg"},
            {"type":"TEXT","name":"Title"},
            {"type":"FRAME","name":"Card","children":[{"type":"ELLIPSE","name":"Avatar"}]}
        ]}"#;
        let parsed = parse_fig(json).unwrap();
        assert_eq!(parsed.top_level_children, 3);
        assert_eq!(parsed.clipboard_nodes.len(), 3);
        assert_eq!(parsed.clipboard_nodes[0].kind, "RECTANGLE");
        assert_eq!(parsed.clipboard_nodes[0].name, "Hero Bg");
        assert_eq!(parsed.clipboard_nodes[1].kind, "TEXT");
        assert_eq!(parsed.clipboard_nodes[1].name, "Title");
        // Nested child not in top-level (only Card is, not its Avatar).
        assert_eq!(parsed.clipboard_nodes[2].kind, "FRAME");
        assert_eq!(parsed.clipboard_nodes[2].name, "Card");
    }

    #[test]
    fn parse_fig_clipboard_handles_missing_fields_gracefully() {
        let json =
            br#"{"type":"FIGMA_DOCUMENT","children":[{"name":"only-name"},{"type":"TEXT"}]}"#;
        let parsed = parse_fig(json).unwrap();
        assert_eq!(parsed.clipboard_nodes[0].kind, "");
        assert_eq!(parsed.clipboard_nodes[0].name, "only-name");
        assert_eq!(parsed.clipboard_nodes[1].kind, "TEXT");
        assert_eq!(parsed.clipboard_nodes[1].name, "");
    }

    #[test]
    fn figma_kind_maps_to_pen_node_variant_for_common_types() {
        let cases = [
            ("RECTANGLE", FigmaNodeVariant::Rectangle),
            ("ELLIPSE", FigmaNodeVariant::Ellipse),
            ("LINE", FigmaNodeVariant::Line),
            ("POLYGON", FigmaNodeVariant::Polygon),
            ("REGULAR_POLYGON", FigmaNodeVariant::Polygon),
            ("VECTOR", FigmaNodeVariant::Path),
            ("TEXT", FigmaNodeVariant::Text),
            ("FRAME", FigmaNodeVariant::Frame),
            ("GROUP", FigmaNodeVariant::Group),
            ("SECTION", FigmaNodeVariant::Frame),
        ];
        for (input, expected) in cases {
            let n = FigmaClipboardNode {
                kind: input.into(),
                name: String::new(),
            };
            assert_eq!(n.to_variant(), expected, "{input}");
        }
    }

    #[test]
    fn figma_clipboard_node_to_node_builds_matching_pen_node_variant() {
        // (figma clipboard kind, predicate asserting the mapped variant).
        type VariantCase = (&'static str, fn(&PenNode) -> bool);
        let cases: [VariantCase; 8] = [
            ("RECTANGLE", |n| matches!(n, PenNode::Rectangle(_))),
            ("ELLIPSE", |n| matches!(n, PenNode::Ellipse(_))),
            ("LINE", |n| matches!(n, PenNode::Line(_))),
            ("POLYGON", |n| matches!(n, PenNode::Polygon(_))),
            ("VECTOR", |n| matches!(n, PenNode::Path(_))),
            ("TEXT", |n| matches!(n, PenNode::Text(_))),
            ("FRAME", |n| matches!(n, PenNode::Frame(_))),
            ("GROUP", |n| matches!(n, PenNode::Group(_))),
        ];
        for (kind, check) in cases {
            let entry = FigmaClipboardNode {
                kind: kind.into(),
                name: String::new(),
            };
            let mut next = 1u64;
            let node = entry.to_node(&mut next);
            assert!(check(&node), "{kind}");
        }
    }

    #[test]
    fn figma_clipboard_node_to_node_mints_fresh_id() {
        let entry = FigmaClipboardNode {
            kind: "RECTANGLE".into(),
            name: "Hero".into(),
        };
        let mut next = 100u64;
        let n1 = entry.to_node(&mut next);
        let n2 = entry.to_node(&mut next);
        assert_eq!(n1.id_str(), "n100");
        assert_eq!(n2.id_str(), "n101");
        assert_eq!(next, 102);
        assert_eq!(n1.base().name.as_deref(), Some("Hero"));
    }

    #[test]
    fn figma_clipboard_node_falls_back_to_kind_as_name() {
        let entry = FigmaClipboardNode {
            kind: "ELLIPSE".into(),
            name: String::new(),
        };
        let mut next = 1u64;
        let n = entry.to_node(&mut next);
        // Empty name → lowercased kind so the layer panel has
        // something to render.
        assert_eq!(n.base().name.as_deref(), Some("ellipse"));
    }

    #[test]
    fn figma_kind_unknown_falls_back_to_frame_with_role() {
        let entry = FigmaClipboardNode {
            kind: "BOOLEAN_OPERATION".into(),
            name: String::new(),
        };
        assert_eq!(entry.to_variant(), FigmaNodeVariant::Frame);
        let mut next = 1u64;
        let n = entry.to_node(&mut next);
        assert!(matches!(n, PenNode::Frame(_)));
        // Original Figma kind preserved in `base.role` so a later
        // converter can recover it.
        assert_eq!(n.base().role.as_deref(), Some("BOOLEAN_OPERATION"));
    }

    #[test]
    fn figma_known_kind_leaves_role_unset() {
        let entry = FigmaClipboardNode {
            kind: "RECTANGLE".into(),
            name: "Hero".into(),
        };
        let mut next = 1u64;
        let n = entry.to_node(&mut next);
        assert_eq!(n.base().role, None);
    }

    #[test]
    fn parse_fig_returns_unknown_for_random_bytes() {
        assert_eq!(parse_fig(b"random"), Err(FigParseError::UnknownFormat));
    }
}
