//! Greedy CJK/Latin line wrapping for canvas text nodes and overlays,
//! split out of `canvas_viewport_overlay.rs` to keep both modules
//! cohesive and under the 800-line ceiling.

use super::canvas_text_kinsoku::carry_for_line_start;

/// Greedy line-wrap for canvas text nodes with explicit width.
/// Splits on `\n` first (mirrors `pen-renderer/text-renderer.ts`
/// which pre-splits before calling `wrapLine`), then wraps each
/// segment via the per-character-CJK / word-Latin algorithm from
/// `pen-renderer/paint-utils.ts::wrapLine`. Empty segments survive
/// as blank lines so authored paragraph breaks paint as gaps.
#[cfg(test)]
pub(super) fn wrap_text(
    backend: &mut dyn crate::RenderBackend,
    text: &str,
    font_size: f32,
    max_w: f32,
    weight: u16,
    letter_spacing: f32,
) -> Vec<String> {
    wrap_text_in_family(
        backend,
        text,
        &WrapFont {
            font_size,
            weight,
            letter_spacing,
            family: "",
            italic: false,
        },
        max_w,
    )
}

/// The font a wrap is measured in. Line breaks MUST be decided in the
/// family the painter draws with: a family-blind measure resolves the
/// backend's default face, and when the drawn family is wider (a
/// monospace display figure in "DM Mono") the line that "fits" by the
/// blind measure paints past its box — the jian layout (family-aware)
/// reserved two lines while paint drew one overflowing line.
pub(super) struct WrapFont<'a> {
    pub(super) font_size: f32,
    pub(super) weight: u16,
    pub(super) letter_spacing: f32,
    /// Empty = the backend default face (the historical family-blind path).
    pub(super) family: &'a str,
    pub(super) italic: bool,
}

/// [`wrap_text`] measured in `font`'s family (see [`WrapFont`]).
pub(super) fn wrap_text_in_family(
    backend: &mut dyn crate::RenderBackend,
    text: &str,
    font: &WrapFont<'_>,
    max_w: f32,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for segment in text.split('\n') {
        if max_w <= 0.0 || measure_text_with_letter_spacing(backend, segment, font) <= max_w {
            out.push(segment.to_string());
            continue;
        }
        wrap_segment(backend, segment, font, max_w, &mut out);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Wrap a single newline-free segment.
fn wrap_segment(
    backend: &mut dyn crate::RenderBackend,
    segment: &str,
    font: &WrapFont<'_>,
    max_w: f32,
    out: &mut Vec<String>,
) {
    let chars: Vec<char> = segment.chars().collect();
    let mut current = String::new();
    let mut probe = String::with_capacity(segment.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if is_cjk(ch) {
            let mut ch_buf = [0; 4];
            build_probe(&mut probe, &current, ch.encode_utf8(&mut ch_buf));
            if measure_text_with_letter_spacing(backend, &probe, font) > max_w
                && !current.is_empty()
            {
                let carried = carry_for_line_start(&mut current, ch);
                out.push(std::mem::take(&mut current));
                current = carried;
                current.push(ch);
            } else {
                std::mem::swap(&mut current, &mut probe);
            }
            i += 1;
        } else if ch == ' ' {
            build_probe(&mut probe, &current, " ");
            if measure_text_with_letter_spacing(backend, &probe, font) > max_w
                && !current.is_empty()
            {
                out.push(std::mem::take(&mut current));
            } else {
                std::mem::swap(&mut current, &mut probe);
            }
            i += 1;
        } else {
            let mut word = String::new();
            while i < chars.len() && chars[i] != ' ' && !is_cjk(chars[i]) {
                word.push(chars[i]);
                i += 1;
            }
            build_probe(&mut probe, &current, &word);
            if measure_text_with_letter_spacing(backend, &probe, font) > max_w
                && !current.is_empty()
            {
                let carried = word
                    .chars()
                    .next()
                    .map(|first| carry_for_line_start(&mut current, first))
                    .unwrap_or_default();
                out.push(std::mem::take(&mut current));
                current = carried;
                current.push_str(&word);
            } else {
                std::mem::swap(&mut current, &mut probe);
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
}

fn measure_text_with_letter_spacing(
    backend: &mut dyn crate::RenderBackend,
    text: &str,
    font: &WrapFont<'_>,
) -> f32 {
    let base = if font.family.is_empty() {
        backend.measure_text_weighted(text, font.font_size, font.weight)
    } else {
        backend.measure_text_family_styled(
            text,
            font.font_size,
            font.family,
            font.weight,
            font.italic,
        )
    };
    let gaps = text.chars().count().saturating_sub(1) as f32;
    (base + gaps * font.letter_spacing).max(0.0)
}

fn build_probe(probe: &mut String, current: &str, suffix: &str) {
    probe.clear();
    probe.push_str(current);
    probe.push_str(suffix);
}

/// Codepoints that wrap per-character (no inter-word spaces).
/// Mirrors `pen-core/layout/text-measure.ts::isCjkCodePoint`.
fn is_cjk(ch: char) -> bool {
    let c = ch as u32;
    (0x4e00..=0x9fff).contains(&c)
        || (0x3400..=0x4dbf).contains(&c)
        || (0x3040..=0x30ff).contains(&c)
        || (0xac00..=0xd7af).contains(&c)
        || (0x3000..=0x303f).contains(&c)
        || (0xff00..=0xffef).contains(&c)
}

#[cfg(test)]
mod wrap_tests {
    use super::*;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    /// Backend that measures every char as 10 px wide regardless of
    /// glyph — keeps the test arithmetic exact.
    struct UniformBackend;
    impl RenderBackend for UniformBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn fill_oval(&mut self, _: Rect, _: Color) {}
        fn stroke_oval(&mut self, _: Rect, _: Color, _: f32) {}
        fn fill_polygon(&mut self, _: &[Point2D], _: Color) {}
        fn stroke_polygon(&mut self, _: &[Point2D], _: Color, _: f32) {}
        fn rotate(&mut self, _: f32, _: Point2D) {}
        fn fill_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: f32, _: Color) {}
        fn measure_text(&mut self, text: &str, _: f32) -> f32 {
            text.chars().count() as f32 * 10.0
        }
        fn measure_text_weighted(&mut self, text: &str, _: f32, _: u16) -> f32 {
            text.chars().count() as f32 * 10.0
        }
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn ascii_word_breaks_at_word_boundary() {
        let mut b = UniformBackend;
        // "hello world foo" = 15 chars × 10 = 150 px. Budget 100.
        // Greedy walk: "hello " (60) fits, "hello world" (110)
        // exceeds — flush "hello ", start with "world". Then
        // "world " (60), "world foo" (90) fits.
        let lines = wrap_text(&mut b, "hello world foo", 13.0, 100.0, 400, 0.0);
        assert_eq!(lines, vec!["hello ", "world foo"]);
    }

    #[test]
    fn a_trailing_full_stop_never_starts_a_line() {
        let mut b = UniformBackend;
        let lines = wrap_text(&mut b, "一二三四五。", 13.0, 50.0, 400, 0.0);
        assert_eq!(lines, vec!["一二三四", "五。"]);
    }

    #[test]
    fn cjk_breaks_per_character() {
        let mut b = UniformBackend;
        // 8 CJK chars × 10 px = 80. Budget 50 forces a break every
        // 5 chars. Pure split_whitespace would treat the whole
        // string as one word and return a single 80-px line.
        let lines = wrap_text(&mut b, "中文测试段落很长哦", 13.0, 50.0, 400, 0.0);
        assert!(lines.len() >= 2, "got {:?}", lines);
        for line in &lines {
            assert!(line.chars().count() <= 5, "line too long: {:?}", line);
        }
    }

    #[test]
    fn explicit_newlines_become_line_breaks() {
        let mut b = UniformBackend;
        // "ab\ncd\nef" — each segment fits the 100 px budget, but
        // we still split on `\n`. Pure wrap (no newline handling)
        // would join everything as "abcdef" and return one line.
        let lines = wrap_text(&mut b, "ab\ncd\nef", 13.0, 100.0, 400, 0.0);
        assert_eq!(lines, vec!["ab", "cd", "ef"]);
    }

    #[test]
    fn blank_lines_survive_in_output() {
        let mut b = UniformBackend;
        // "para1\n\npara2" — the empty segment between the two
        // paragraphs must paint as a blank line so authored gaps
        // survive.
        let lines = wrap_text(&mut b, "para1\n\npara2", 13.0, 200.0, 400, 0.0);
        assert_eq!(lines, vec!["para1", "", "para2"]);
    }

    #[test]
    fn newlines_split_before_wrap_runs() {
        let mut b = UniformBackend;
        // "hello world\nfoo bar baz" with budget 100. First segment
        // "hello world" = 110 → wraps to ["hello ", "world"].
        // Second segment "foo bar baz" = 110 → ["foo bar ", "baz"].
        let lines = wrap_text(&mut b, "hello world\nfoo bar baz", 13.0, 100.0, 400, 0.0);
        assert_eq!(lines, vec!["hello ", "world", "foo bar ", "baz"]);
    }

    #[test]
    fn cjk_latin_mix_breaks_within_cjk_run() {
        let mut b = UniformBackend;
        let lines = wrap_text(&mut b, "hello 中文段落", 13.0, 60.0, 400, 0.0);
        assert!(lines.len() >= 2, "got {:?}", lines);
        assert!(lines[0].starts_with("hello"));
    }

    /// Stub backend whose advance widths depend on the weight —
    /// 10 px per char at weight 400, 14 px per char at weight 700.
    /// Mirrors the real native backend behaviour where bold glyphs
    /// have wider advances; if wrap_text ignored weight, the same
    /// string would break at a different line index.
    struct WeightedBackend;
    impl RenderBackend for WeightedBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn fill_oval(&mut self, _: Rect, _: Color) {}
        fn stroke_oval(&mut self, _: Rect, _: Color, _: f32) {}
        fn fill_polygon(&mut self, _: &[Point2D], _: Color) {}
        fn stroke_polygon(&mut self, _: &[Point2D], _: Color, _: f32) {}
        fn rotate(&mut self, _: f32, _: Point2D) {}
        fn fill_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: f32, _: Color) {}
        fn measure_text(&mut self, text: &str, _: f32) -> f32 {
            text.chars().count() as f32 * 10.0
        }
        fn measure_text_weighted(&mut self, text: &str, _: f32, weight: u16) -> f32 {
            let per_char = if weight >= 600 { 14.0 } else { 10.0 };
            text.chars().count() as f32 * per_char
        }
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn wrap_uses_weighted_advances() {
        // "hello world" at weight 700 = 11 chars × 14 = 154 px.
        // Budget 120 forces a break ("hello " = 6×14=84 fits, +5×14=70 doesn't).
        // At weight 400 (110 px) it'd fit on one line — codex's
        // BLOCK: wrap was measuring at 400 while paint used 700.
        let mut b = WeightedBackend;
        let bold = wrap_text(&mut b, "hello world", 13.0, 120.0, 700, 0.0);
        let regular = wrap_text(&mut b, "hello world", 13.0, 120.0, 400, 0.0);
        assert_eq!(regular, vec!["hello world"], "weight 400 fits");
        assert!(bold.len() >= 2, "weight 700 must wrap: {:?}", bold);
    }

    #[test]
    fn wrap_includes_negative_letter_spacing() {
        let mut b = UniformBackend;
        // Base width is 110 px; ten -1 px gaps bring it to exactly 100 px.
        let lines = wrap_text(&mut b, "hello world", 13.0, 100.0, 400, -1.0);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn probe_builder_reuses_existing_string_allocation() {
        let mut probe = String::with_capacity(64);
        let ptr = probe.as_ptr();

        build_probe(&mut probe, "hello", " world");

        assert_eq!(probe, "hello world");
        assert_eq!(
            probe.as_ptr(),
            ptr,
            "probe construction should reuse the existing String allocation"
        );
    }
}
