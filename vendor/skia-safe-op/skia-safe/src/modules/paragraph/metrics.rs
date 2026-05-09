use crate::{paragraph::TextStyle, prelude::*, FontMetrics};
use skia_bindings::{self as sb, skia_textlayout_LineMetrics, skia_textlayout_StyleMetrics};
use std::{marker::PhantomData, ops::Range, ptr};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct StyleMetrics<'a> {
    pub text_style: &'a TextStyle,

    /// [`FontMetrics`] contains the following metrics:
    ///
    /// * Top                 distance to reserve above baseline
    /// * Ascent              distance to reserve below baseline
    /// * Descent             extent below baseline
    /// * Bottom              extent below baseline
    /// * Leading             distance to add between lines
    /// * AvgCharWidth        average character width
    /// * MaxCharWidth        maximum character width
    /// * XMin                minimum x
    /// * XMax                maximum x
    /// * XHeight             height of lower-case 'x'
    /// * CapHeight           height of an upper-case letter
    /// * UnderlineThickness  underline thickness
    /// * UnderlinePosition   underline position relative to baseline
    /// * StrikeoutThickness  strikeout thickness
    /// * StrikeoutPosition   strikeout position relative to baseline
    pub font_metrics: FontMetrics,
}

native_transmutable!(skia_textlayout_StyleMetrics, StyleMetrics<'_>);

impl<'a> StyleMetrics<'a> {
    pub fn new(style: &'a TextStyle, metrics: impl Into<Option<FontMetrics>>) -> Self {
        Self {
            text_style: style,
            font_metrics: metrics.into().unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LineMetrics<'a> {
    // The following fields are used in the layout process itself.
    /// The index in the text buffer the line begins.
    pub start_index: usize,
    /// The index in the text buffer the line ends.
    pub end_index: usize,
    pub end_excluding_whitespaces: usize,
    pub end_including_newline: usize,
    pub hard_break: bool,

    // The following fields are tracked after or during layout to provide to
    // the user as well as for computing bounding boxes.
    /// The final computed ascent and descent for the line. This can be impacted by
    /// the strut, height, scaling, as well as outlying runs that are very tall.
    ///
    /// The top edge is `baseline - ascent` and the bottom edge is `baseline +
    /// descent`. Ascent and descent are provided as positive numbers. Raw numbers
    /// for specific runs of text can be obtained in run_metrics_map. These values
    /// are the cumulative metrics for the entire line.
    pub ascent: f64,
    pub descent: f64,
    pub unscaled_ascent: f64,
    /// Total height of the paragraph including the current line.
    ///
    /// The height of the current line is `round(ascent + descent)`.
    pub height: f64,
    /// Width of the line.
    pub width: f64,
    /// The left edge of the line. The right edge can be obtained with `left +
    /// width`
    pub left: f64,
    /// The y position of the baseline for this line from the top of the paragraph.
    pub baseline: f64,
    /// Zero indexed line number
    pub line_number: usize,
    /// Mapping between text index ranges and the FontMetrics associated with
    /// them. The first run will be keyed under start_index. The metrics here
    /// are before layout and are the base values we calculate from.
    style_metrics: Vec<sb::IndexedStyleMetrics>,
    pd: PhantomData<&'a StyleMetrics<'a>>,
}

impl<'a> LineMetrics<'a> {
    // TODO: may support constructors (but what about the lifetime bounds?).

    /// Returns the number of style metrics in the given index range.
    pub fn get_style_metrics_count(&self, range: Range<usize>) -> usize {
        let lower = self
            .style_metrics
            .partition_point(|ism| ism.index < range.start);
        let upper = self
            .style_metrics
            .partition_point(|ism| ism.index < range.end);
        upper - lower
    }

    /// Returns indices and references to style metrics in the given range.
    pub fn get_style_metrics(&'a self, range: Range<usize>) -> Vec<(usize, &'a StyleMetrics<'a>)> {
        let lower = self
            .style_metrics
            .partition_point(|ism| ism.index < range.start);
        let upper = self
            .style_metrics
            .partition_point(|ism| ism.index < range.end);
        self.style_metrics[lower..upper]
            .iter()
            .map(|ism| (ism.index, StyleMetrics::from_native_ref(&ism.metrics)))
            .collect()
    }

    // We can't use a `std::map` in rust, it does not seem to be safe to move. So we copy it into a
    // sorted Vec.
    pub(crate) fn from_native_ref<'b>(lm: &skia_textlayout_LineMetrics) -> LineMetrics<'b> {
        let sm_count = unsafe { sb::C_LineMetrics_styleMetricsCount(lm) };
        let mut style_metrics = vec![
            sb::IndexedStyleMetrics {
                index: 0,
                metrics: sb::skia_textlayout_StyleMetrics {
                    text_style: ptr::null(),
                    font_metrics: sb::SkFontMetrics {
                        fFlags: 0,
                        fTop: 0.0,
                        fAscent: 0.0,
                        fDescent: 0.0,
                        fBottom: 0.0,
                        fLeading: 0.0,
                        fAvgCharWidth: 0.0,
                        fMaxCharWidth: 0.0,
                        fXMin: 0.0,
                        fXMax: 0.0,
                        fXHeight: 0.0,
                        fCapHeight: 0.0,
                        fUnderlineThickness: 0.0,
                        fUnderlinePosition: 0.0,
                        fStrikeoutThickness: 0.0,
                        fStrikeoutPosition: 0.0
                    }
                }
            };
            sm_count
        ];

        unsafe { sb::C_LineMetrics_getAllStyleMetrics(lm, style_metrics.as_mut_ptr()) }

        LineMetrics {
            start_index: lm.fStartIndex,
            end_index: lm.fEndIndex,
            end_excluding_whitespaces: lm.fEndExcludingWhitespaces,
            end_including_newline: lm.fEndIncludingNewline,
            hard_break: lm.fHardBreak,
            ascent: lm.fAscent,
            descent: lm.fDescent,
            unscaled_ascent: lm.fUnscaledAscent,
            height: lm.fHeight,
            width: lm.fWidth,
            left: lm.fLeft,
            baseline: lm.fBaseline,
            line_number: lm.fLineNumber,
            style_metrics,
            pd: PhantomData,
        }
    }
}
