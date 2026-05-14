//! Figma `.fig` import — file recognition + outline of the
//! conversion pipeline. Real binary parsing (Zstd-decompressed
//! schema-encoded blob → `FigmaNode` tree → `PenNode`) lives in
//! `packages/pen-figma`; the Rust port is a 17-file effort that
//! lands incrementally behind this module's trait.

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
    // 8-byte ASCII magic followed by the Zstd-compressed payload.
    if bytes.len() >= 8 && &bytes[0..8] == b"fig-kiwi" {
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

/// Convert a `.fig` byte stream to a `ParsedFigStub`. For clipboard
/// JSON, extracts the top-level shape (`type` + children count); for
/// binary `.fig`, recognises the magic but defers full parsing to
/// the upcoming Zstd-backed pipeline. Returns `UnknownFormat` on
/// unrecognised bytes.
pub fn parse_fig(bytes: &[u8]) -> Result<ParsedFigStub, FigParseError> {
    match detect_kind(bytes) {
        FigFileKind::Binary => {
            // Binary format requires Zstd decompression before the
            // schema-encoded body becomes readable. Adding a Zstd
            // dep to shell-core (wasm32-clean) is the work for the
            // dedicated server-side parser binary; the stub kind
            // recognition stays here.
            Err(FigParseError::NotYetImplemented(FigFileKind::Binary))
        }
        FigFileKind::ClipboardJson => {
            let s =
                std::str::from_utf8(bytes).map_err(|_| FigParseError::UnknownFormat)?;
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

/// Walk a clipboard-JSON payload and count the top-level entries in
/// the `"children": [ ... ]` array. Hand-rolled parser (no serde,
/// shell-core stays wasm32-light) — counts `{` openings at depth 1
/// inside the children array. Returns `None` when the children key
/// is absent or the brace tracker can't find a matching close.
fn count_top_level_children(s: &str) -> Option<usize> {
    let needle = "\"children\"";
    let key_at = s.find(needle)?;
    let after = &s[key_at + needle.len()..];
    let bracket = after.find('[')?;
    let mut depth = 0i32;
    let mut count = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for c in after[bracket..].chars().skip(1) {
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
                    count += 1;
                }
                depth += 1;
            }
            '}' => depth -= 1,
            ']' if depth == 0 => return Some(count),
            _ => {}
        }
    }
    None
}

/// One node extracted from a clipboard-JSON `children` array.
/// `type` is the Figma node kind string (`RECTANGLE`, `ELLIPSE`,
/// `TEXT`, `FRAME`, ...); future `pen-figma`-style converters map
/// it to a `PenNode` variant. Empty arrays / missing fields yield
/// empty defaults so the caller can `unwrap_or` without panics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FigmaClipboardNode {
    pub kind: String,
    pub name: String,
}

impl FigmaClipboardNode {
    /// Map this clipboard node's Figma kind string onto the
    /// closest `NodeKind` in the shell-core document model.
    /// Unknown kinds round-trip as `NodeKind::Other(kind)` so the
    /// shell still has a layer-panel entry to display.
    /// Covers the 8 most-common Figma kinds — the remaining
    /// specialised types (BOOLEAN_OPERATION, SLICE, COMPONENT,
    /// INSTANCE, STAR, ...) ship in follow-up commits as the
    /// converters need them.
    pub fn to_node_kind(&self) -> crate::document::NodeKind {
        use crate::document::NodeKind;
        match self.kind.as_str() {
            "RECTANGLE" => NodeKind::Rect,
            "ELLIPSE" => NodeKind::Ellipse,
            "LINE" => NodeKind::Line,
            "POLYGON" | "REGULAR_POLYGON" => NodeKind::Polygon,
            "VECTOR" => NodeKind::Path,
            "TEXT" => NodeKind::Text,
            "FRAME" | "GROUP" | "SECTION" => {
                if self.kind == "GROUP" {
                    NodeKind::Group
                } else {
                    NodeKind::Frame
                }
            }
            _ => NodeKind::Other(self.kind.clone()),
        }
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
    UnknownFormat,
    NotYetImplemented(FigFileKind),
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn parse_fig_binary_returns_not_yet() {
        let mut bin = b"fig-kiwi".to_vec();
        bin.extend_from_slice(&[0u8; 4]);
        assert_eq!(
            parse_fig(&bin),
            Err(FigParseError::NotYetImplemented(FigFileKind::Binary))
        );
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
        let json = br#"{"type":"FIGMA_DOCUMENT","children":[{"name":"has \"{\" in name"},{"id":"b"}]}"#;
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
        let json = br#"{"type":"FIGMA_DOCUMENT","children":[{"name":"only-name"},{"type":"TEXT"}]}"#;
        let parsed = parse_fig(json).unwrap();
        assert_eq!(parsed.clipboard_nodes[0].kind, "");
        assert_eq!(parsed.clipboard_nodes[0].name, "only-name");
        assert_eq!(parsed.clipboard_nodes[1].kind, "TEXT");
        assert_eq!(parsed.clipboard_nodes[1].name, "");
    }

    #[test]
    fn figma_kind_maps_to_node_kind_for_common_types() {
        use crate::document::NodeKind;
        let cases = [
            ("RECTANGLE", NodeKind::Rect),
            ("ELLIPSE", NodeKind::Ellipse),
            ("LINE", NodeKind::Line),
            ("POLYGON", NodeKind::Polygon),
            ("REGULAR_POLYGON", NodeKind::Polygon),
            ("VECTOR", NodeKind::Path),
            ("TEXT", NodeKind::Text),
            ("FRAME", NodeKind::Frame),
            ("GROUP", NodeKind::Group),
            ("SECTION", NodeKind::Frame),
        ];
        for (input, expected) in cases {
            let n = FigmaClipboardNode {
                kind: input.into(),
                name: String::new(),
            };
            assert_eq!(n.to_node_kind(), expected, "{input}");
        }
    }

    #[test]
    fn figma_kind_unknown_round_trips_via_other() {
        use crate::document::NodeKind;
        let n = FigmaClipboardNode {
            kind: "BOOLEAN_OPERATION".into(),
            name: String::new(),
        };
        assert_eq!(
            n.to_node_kind(),
            NodeKind::Other("BOOLEAN_OPERATION".into())
        );
    }

    #[test]
    fn parse_fig_returns_unknown_for_random_bytes() {
        assert_eq!(parse_fig(b"random"), Err(FigParseError::UnknownFormat));
    }
}
