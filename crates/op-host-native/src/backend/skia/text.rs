//! Text rendering for `NativeBackend` — typeface resolution caches,
//! per-script segmentation, weighted/styled measurement and
//! `draw_text`. Carved out of `skia.rs` (800-line cap) when italic
//! support landed; the typeface caches stay fields on the spine
//! struct, this sibling only houses the `impl` block.

use op_editor_ui::{Point2D, TextLayout};

use super::font_script;
use super::{NativeBackend, ROBOTO_TTF};

/// Build a skia `FontStyle` from the canonical schema's CSS-style
/// numeric weight (100-900) plus the slant bit. Width stays at the
/// system default.
fn font_style_for(weight: u16, italic: bool) -> skia_safe::FontStyle {
    let w = weight as i32;
    skia_safe::FontStyle::new(
        skia_safe::font_style::Weight::from(w),
        skia_safe::font_style::Width::NORMAL,
        if italic {
            skia_safe::font_style::Slant::Italic
        } else {
            skia_safe::font_style::Slant::Upright
        },
    )
}

fn primary_font_family(stack: &str) -> Option<&str> {
    let first = stack.split(',').next()?.trim().trim_matches(['"', '\'']);
    if first.is_empty()
        || matches!(
            first,
            "system-ui" | "sans-serif" | "serif" | "monospace" | "-apple-system"
        )
    {
        None
    } else {
        Some(first)
    }
}

/// Synthetic oblique skew applied when the resolved typeface has no
/// real italic variant — mirrors the synthetic-bold stroke trick.
const SYNTH_ITALIC_SKEW: f32 = -0.25;

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
        self.typeface_for_char_styled(c, weight, false)
    }

    fn typeface_for_char_styled(
        &mut self,
        c: char,
        weight: u16,
        italic: bool,
    ) -> Option<skia_safe::Typeface> {
        // ASCII at weight 400 upright stays on the bundled
        // Roboto-Regular for hot chrome paint; non-default weight,
        // italic, or non-ASCII falls through to the system FontMgr,
        // which honours weight + slant.
        if c.is_ascii() && weight == 400 && !italic {
            return self.ensure_typeface().cloned();
        }
        if font_script::is_hangul_codepoint(c) {
            if let Some(tf) = self.ensure_korean_typeface().cloned() {
                return Some(tf);
            }
        } else if font_script::is_east_asian_codepoint(c) {
            // CJK families rarely ship a real italic; the cached face
            // is reused and `draw_text` skews it synthetically.
            return self.ensure_cjk_typeface().cloned();
        }
        let cp = c as i32;
        let key = (cp, weight, italic);
        if let Some(cached) = self.char_typeface_cache.get(&key) {
            return cached.clone();
        }
        let style = font_style_for(weight, italic);
        let tf = self
            .font_mgr
            .match_family_style_character("", style, &[], cp);
        let resolved = tf.or_else(|| {
            // CJK fallback path doesn't yet vary by weight — TS app
            // synthesises bold via paint stroke when the family is
            // weight-locked. Mirror that downstream of `draw_text`
            // with `Paint::set_stroke_width` when needed.
            self.ensure_cjk_typeface().cloned()
        });
        self.char_typeface_cache.insert(key, resolved.clone());
        resolved
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
        self.typeface_for_family_char_styled(c, family, weight, false)
    }

    fn typeface_for_family_char_styled(
        &mut self,
        c: char,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> Option<skia_safe::Typeface> {
        let Some(primary) = primary_font_family(family) else {
            return self.typeface_for_char_styled(c, weight, italic);
        };
        if font_script::is_hangul_codepoint(c) {
            if let Some(tf) = self.ensure_korean_typeface().cloned() {
                return Some(tf);
            }
        } else if font_script::is_east_asian_codepoint(c) {
            return self.ensure_cjk_typeface().cloned();
        }
        let key = (primary.to_string(), c as i32, weight, italic);
        if let Some(cached) = self.family_typeface_cache.get(&key) {
            return cached.clone();
        }
        let style = font_style_for(weight, italic);
        let resolved = self
            .font_mgr
            .match_family_style_character(primary, style, &[], c as i32)
            .or_else(|| self.typeface_for_char_styled(c, weight, italic));
        self.family_typeface_cache.insert(key, resolved.clone());
        resolved
    }

    /// Split `text` into contiguous segments that share a typeface,
    /// preserving char order. Glyphs without any covering typeface
    /// are bucketed with the previous segment so they at least
    /// occupy space (rather than disappearing).
    fn segment_text_styled(
        &mut self,
        text: &str,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> Vec<(skia_safe::Typeface, String)> {
        let mut segments: Vec<(skia_safe::Typeface, String)> = Vec::new();
        for c in text.chars() {
            let tf = self.typeface_for_family_char_styled(c, family, weight, italic);
            let Some(tf) = tf else {
                if let Some(last) = segments.last_mut() {
                    last.1.push(c);
                }
                continue;
            };
            match segments.last_mut() {
                Some(last) if last.0.unique_id() == tf.unique_id() => last.1.push(c),
                _ => segments.push((tf, c.to_string())),
            }
        }
        segments
    }

    /// Lazy-init the Step 4 cached Roboto typeface (ASCII path).
    fn ensure_typeface(&mut self) -> Option<&skia_safe::Typeface> {
        if !self.typeface_tried {
            self.typeface = self.font_mgr.new_from_data(ROBOTO_TTF, None);
            self.typeface_tried = true;
        }
        self.typeface.as_ref()
    }

    /// Lazy-resolve a system typeface that has CJK glyph coverage.
    /// Picks whichever font the system FontMgr would use for the
    /// canonical Han ideograph U+4E00 — on macOS this is PingFang SC,
    /// on Linux it's Noto Sans CJK, on Windows it's Microsoft YaHei
    /// or similar. Cached for the lifetime of the backend so we
    /// don't pay the FontMgr lookup more than once.
    fn ensure_cjk_typeface(&mut self) -> Option<&skia_safe::Typeface> {
        if !self.cjk_typeface_tried {
            self.cjk_typeface = self.font_mgr.match_family_style_character(
                "",
                skia_safe::FontStyle::default(),
                &[],
                '一' as i32,
            );
            self.cjk_typeface_tried = true;
        }
        self.cjk_typeface.as_ref()
    }

    /// Lazy-init the cached Korean (Hangul) typeface, resolved from a
    /// Hangul syllable so the OS picks a Hangul-covering face (e.g.
    /// Apple SD Gothic Neo / Noto Sans KR) rather than the Chinese
    /// font the Han-ideograph match returns.
    fn ensure_korean_typeface(&mut self) -> Option<&skia_safe::Typeface> {
        if !self.korean_typeface_tried {
            self.korean_typeface = self.font_mgr.match_family_style_character(
                "",
                skia_safe::FontStyle::default(),
                &[],
                '한' as i32,
            );
            self.korean_typeface_tried = true;
        }
        self.korean_typeface.as_ref()
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
        let segments = self.segment_text_styled(text, "", weight, italic);
        if segments.is_empty() {
            return 0.0;
        }
        let mut advance = 0.0_f32;
        for (typeface, segment) in segments {
            let font = skia_safe::Font::new(&typeface, font_size);
            let (a, _) = font.measure_str(&segment, None);
            advance += a;
        }
        advance
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
        let runs: Vec<_> = layout.runs().to_vec();
        for run in runs {
            let segments = self.segment_text_styled(
                run.content.as_str(),
                &run.font_family,
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
            let synth_bold = run.font_weight >= 600;
            if synth_bold {
                paint.set_style(skia_safe::PaintStyle::StrokeAndFill);
                paint.set_stroke_width(run.font_size * 0.06);
            }
            let mut x = origin.x + run.origin.x;
            let y = origin.y + run.origin.y;
            for (typeface, segment) in segments {
                let mut font = skia_safe::Font::new(&typeface, run.font_size);
                if italic && !typeface.is_italic() {
                    font.set_skew_x(SYNTH_ITALIC_SKEW);
                }
                canvas.draw_str(&segment, (x, y), &font, &paint);
                let (advance, _) = font.measure_str(&segment, None);
                x += advance;
            }
        }
    }
}
