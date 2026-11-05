//! Text-run mapping for browser snapshots.
//!
//! The capture records the browser's own text box: the rect a `Range` around
//! the run reported, measured with the page's fonts. OpenPencil re-renders
//! that run with whatever font it can resolve (usually a fallback), and a
//! fallback is routinely a few percent wider per glyph. Pinning the node to
//! the captured width therefore clipped the tail off every run that the
//! browser had fitted exactly — "openpencil" rendered as "openpenci",
//! ".vscode" as ".vscod".
//!
//! The fix keeps the captured box as a *floor*, not a ceiling, and uses the
//! captured line-box count to decide which axis may grow:
//!
//! - single line → hug the text, with `min_width` holding the captured width
//!   so alignment and neighbouring geometry stay put while a wider fallback
//!   simply extends the box;
//! - multiple lines → keep the captured width (so the run wraps where the
//!   browser wrapped it) and let the height grow instead, with `min_height`
//!   holding the captured height.

use std::collections::BTreeMap;

use jian_ops_schema::node::text::{FontStyleKind, FontWeight, TextContent, TextGrowth, TextNode};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior};
use jian_ops_schema::style::{FontStyleKind as SegmentFontStyle, StyledTextSegment};
use serde_json::{Map, Value};

use super::{parse_px, parse_text_align, solid_fill, Rect, SnapshotCtx};
use crate::color::parse_css_color;

/// Sizing decided for one captured run.
struct TextBox {
    width: Option<SizingBehavior>,
    height: Option<SizingBehavior>,
    limits: SizeLimits,
    growth: TextGrowth,
}

fn text_box(rect: Rect, lines: u64, nowrap: bool) -> TextBox {
    if lines > 1 && !nowrap {
        // The browser wrapped this run. Re-wrapping at the captured width is
        // the only way to reproduce the same shape; the height is what has to
        // absorb a taller fallback, so it hugs with the captured height as a
        // floor.
        return TextBox {
            width: Some(SizingBehavior::Number(rect.w)),
            height: None,
            limits: SizeLimits {
                min_height: Some(rect.h),
                ..SizeLimits::default()
            },
            growth: TextGrowth::FixedWidth,
        };
    }
    TextBox {
        // Hug: `TextGrowth::Auto` reports the natural single-line extent, so
        // the box tracks whatever the resolved font actually needs. The
        // captured width survives as `min_width` so the box never comes out
        // *narrower* than the page's — which is what keeps a centred or
        // right-aligned run from collapsing onto its own left edge. It does
        // not re-centre anything: the run is placed by the absolute `x` the
        // capture recorded, so a wider fallback grows to the right.
        width: None,
        height: Some(SizingBehavior::Number(rect.h)),
        limits: SizeLimits {
            min_width: Some(rect.w),
            ..SizeLimits::default()
        },
        growth: TextGrowth::Auto,
    }
}

impl SnapshotCtx<'_> {
    pub(super) fn map_text(
        &mut self,
        object: &Map<String, Value>,
        rect: Rect,
        parent_rect: Rect,
    ) -> Option<PenNode> {
        let text = object.get("text").and_then(Value::as_str)?.to_string();
        if text.trim().is_empty() {
            return None;
        }
        let id = self.allocate_id()?;
        let styles = super::style_map(object);
        let font_size = styles
            .get("font-size")
            .and_then(|value| parse_px(value))
            .unwrap_or(self.opts.base_font_size);
        let font_weight = styles.get("font-weight").map(|value| {
            value
                .parse::<u32>()
                .map(FontWeight::Number)
                .unwrap_or_else(|_| FontWeight::Keyword(value.clone()))
        });
        let font_style = match styles.get("font-style").map(String::as_str) {
            Some("italic" | "oblique") => Some(FontStyleKind::Italic),
            Some("normal") => Some(FontStyleKind::Normal),
            _ => None,
        };
        let lines = line_count(object);
        let nowrap = is_nowrap(&styles);
        // The capture measures glyph boxes (a `Range`'s rects), so the page's
        // own half-leading is already baked into the captured `y` — a
        // vertically-centred footer (`line-height: 40px` on 14px text) hands
        // this importer the ~15px glyph box, not the 40px line box. Painting
        // that line-height again applies the half-leading a second time and
        // pushed such runs a dozen pixels below the captured box (while a
        // neighbouring run with a normal line-height stayed put — the footer
        // misalignment). On a run the browser kept to one line the
        // line-height is pure leading, so it is clamped to the captured box;
        // a wrapped run keeps it, because there it is the stride between
        // lines.
        let line_height = styles
            .get("line-height")
            .and_then(|value| parse_px(value))
            .filter(|_| font_size > 0.0)
            .map(|height| height / font_size);
        let line_height = if is_single_line(lines, nowrap) {
            clamp_single_line_leading(line_height, font_size, rect.h)
        } else {
            line_height
        };
        let letter_spacing = styles
            .get("letter-spacing")
            .filter(|value| value.as_str() != "normal")
            .and_then(|value| parse_px(value));
        let text_align = styles
            .get("text-align")
            .and_then(|value| parse_text_align(value));
        // Transparent glyphs under a `background-clip: text` ancestor take
        // the colour that ancestor's background moved onto the context (the
        // gradient-text idiom — see `map_element`). Transparent text WITHOUT
        // such an ancestor stays transparent: that is what the page shows.
        let fill = match styles.get("color") {
            Some(value) if is_transparent_color(value) => self
                .text_fill_override
                .clone()
                .or_else(|| parse_css_color(value))
                .map(|color| vec![solid_fill(color)]),
            Some(value) => parse_css_color(value).map(|color| vec![solid_fill(color)]),
            None => self
                .text_fill_override
                .clone()
                .map(|color| vec![solid_fill(color)]),
        };
        let sizing = text_box(rect, lines, nowrap);
        Some(PenNode::Text(TextNode {
            base: self.base(id, Some("Text".into()), rect, Some(parent_rect)),
            limits: sizing.limits,
            width: sizing.width,
            height: sizing.height,
            content: styled_content(object, text, self.text_fill_override.as_deref()),
            font_family: styles.get("font-family").cloned(),
            font_size: Some(font_size.into()),
            font_weight,
            font_style,
            letter_spacing,
            line_height,
            text_align,
            text_align_vertical: None,
            text_growth: Some(sizing.growth),
            underline: None,
            strikethrough: None,
            fill,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }))
    }
}

/// Build the text node's content from a folded inline block's `segments`.
///
/// The extractor collapses a block whose children are all inline (bare text
/// plus `<a>` / `<code>` / `<span>`) into ONE positioned text node carrying the
/// concatenated text as styled `segments`, so the run flows and wraps once
/// instead of each inline child stacking at the block origin (the overlap this
/// fixes). A payload with no `segments` (an older capture) or one that reduces
/// to a single unstyled run falls back to plain text.
fn styled_content(
    object: &Map<String, Value>,
    plain: String,
    fill_override: Option<&str>,
) -> TextContent {
    let Some(items) = object.get("segments").and_then(Value::as_array) else {
        return TextContent::Plain(plain);
    };
    let mut segments = Vec::new();
    for item in items {
        let Some(segment) = item.as_object() else {
            continue;
        };
        let Some(text) = segment.get("text").and_then(Value::as_str) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        let styles = segment.get("styles").and_then(Value::as_object);
        let style = |key: &str| styles.and_then(|map| map.get(key)).and_then(Value::as_str);
        let decoration = style("text-decoration-line").unwrap_or("");
        segments.push(StyledTextSegment {
            text: text.to_string(),
            font_family: style("font-family").map(str::to_string),
            font_size: style("font-size")
                .and_then(parse_px)
                .map(|value| value as f32),
            font_weight: style("font-weight").and_then(|value| value.parse::<u32>().ok()),
            font_style: match style("font-style") {
                Some("italic" | "oblique") => Some(SegmentFontStyle::Italic),
                Some("normal") => Some(SegmentFontStyle::Normal),
                _ => None,
            },
            // Transparent segments under a `background-clip: text` ancestor
            // take the moved background colour, like the node-level fill.
            fill: match style("color") {
                Some(value) if is_transparent_color(value) => fill_override
                    .map(str::to_string)
                    .or_else(|| parse_css_color(value)),
                other => other.and_then(parse_css_color),
            },
            underline: decoration.contains("underline").then_some(true),
            strikethrough: decoration.contains("line-through").then_some(true),
            href: segment
                .get("href")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    // A lone run carrying no overrides is indistinguishable from plain text —
    // keep the node simple so the common single-`<span>` block does not pay for
    // styled content it does not use.
    let trivial = segments.len() <= 1
        && segments.iter().all(|segment| {
            segment.font_family.is_none()
                && segment.font_size.is_none()
                && segment.font_weight.is_none()
                && segment.font_style.is_none()
                && segment.fill.is_none()
                && segment.underline.is_none()
                && segment.strikethrough.is_none()
                && segment.href.is_none()
        });
    if segments.is_empty() || trivial {
        TextContent::Plain(plain)
    } else {
        TextContent::Styled(segments)
    }
}

/// Line-box count recorded by the extractor. Payloads captured before the
/// field existed report nothing; a single line is the safe reading, because
/// hugging a run that did wrap only makes it one line wide instead of
/// clipping it.
///
/// The count comes from `Range::getClientRects().length`, which is one rect
/// per line box *plus* one per direction change inside a line, so it can
/// over-report on bidi text. Over-reporting only means the run keeps the
/// captured width instead of hugging — the conservative direction — so it is
/// deliberately not corrected for.
fn line_count(object: &Map<String, Value>) -> u64 {
    object
        .get("lines")
        .and_then(Value::as_u64)
        .filter(|lines| *lines > 0)
        .unwrap_or(1)
}

/// The two spellings a captured fully-transparent glyph colour arrives in
/// (Chrome computes `transparent` as `rgba(0, 0, 0, 0)`; console captures on
/// other engines may keep the keyword).
fn is_transparent_color(value: &str) -> bool {
    value == "rgba(0, 0, 0, 0)" || value.eq_ignore_ascii_case("transparent")
}

/// Whether `text_box` treats the run as single-line (the hug branch).
fn is_single_line(lines: u64, nowrap: bool) -> bool {
    lines <= 1 || nowrap
}

/// Cap a single-line run's line-height at the captured glyph box so paint
/// does not re-apply half-leading the page already positioned (see the
/// comment at the call site). A missing or already-tight line-height passes
/// through.
fn clamp_single_line_leading(line_height: Option<f64>, font_size: f64, rect_h: f64) -> Option<f64> {
    line_height.map(|leading| {
        if font_size > 0.0 && rect_h > 0.0 {
            leading.min(rect_h / font_size)
        } else {
            leading
        }
    })
}

/// Runs the browser never wrapped, so the importer must not wrap them either.
///
/// Only `nowrap` qualifies. The `pre` family (`pre`, `pre-wrap`, `pre-line`,
/// `break-spaces`) also suppresses *automatic* wrapping, but it preserves
/// newlines — and the capture collapses every `\s+` run to a single space, so
/// by the time the text reaches here a three-line `<pre>` block is one long
/// line with no break left in it. Hugging that produced a 250 px run inside a
/// 101 px slot. Treating it as wrapped instead keeps the captured width, which
/// re-breaks the text at roughly the right places. Revisit if the capture ever
/// preserves newlines.
fn is_nowrap(styles: &BTreeMap<String, String>) -> bool {
    matches!(
        styles.get("white-space").map(String::as_str),
        Some("nowrap")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: 120.0,
            h: 20.0,
        }
    }

    #[test]
    fn single_line_hugs_with_the_captured_width_as_a_floor() {
        let sizing = text_box(rect(), 1, false);
        assert!(sizing.width.is_none());
        assert_eq!(sizing.limits.min_width, Some(120.0));
        assert_eq!(sizing.growth, TextGrowth::Auto);
        assert!(matches!(sizing.height, Some(SizingBehavior::Number(h)) if h == 20.0));
    }

    #[test]
    fn wrapped_run_keeps_its_width_and_grows_in_height() {
        let sizing = text_box(rect(), 3, false);
        assert!(matches!(sizing.width, Some(SizingBehavior::Number(w)) if w == 120.0));
        assert!(sizing.height.is_none());
        assert_eq!(sizing.limits.min_height, Some(20.0));
        assert_eq!(sizing.growth, TextGrowth::FixedWidth);
    }

    #[test]
    fn nowrap_run_hugs_even_when_the_range_reported_several_rects() {
        let sizing = text_box(rect(), 2, true);
        assert!(sizing.width.is_none());
        assert_eq!(sizing.growth, TextGrowth::Auto);
    }

    /// `white-space: pre` is not `nowrap` here: the capture already collapsed
    /// the newlines that made it single-line-per-source-line, so hugging it
    /// would lay a whole `<pre>` block out on one line.
    #[test]
    fn pre_is_not_treated_as_a_single_line_hug() {
        let mut styles = BTreeMap::new();
        styles.insert("white-space".to_string(), "pre".to_string());
        assert!(!is_nowrap(&styles));
        for value in ["pre-wrap", "pre-line", "break-spaces", "normal"] {
            styles.insert("white-space".to_string(), value.to_string());
            assert!(!is_nowrap(&styles), "{value}");
        }
        styles.insert("white-space".to_string(), "nowrap".to_string());
        assert!(is_nowrap(&styles));
    }

    #[test]
    fn missing_line_count_reads_as_one_line() {
        let object = serde_json::json!({ "text": "x" });
        assert_eq!(line_count(object.as_object().unwrap()), 1);
    }

    /// The vertically-centred footer case: `line-height: 40px` on 14px text
    /// whose captured glyph box is 15.5px tall. Painting the authored
    /// leading again would push the run ~12px below the captured box.
    #[test]
    fn a_single_line_run_clamps_its_leading_to_the_captured_box() {
        let clamped = clamp_single_line_leading(Some(40.0 / 14.0), 14.0, 15.5);
        assert!((clamped.unwrap() - 15.5 / 14.0).abs() < 1e-9);
        // An already-tight leading passes through untouched.
        let tight = clamp_single_line_leading(Some(1.05), 14.0, 15.5);
        assert_eq!(tight, Some(1.05));
        assert_eq!(clamp_single_line_leading(None, 14.0, 15.5), None);
        // Degenerate geometry never divides by zero.
        assert_eq!(clamp_single_line_leading(Some(2.0), 0.0, 15.5), Some(2.0));
    }

    #[test]
    fn only_the_hug_branch_counts_as_single_line() {
        assert!(is_single_line(1, false));
        assert!(is_single_line(3, true), "nowrap hugs whatever rects say");
        assert!(!is_single_line(2, false));
    }
}
