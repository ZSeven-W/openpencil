//! Variable-font default-weight normalization for the web host.
//!
//! Several of the bundled OFL variable fonts (Outfit, Manrope, Space Grotesk,
//! Cormorant Garamond) place their default instance on the *thinnest* master —
//! `wght` 100–300. FreeType hands back that default instance, so a document
//! asking for the family at 400/500 renders and measures hairline; synthetic
//! bold only compensates from 600 up.
//!
//! The native host fixes this by instancing the typeface through skia's
//! variation API (`jian_skia::bundled_fonts`). The vendored CanvasKit build
//! exposes no variation API at all, so the web equivalent rewrites the `fvar`
//! table's `wght` default in the raw bytes *before* handing them to
//! `MakeFreeTypeFaceFromData`. It is a four-byte in-place edit — no table is
//! added, moved, or resized — which keeps every other offset in the file valid.

/// `wght` axis tag, big-endian.
const WGHT_TAG: u32 = u32::from_be_bytes(*b"wght");
/// `fvar` table tag, big-endian.
const FVAR_TAG: u32 = u32::from_be_bytes(*b"fvar");
/// Regular weight as a 16.16 fixed-point value — what the default instance
/// should sit on.
const TARGET_WGHT_FIXED: i32 = 400 << 16;
/// Byte offset of `defaultValue` inside an `fvar` axis record.
const AXIS_DEFAULT_OFFSET: usize = 8;
/// Smallest legal `fvar` axis record (tag + min/def/max + flags + nameID).
const MIN_AXIS_SIZE: usize = 20;

/// Rewrite the `wght` axis default to regular (400), clamped to the axis range.
///
/// Returns `Some(patched)` only when a patch was both needed and applied;
/// `None` means "register the original bytes" — a non-variable face, a face
/// whose default already sits at the target, a font collection (`ttcf`, whose
/// faces share tables), or anything that does not parse as an sfnt.
///
/// The edit leaves the `fvar` table checksum and the head `checkSumAdjustment`
/// stale. Neither FreeType (CanvasKit's font backend) nor `ttf-parser` verifies
/// them, and recomputing would mean rewriting the whole directory for no
/// consumer's benefit.
pub(crate) fn with_default_wght_400(bytes: &[u8]) -> Option<Vec<u8>> {
    let fvar = fvar_table_offset(bytes)?;
    let axis = wght_axis_offset(bytes, fvar)?;
    let default_at = axis + AXIS_DEFAULT_OFFSET;

    let min = read_i32(bytes, axis + 4)?;
    let default = read_i32(bytes, default_at)?;
    let max = read_i32(bytes, axis + 12)?;
    // A malformed range (min > max) would make `clamp` panic; treat it as
    // "nothing sensible to write" instead.
    if min > max {
        return None;
    }
    let target = TARGET_WGHT_FIXED.clamp(min, max);
    if default == target {
        return None;
    }

    let mut patched = bytes.to_vec();
    patched[default_at..default_at + 4].copy_from_slice(&target.to_be_bytes());
    Some(patched)
}

/// Locate the `fvar` table in the sfnt table directory.
fn fvar_table_offset(bytes: &[u8]) -> Option<usize> {
    let version = read_u32(bytes, 0)?;
    // TrueType outlines (0x00010000) and CFF outlines ('OTTO') both carry a
    // normal table directory. A 'ttcf' collection instead carries a header of
    // offsets to several directories — out of scope, and patching one face's
    // shared table would corrupt the others.
    if version != 0x0001_0000 && version != u32::from_be_bytes(*b"OTTO") {
        return None;
    }
    let table_count = read_u16(bytes, 4)? as usize;
    for index in 0..table_count {
        let record = 12 + index * 16;
        let tag = read_u32(bytes, record)?;
        if tag != FVAR_TAG {
            continue;
        }
        let offset = read_u32(bytes, record + 8)? as usize;
        let length = read_u32(bytes, record + 12)? as usize;
        // The whole table must be inside the file before any field is read.
        if offset.checked_add(length)? > bytes.len() {
            return None;
        }
        return Some(offset);
    }
    None
}

/// Locate the `wght` axis record inside an `fvar` table.
fn wght_axis_offset(bytes: &[u8], fvar: usize) -> Option<usize> {
    let axes_offset = read_u16(bytes, fvar + 4)? as usize;
    let axis_count = read_u16(bytes, fvar + 8)? as usize;
    let axis_size = read_u16(bytes, fvar + 10)? as usize;
    // `axisSize` is declared per-font so later spec revisions can grow the
    // record; anything smaller than the v1 record cannot be walked.
    if axis_size < MIN_AXIS_SIZE {
        return None;
    }
    let axes_start = fvar.checked_add(axes_offset)?;
    for index in 0..axis_count {
        let axis = axes_start.checked_add(index.checked_mul(axis_size)?)?;
        if read_u32(bytes, axis)? == WGHT_TAG {
            // Every field this module reads or writes must be in bounds.
            if axis.checked_add(MIN_AXIS_SIZE)? > bytes.len() {
                return None;
            }
            return Some(axis);
        }
    }
    None
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let slice = bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32(bytes: &[u8], at: usize) -> Option<i32> {
    read_u32(bytes, at).map(|value| value as i32)
}

#[cfg(test)]
mod tests {
    use super::with_default_wght_400;

    // Real bundled OFL faces (shared with op-host-desktop). `include_bytes!`
    // from a sibling crate's assets is a compile-time file read, not a dep edge.
    const OUTFIT_VF: &[u8] = include_bytes!("../../op-host-desktop/assets/fonts/Outfit-VF.ttf");
    const INTER_VF: &[u8] = include_bytes!("../../op-host-desktop/assets/fonts/Inter-VF.ttf");
    const INSTRUMENT_SERIF: &[u8] =
        include_bytes!("../../op-host-desktop/assets/fonts/InstrumentSerif-Regular.ttf");

    fn default_wght(bytes: &[u8]) -> f32 {
        let face = ttf_parser::Face::parse(bytes, 0).expect("parseable font");
        face.variation_axes()
            .into_iter()
            .find(|axis| axis.tag == ttf_parser::Tag::from_bytes(b"wght"))
            .expect("a wght axis")
            .def_value
    }

    #[test]
    fn a_thin_default_variable_font_is_repointed_at_regular() {
        assert!(
            default_wght(OUTFIT_VF) < 400.0,
            "fixture drifted: Outfit is expected to default to a thin master"
        );
        let patched = with_default_wght_400(OUTFIT_VF).expect("a thin default needs a patch");
        assert_eq!(default_wght(&patched), 400.0);
        assert_eq!(
            patched.len(),
            OUTFIT_VF.len(),
            "the patch is in place — no table may move"
        );
    }

    #[test]
    fn a_face_already_defaulting_to_regular_is_left_alone() {
        assert_eq!(default_wght(INTER_VF), 400.0);
        assert!(with_default_wght_400(INTER_VF).is_none());
    }

    #[test]
    fn a_static_face_and_non_font_bytes_need_no_patch() {
        assert!(with_default_wght_400(INSTRUMENT_SERIF).is_none());
        assert!(with_default_wght_400(b"this is not a font file").is_none());
        assert!(with_default_wght_400(&[]).is_none());
    }
}
