//! Text rendering for `NativeBackend` — typeface resolution caches,
//! per-script segmentation, weighted/styled measurement and
//! `draw_text`. Carved out of `skia.rs` (800-line cap) when italic
//! support landed; the typeface caches stay fields on the spine
//! struct, this sibling only houses the `impl` block.

use op_editor_ui::{Point2D, TextLayout};

use super::NativeBackend;

pub(super) fn draw_text_runs(layout: &TextLayout) -> &[jian_core::render::TextRun] {
    layout.runs()
}

impl NativeBackend {
    /// Resolve a typeface that covers `c`. Cached per codepoint —
    /// `FontMgr::match_family_style_character` is fast on first call
    /// but we still avoid the look-up on every chrome paint.
    /// Falls back to the cached CJK typeface, then the cached
    /// Roboto, so a worst-case missing-font system still renders
    /// something rather than dropping the glyph.
    pub(crate) fn typeface_for_char(
        &mut self,
        c: char,
        weight: u16,
    ) -> Option<skia_safe::Typeface> {
        self.font_resolver
            .typeface_for_char(None, c, weight, false)
            .map(|resolved| resolved.typeface)
    }

    /// Upright convenience wrapper — production paths go through the
    /// styled variant; the cache-behaviour tests exercise this one.
    #[cfg(test)]
    pub(crate) fn typeface_for_family_char(
        &mut self,
        c: char,
        family: &str,
        weight: u16,
    ) -> Option<skia_safe::Typeface> {
        self.font_resolver
            .typeface_for_char(Some(family), c, weight, false)
            .map(|resolved| resolved.typeface)
    }

    /// Measure the rendered horizontal advance of `text` at
    /// `font_size`. Uses the same per-script typeface dispatch as
    /// `draw_text` so the measurement matches what's painted.
    /// Falls back to a conservative heuristic when typefaces
    /// aren't available.
    pub fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.measure_text_weighted(text, font_size, 400)
    }

    /// Weight-aware text measurement. Resolves the per-codepoint
    /// typeface against `FontStyle::new(Weight, ...)` so wrap-pass
    /// line breaks decided at, say, weight 700 use the same glyph
    /// advances `draw_text` will paint with at weight 700. Without
    /// this the wrap pass measured at 400 and paint at 700 — the
    /// rendered string could then overflow the wrap budget.
    pub fn measure_text_weighted(&mut self, text: &str, font_size: f32, weight: u16) -> f32 {
        self.measure_text_styled(text, font_size, weight, false)
    }

    /// Weight + slant aware measurement — styled text-run slices use
    /// it so a run resolved to a REAL italic face measures with that
    /// face's advances (a synthetic skew keeps upright advances, so
    /// both cases stay consistent with `draw_text`).
    pub fn measure_text_styled(
        &mut self,
        text: &str,
        font_size: f32,
        weight: u16,
        italic: bool,
    ) -> f32 {
        self.measure_text_family_styled(text, font_size, "", weight, italic)
    }

    /// Like [`Self::measure_text_styled`] but resolves the per-codepoint
    /// typeface against `family` — exactly as `draw_text` does for a run
    /// carrying that family. The family-blind `measure_text*` pass `""`
    /// (bundled Roboto); chrome inputs that DRAW in a named family (e.g.
    /// "Inter") must measure with that same family so their caret /
    /// selection geometry lines up with the painted glyphs.
    pub fn measure_text_family_styled(
        &mut self,
        text: &str,
        font_size: f32,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> f32 {
        self.font_resolver
            .measure_text(text, font_size, Some(family), weight, italic)
    }

    /// Family-aware width at the default weight/upright — the measurement
    /// backing `Painter::measure_text_family` for caret positioning.
    pub fn measure_text_family(&mut self, text: &str, font_size: f32, family: &str) -> f32 {
        self.measure_text_family_styled(text, font_size, family, 400, false)
    }

    /// Render every shaped run in the layout via cached typefaces +
    /// `Canvas::draw_str` (Step 4 perf fix — see comment on the
    /// `typeface` / `cjk_typeface` fields).
    ///
    /// Two fast paths + one fallback:
    ///   - run is ASCII-only → cached Roboto + draw_str
    ///   - run contains non-ASCII → cached system CJK typeface +
    ///     draw_str (PingFang / Noto CJK / etc.). PingFang covers
    ///     Latin too, so mixed CJK + Latin runs render correctly.
    ///   - the system has no CJK font (rare; Linux without Noto)
    ///     → fall back to jian-skia's textlayout path so glyphs
    ///     don't drop. Still slow on that branch, but functional.
    ///
    /// `layout.italic()` resolves a slanted face per codepoint; when
    /// the system serves no italic variant the glyphs are skewed
    /// synthetically (same philosophy as synthetic bold).
    #[tracing::instrument(skip(self, canvas, layout))]
    pub fn draw_text(&mut self, canvas: &skia_safe::Canvas, layout: &TextLayout, origin: Point2D) {
        let italic = layout.italic();
        for run in draw_text_runs(layout) {
            let segments = self.font_resolver.segment_text(
                run.content.as_str(),
                Some(&run.font_family),
                run.font_weight,
                italic,
            );
            if segments.is_empty() {
                continue;
            }
            let jc = run.color;
            let mut paint = skia_safe::Paint::new(
                skia_safe::Color4f::new(
                    f32::from(jc.r()) / 255.0,
                    f32::from(jc.g()) / 255.0,
                    f32::from(jc.b()) / 255.0,
                    f32::from(jc.a()) / 255.0,
                ),
                None,
            );
            paint.set_anti_alias(true);
            // Synthetic bold for typefaces the system serves at one
            // weight only (notably the bundled Roboto-Regular for
            // ASCII at weight ≥600). Stroke width scales with size
            // so 28pt headline gets the same visual weight relative
            // to its glyph as 13pt body text.
            let want_bold = run.font_weight >= 600;
            let mut x = origin.x + run.origin.x;
            let y = origin.y + run.origin.y;
            for segment in segments {
                let mut font = skia_safe::Font::new(&segment.typeface, run.font_size);
                if segment.synthetic_italic {
                    font.set_skew_x(jian_skia::SYNTHETIC_ITALIC_SKEW);
                }
                if want_bold && segment.synthetic_bold {
                    paint.set_style(skia_safe::PaintStyle::StrokeAndFill);
                    paint.set_stroke_width(run.font_size * 0.06);
                } else {
                    paint.set_style(skia_safe::PaintStyle::Fill);
                    paint.set_stroke_width(0.0);
                }
                canvas.draw_str(&segment.text, (x, y), &font, &paint);
                let (mut advance, _) = font.measure_str(&segment.text, None);
                if segment.synthetic_bold {
                    advance *= jian_skia::SYNTHETIC_BOLD_WIDTH_FACTOR;
                }
                x += advance;
            }
        }
    }
}
