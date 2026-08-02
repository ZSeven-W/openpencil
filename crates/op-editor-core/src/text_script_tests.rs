//! Sibling test file for `text_script.rs` (800-line cap convention) —
//! complex-script detection and UAX#9 base-direction resolution, the fork
//! that decides whether a run is painted by the cmap fast path or a shaper.

#![cfg(test)]

use crate::text_script::*;

#[test]
fn ascii_text_stays_on_the_simple_paint_path() {
    assert!(!needs_complex_shaping("Design System"));
}

#[test]
fn cjk_text_stays_on_the_simple_paint_path() {
    // CJK is a 1:1 cmap lookup — no joining, no reordering. Sending it to
    // the shaper would spend frame time for an identical result.
    assert!(!needs_complex_shaping("设计系统"));
}

#[test]
fn empty_text_stays_on_the_simple_paint_path() {
    assert!(!needs_complex_shaping(""));
}

#[test]
fn arabic_text_needs_shaping() {
    assert!(needs_complex_shaping("العربية"));
}

#[test]
fn devanagari_text_needs_shaping() {
    assert!(needs_complex_shaping("नमस्ते"));
}

#[test]
fn thai_text_needs_shaping() {
    assert!(needs_complex_shaping("สวัสดี"));
}

#[test]
fn arabic_presentation_forms_need_shaping() {
    // U+FB50..U+FDFF and U+FE70..U+FEFF carry pre-joined Arabic glyphs.
    // Joining is already baked in, but they still need bidi reordering.
    assert!(needs_complex_shaping("\u{FEF2}"));
}

#[test]
fn latin_mixed_with_arabic_needs_shaping() {
    // Digits next to an Arabic unit abbreviation — "50 km".
    assert!(needs_complex_shaping("50 ك.م"));
}

#[test]
fn every_supported_complex_block_is_detected() {
    // One representative letter per block the range table claims to cover,
    // so the table can't silently lose an entry.
    for (script, sample) in [
        ("Syriac", "\u{0710}"),
        ("Thaana", "\u{0780}"),
        ("NKo", "\u{07CA}"),
        ("Bengali", "\u{0985}"),
        ("Gurmukhi", "\u{0A05}"),
        ("Gujarati", "\u{0A85}"),
        ("Oriya", "\u{0B05}"),
        ("Tamil", "\u{0B85}"),
        ("Telugu", "\u{0C05}"),
        ("Kannada", "\u{0C85}"),
        ("Malayalam", "\u{0D05}"),
        ("Sinhala", "\u{0D85}"),
        ("Lao", "\u{0E81}"),
        ("Tibetan", "\u{0F40}"),
        ("Myanmar", "\u{1000}"),
        ("Khmer", "\u{1780}"),
    ] {
        assert!(
            needs_complex_shaping(sample),
            "{script} must route to the shaper"
        );
    }
}

#[test]
fn explicit_bidi_controls_need_shaping() {
    // A right-to-left override reorders even otherwise-Latin text, so the
    // fast path would paint it in the wrong order.
    assert!(needs_complex_shaping("\u{202E}abc"));
}

#[test]
fn latin_paragraph_resolves_left_to_right() {
    assert_eq!(base_direction("Hello"), BaseDirection::Ltr);
}

#[test]
fn arabic_paragraph_resolves_right_to_left() {
    assert_eq!(base_direction("مرحبا"), BaseDirection::Rtl);
}

#[test]
fn leading_digits_do_not_decide_direction() {
    // UAX#9 P2: digits are weak, so the first *strong* character decides.
    // A leading number does not make this a left-to-right paragraph.
    assert_eq!(base_direction("50 ك.م"), BaseDirection::Rtl);
}

#[test]
fn leading_punctuation_does_not_decide_direction() {
    assert_eq!(base_direction("«مرحبا»"), BaseDirection::Rtl);
}

#[test]
fn first_strong_character_wins_in_mixed_text() {
    assert_eq!(base_direction("Hello مرحبا"), BaseDirection::Ltr);
}

#[test]
fn text_without_strong_characters_defaults_left_to_right() {
    assert_eq!(base_direction("123 456"), BaseDirection::Ltr);
}
