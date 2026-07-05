//! Pure-Rust font-family extraction for the web import path.
//!
//! The vendored CanvasKit build (`assets/canvaskit/canvaskit.js`) is a slimmed
//! build with NO family-name introspection — `Typeface` has no
//! `getFamilyName()` and `FontMgr` exposes no `countFamilies`/`getFamilyName`.
//! So a freshly imported font's family (needed to key the CanvasKit typeface
//! map, the picker entry, and the IndexedDB record) is parsed here from the
//! font's `name` table with `ttf-parser` — wasm-safe and unit-testable without
//! a browser, unlike the JS/CanvasKit round-trip.

/// Extract a font's family display name from its raw `.ttf` / `.otf` bytes.
/// Prefers the Typographic Family (name id 16) over the legacy Family (id 1),
/// and a decodable Unicode/Windows record. Returns `None` when the bytes are
/// not a parseable font or carry no usable family name.
pub(crate) fn parse_family(bytes: &[u8]) -> Option<String> {
    // Face index 0 — a plain .ttf/.otf has one face; a .ttc collection's first.
    let face = ttf_parser::Face::parse(bytes, 0).ok()?;

    const FAMILY: u16 = 1;
    const TYPOGRAPHIC_FAMILY: u16 = 16;

    let mut family: Option<String> = None;
    let mut typographic: Option<String> = None;
    for name in face.names() {
        if name.name_id != FAMILY && name.name_id != TYPOGRAPHIC_FAMILY {
            continue;
        }
        // `to_string` decodes Unicode/Windows-platform records; Mac-Roman and
        // other legacy encodings yield `None` and are skipped — every font
        // that targets browsers/OSes ships a Windows Unicode family record.
        let Some(value) = name.to_string() else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if name.name_id == TYPOGRAPHIC_FAMILY {
            typographic.get_or_insert_with(|| trimmed.to_string());
        } else {
            family.get_or_insert_with(|| trimmed.to_string());
        }
    }
    typographic.or(family)
}

#[cfg(test)]
mod tests {
    use super::parse_family;

    // A real bundled OFL font (shared with op-host-desktop). include_bytes from
    // a sibling crate's assets is a compile-time file read, not a dep edge.
    const INSTRUMENT_SERIF: &[u8] =
        include_bytes!("../../op-host-desktop/assets/fonts/InstrumentSerif-Regular.ttf");
    const OUTFIT_VF: &[u8] = include_bytes!("../../op-host-desktop/assets/fonts/Outfit-VF.ttf");

    #[test]
    fn extracts_family_from_real_fonts() {
        assert_eq!(
            parse_family(INSTRUMENT_SERIF).as_deref(),
            Some("Instrument Serif")
        );
        assert_eq!(parse_family(OUTFIT_VF).as_deref(), Some("Outfit"));
    }

    #[test]
    fn rejects_non_font_bytes() {
        assert_eq!(parse_family(b"this is not a font file"), None);
        assert_eq!(parse_family(&[]), None);
    }
}
