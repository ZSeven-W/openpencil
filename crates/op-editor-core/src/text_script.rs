//! Complex-script detection + base-direction resolution.
//!
//! Both paint backends keep a fast path that maps codepoints straight to
//! glyphs (`Canvas::draw_str` on native, `canvas.drawText` on CanvasKit).
//! That path does no bidi reordering and no contextual glyph selection, so
//! it renders Arabic in storage order with isolated letterforms. It is
//! still the right path for Latin and CJK, which are 1:1 cmap lookups —
//! routing those through a shaper costs frame time for no visual gain.
//!
//! These predicates are the fork between the two paths. They live here, in
//! the wasm32-clean core, so the native and web hosts make the identical
//! routing decision and — critically — so paint and measurement can never
//! disagree about which path a run takes.

/// Resolved paragraph direction, per UAX#9 rules P2/P3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseDirection {
    Ltr,
    Rtl,
}

/// Lowest codepoint in any table below. Latin, Greek, Cyrillic and Armenian
/// all sit beneath it, so the overwhelmingly common case rejects
/// on one compare per character instead of walking the range tables.
const FIRST_COMPLEX_CODEPOINT: u32 = 0x0600;

/// Right-to-left scripts. These need bidi reordering; the Arabic-family
/// ones additionally need contextual joining.
const RTL_BLOCKS: &[(u32, u32)] = &[
    (0x0600, 0x06FF), // Arabic
    (0x0700, 0x074F), // Syriac
    (0x0750, 0x077F), // Arabic Supplement
    (0x0780, 0x07BF), // Thaana
    (0x07C0, 0x07FF), // NKo
    (0x0800, 0x083F), // Samaritan
    (0x0840, 0x085F), // Mandaic
    (0x0860, 0x086F), // Syriac Supplement
    (0x0870, 0x089F), // Arabic Extended-B
    (0x08A0, 0x08FF), // Arabic Extended-A
    (0xFB50, 0xFDFF), // Arabic Presentation Forms-A
    (0xFE70, 0xFEFF), // Arabic Presentation Forms-B
];

/// Left-to-right scripts that still need a shaper: conjunct formation,
/// reordered vowel signs, and stacked marks all mean the painted glyph
/// sequence differs from the stored codepoint sequence.
const COMPLEX_LTR_BLOCKS: &[(u32, u32)] = &[
    (0x0900, 0x097F), // Devanagari
    (0x0980, 0x09FF), // Bengali
    (0x0A00, 0x0A7F), // Gurmukhi
    (0x0A80, 0x0AFF), // Gujarati
    (0x0B00, 0x0B7F), // Oriya
    (0x0B80, 0x0BFF), // Tamil
    (0x0C00, 0x0C7F), // Telugu
    (0x0C80, 0x0CFF), // Kannada
    (0x0D00, 0x0D7F), // Malayalam
    (0x0D80, 0x0DFF), // Sinhala
    (0x0E00, 0x0E7F), // Thai
    (0x0E80, 0x0EFF), // Lao
    (0x0F00, 0x0FFF), // Tibetan
    (0x1000, 0x109F), // Myanmar
    (0x1780, 0x17FF), // Khmer
];

/// Explicit bidi formatting characters. Text that is otherwise entirely
/// Latin still reorders when one of these is present.
const BIDI_CONTROLS: &[(u32, u32)] = &[
    (0x200E, 0x200F), // LRM / RLM
    (0x202A, 0x202E), // LRE / RLE / PDF / LRO / RLO
    (0x2066, 0x2069), // LRI / RLI / FSI / PDI
];

fn in_blocks(cp: u32, blocks: &[(u32, u32)]) -> bool {
    blocks.iter().any(|&(lo, hi)| cp >= lo && cp <= hi)
}

fn is_strong_rtl(c: char) -> bool {
    let cp = c as u32;
    cp >= FIRST_COMPLEX_CODEPOINT && in_blocks(cp, RTL_BLOCKS)
}

/// Whether `text` needs a real shaping engine (bidi reordering and/or
/// contextual glyph selection) rather than the 1:1 cmap fast path.
pub fn needs_complex_shaping(text: &str) -> bool {
    text.chars().any(|c| {
        let cp = c as u32;
        cp >= FIRST_COMPLEX_CODEPOINT
            && (in_blocks(cp, RTL_BLOCKS)
                || in_blocks(cp, COMPLEX_LTR_BLOCKS)
                || in_blocks(cp, BIDI_CONTROLS))
    })
}

/// Resolve the base paragraph direction from the first strong directional
/// character, defaulting to [`BaseDirection::Ltr`] when there is none.
///
/// This is UAX#9 P2/P3 narrowed to what paint needs: digits, punctuation
/// and whitespace are weak or neutral and must not decide direction, so a
/// string like `"50 ك.م"` resolves right-to-left on its Arabic tail rather
/// than left-to-right on its leading digits.
pub fn base_direction(text: &str) -> BaseDirection {
    for c in text.chars() {
        if is_strong_rtl(c) {
            return BaseDirection::Rtl;
        }
        // Everything else with a letter category is strong left-to-right —
        // Latin, Greek, Cyrillic, CJK, and the complex-but-LTR scripts.
        if c.is_alphabetic() {
            return BaseDirection::Ltr;
        }
    }
    BaseDirection::Ltr
}
