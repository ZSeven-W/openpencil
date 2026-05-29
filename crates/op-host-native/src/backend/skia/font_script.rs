/// Hangul (Korean): Jamo, Compatibility Jamo, Jamo Extended-A/B, and
/// the precomposed syllable block. Routed to a dedicated Korean
/// typeface — the shared CJK face is resolved from a Han ideograph
/// (a Chinese font) and does NOT cover Hangul, so without this split
/// Korean text renders as blank `.notdef` boxes.
pub(super) fn is_hangul_codepoint(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7A3 | 0xD7B0..=0xD7FF
    )
}

pub(super) fn is_east_asian_codepoint(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        0x2E80..=0x2EFF
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0x3100..=0x312F
            | 0x3130..=0x318F
            | 0x31A0..=0x31BF
            | 0x31C0..=0x31EF
            | 0x31F0..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE1F
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2CEB0..=0x2EBEF
            | 0x30000..=0x3134F
    )
}
