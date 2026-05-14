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

/// Convert a `.fig` byte stream to a `PenDocument`. Returns Err
/// when the file is unrecognised; returns Err("not yet implemented")
/// when the kind IS recognised but the parser hasn't shipped.
/// Real implementations land in `pen-figma` ports in follow-ups.
pub fn parse_fig(bytes: &[u8]) -> Result<ParsedFigStub, FigParseError> {
    match detect_kind(bytes) {
        FigFileKind::Binary => Err(FigParseError::NotYetImplemented(FigFileKind::Binary)),
        FigFileKind::ClipboardJson => {
            Err(FigParseError::NotYetImplemented(FigFileKind::ClipboardJson))
        }
        FigFileKind::Unknown => Err(FigParseError::UnknownFormat),
    }
}

/// Successful parse result (placeholder — the real type holds a
/// `jian_ops_schema::PenDocument` once the parser ports finish).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFigStub {
    pub kind: FigFileKind,
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
    fn parse_fig_returns_not_yet_for_recognised_kinds() {
        let mut bin = b"fig-kiwi".to_vec();
        bin.extend_from_slice(&[0u8; 4]);
        assert_eq!(
            parse_fig(&bin),
            Err(FigParseError::NotYetImplemented(FigFileKind::Binary))
        );
        let json = br#"{"type":"FIGMA_DOCUMENT"}"#;
        assert_eq!(
            parse_fig(json),
            Err(FigParseError::NotYetImplemented(FigFileKind::ClipboardJson))
        );
    }

    #[test]
    fn parse_fig_returns_unknown_for_random_bytes() {
        assert_eq!(parse_fig(b"random"), Err(FigParseError::UnknownFormat));
    }
}
