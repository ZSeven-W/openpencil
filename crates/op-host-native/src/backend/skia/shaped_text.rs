//! Shaped paint + measure for complex scripts.
//!
//! The surrounding fast path (`text.rs`) maps codepoints straight to glyphs
//! via `Canvas::draw_str`, which does no bidi reordering and no contextual
//! glyph selection — correct and cheap for Latin and CJK, wrong for Arabic.
//! Runs that `op_editor_core::text_script::needs_complex_shaping` flags come
//! here instead and go through Skia's Paragraph shaper (ICU bidi +
//! HarfBuzz).
//!
//! The reason jian's `draw_text_paragraph` was bypassed natively is that it
//! builds a fresh `FontCollection` on every call (~605 ms chrome frames).
//! This type keeps the collection cached with the same generation-guard
//! `jian_skia::ParagraphBaseline` uses, so the shaper is affordable on the
//! paint path.

use std::rc::Rc;

use jian_skia::FontResolver;
use op_editor_core::text_script::{base_direction, BaseDirection};
use skia_safe::{
    font_style::{Slant, Weight, Width},
    textlayout::{FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, TextStyle},
    Canvas, FontStyle,
};

/// The default family jian seeds its own collections with, so a shaped run
/// falls back to the same face an unshaped one would.
const DEFAULT_FAMILY: &str = "Roboto";

/// Effectively unbounded wrap budget. Every run reaching paint is already a
/// single line (wrapping happens upstream in `wrap_text`), so the paragraph
/// must not break; `max_intrinsic_width` then reports the natural width.
const NATURAL_LAYOUT_BUDGET: f32 = 1.0e6;

/// One styled run to shape. Grouped into a struct so paint and measure take
/// the identical inputs — if they ever drifted, wrapped text would be laid
/// out against one width and painted at another.
pub(crate) struct ShapedRun<'a> {
    pub text: &'a str,
    pub family: &'a str,
    pub font_size: f32,
    pub weight: u16,
    pub italic: bool,
    pub line_height: f32,
    pub color: skia_safe::Color,
}

pub(crate) struct ShapedText {
    collection: Rc<FontCollection>,
    built_generation: u64,
}

impl ShapedText {
    pub(crate) fn new(font_resolver: &FontResolver) -> Self {
        jian_skia::with_font_lock(|| Self {
            // Read the generation before building: a concurrent font
            // registration can at worst leave this stale-low, costing one
            // harmless rebuild.
            built_generation: jian_skia::font_generation(),
            collection: Rc::new(build_collection(font_resolver)),
        })
    }

    fn collection(&mut self, font_resolver: &FontResolver) -> Rc<FontCollection> {
        let generation = jian_skia::font_generation();
        if generation != self.built_generation {
            self.collection = Rc::new(build_collection(font_resolver));
            self.built_generation = generation;
        }
        Rc::clone(&self.collection)
    }

    /// Natural, unwrapped advance width of the shaped run.
    pub(crate) fn measure(&mut self, font_resolver: &FontResolver, run: &ShapedRun<'_>) -> f32 {
        jian_skia::with_font_lock(|| {
            let mut paragraph = self.build(font_resolver, run);
            paragraph.layout(NATURAL_LAYOUT_BUDGET);
            paragraph.max_intrinsic_width()
        })
    }

    /// Paint the run with `baseline_y` on its alphabetic baseline, matching
    /// the `draw_str` convention the unshaped path uses, so shaped and
    /// unshaped text sit on one line.
    pub(crate) fn paint(
        &mut self,
        canvas: &Canvas,
        font_resolver: &FontResolver,
        run: &ShapedRun<'_>,
        x: f32,
        baseline_y: f32,
    ) {
        jian_skia::with_font_lock(|| {
            let mut paragraph = self.build(font_resolver, run);
            // Two passes. A right-to-left paragraph anchors against the RIGHT
            // edge of its layout box, so laying out once against the probe
            // budget would fling Arabic a million pixels off-screen. Measure
            // the natural width first, then re-lay out into a box of exactly
            // that width so the run occupies the same span at `x` either way.
            //
            // `ceil` rather than the raw width: a box even a float-epsilon
            // narrower than the text makes the shaper wrap, turning one line
            // into two. Rounding up trades a sub-pixel anchor shift for
            // never breaking a line that should not break.
            paragraph.layout(NATURAL_LAYOUT_BUDGET);
            let natural = paragraph.max_intrinsic_width();
            paragraph.layout(natural.ceil());
            // `Paragraph::paint` places the line-box top; callers hand us a
            // baseline.
            let top = baseline_y - paragraph.alphabetic_baseline();
            paragraph.paint(canvas, (x, top));
        });
    }

    fn build(&mut self, font_resolver: &FontResolver, run: &ShapedRun<'_>) -> Paragraph {
        let collection = self.collection(font_resolver);
        let mut paragraph_style = ParagraphStyle::new();
        // Without this the shaper bidi-reorders correctly but anchors the
        // paragraph left-to-right, so a trailing neutral (the "." in an
        // abbreviation like "ك.م") lands on the wrong end.
        paragraph_style.set_text_direction(match base_direction(run.text) {
            BaseDirection::Rtl => skia_safe::textlayout::TextDirection::RTL,
            BaseDirection::Ltr => skia_safe::textlayout::TextDirection::LTR,
        });

        let mut text_style = TextStyle::new();
        text_style.set_font_size(run.font_size);
        text_style.set_color(run.color);
        let families = font_resolver.font_families_for_shaping(if run.family.is_empty() {
            None
        } else {
            Some(run.family)
        });
        let family_refs = families.iter().map(String::as_str).collect::<Vec<_>>();
        if !family_refs.is_empty() {
            text_style.set_font_families(&family_refs);
        }
        text_style.set_font_style(FontStyle::new(
            Weight::from(i32::from(run.weight)),
            Width::NORMAL,
            if run.italic {
                Slant::Italic
            } else {
                Slant::Upright
            },
        ));
        if run.line_height > 0.0 {
            text_style.set_height(run.line_height);
            text_style.set_height_override(true);
            text_style.set_half_leading(true);
        }

        let mut builder = ParagraphBuilder::new(&paragraph_style, (*collection).clone());
        builder.push_style(&text_style);
        builder.add_text(run.text);
        builder.pop();
        builder.build()
    }
}

fn build_collection(font_resolver: &FontResolver) -> FontCollection {
    let mut collection = FontCollection::new();
    collection.set_default_font_manager(font_resolver.ordered_font_manager(), Some(DEFAULT_FAMILY));
    if let Some(provider) = jian_skia::bundled_fonts::asset_provider() {
        collection.set_asset_font_manager(Some(provider.into()));
    }
    collection
}
