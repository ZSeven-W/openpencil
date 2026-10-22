//! A solid fill's own `opacity` must reach the background composite.
//!
//! Measured 2026-10-22 (surf forecast, astra rep-2): a "best window" card
//! filled `$--primary` at 8% opacity on a near-black page carried dark
//! `$--card` text. Read as opaque orange the text passed 5:1; rendered, the
//! card is near-black and the value was invisible.
use super::*;

const PAGE: &str = "#081B2C";
const DARK_TEXT: &str = "#1E293B";

#[test]
fn tinted_card_is_measured_as_the_surface_it_mostly_shows() {
    let (mut sink, root_id) = contrast_sink(
        Some(json!({"type": "solid", "color": PAGE})),
        Some(json!({"type": "solid", "color": "#F97316", "opacity": 0.08})),
        json!(DARK_TEXT),
    );

    assert_eq!(repair_text_contrast(&mut sink, &root_id), 1);
    assert_ne!(contrast_label_fill(&sink), DARK_TEXT);
}

#[test]
fn fully_opaque_accent_card_keeps_its_dark_label() {
    let (mut sink, root_id) = contrast_sink(
        Some(json!({"type": "solid", "color": PAGE})),
        Some(json!({"type": "solid", "color": "#F97316", "opacity": 1.0})),
        json!(DARK_TEXT),
    );

    assert_eq!(repair_text_contrast(&mut sink, &root_id), 0);
    assert_eq!(contrast_label_fill(&sink), DARK_TEXT);
}
