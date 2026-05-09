use crate::Rect;
use skia_bindings as sb;
use std::{
    cmp::{max, min},
    ops::Range,
};

pub use sb::skia_textlayout_Affinity as Affinity;
variant_name!(Affinity::Downstream);

#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub enum RectHeightStyle {
    /// Provide tight bounding boxes that fit heights per run.
    #[default]
    Tight,
    // The height of the boxes will be the maximum height of all runs in the
    // line. All rects in the same line will be the same height.
    Max,
    // Extends the top and/or bottom edge of the bounds to fully cover any line
    // spacing. The top edge of each line should be the same as the bottom edge
    // of the line above. There should be no gaps in vertical coverage given any
    // ParagraphStyle line_height.
    //
    // The top and bottom of each rect will cover half of the
    // space above and half of the space below the line.
    IncludeLineSpacingMiddle,
    // The line spacing will be added to the top of the rect.
    IncludeLineSpacingTop,
    // The line spacing will be added to the bottom of the rect.
    IncludeLineSpacingBottom,
    Strut,
}
native_transmutable!(sb::skia_textlayout_RectHeightStyle, RectHeightStyle);

#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub enum RectWidthStyle {
    /// Provide tight bounding boxes that fit widths to the runs of each line
    /// independently.
    #[default]
    Tight,
    /// Extends the width of the last rect of each line to match the position of
    /// the widest rect over all the lines.
    Max,
}
native_transmutable!(sb::skia_textlayout_RectWidthStyle, RectWidthStyle);

pub use sb::skia_textlayout_TextAlign as TextAlign;
variant_name!(TextAlign::End);
pub use sb::skia_textlayout_TextDirection as TextDirection;
variant_name!(TextDirection::LTR);

pub use sb::skia_textlayout_PositionWithAffinity as PositionWithAffinity;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct TextBox {
    pub rect: Rect,
    pub direct: TextDirection,
}

native_transmutable!(sb::skia_textlayout_TextBox, TextBox);

pub const EMPTY_INDEX: usize = usize::MAX;

pub trait RangeExtensions {
    fn width(&self) -> usize;
    fn shift(&mut self, d: isize);
    fn contains(&self, other: &Self) -> bool;
    fn intersects(&self, other: &Self) -> bool;
    #[must_use]
    fn intersection(&self, other: &Self) -> Self;
    fn empty(&self) -> bool;
}

impl RangeExtensions for Range<usize> {
    fn width(&self) -> usize {
        self.end - self.start
    }

    fn shift(&mut self, d: isize) {
        if d >= 0 {
            let u = d as usize;
            self.start += u;
            self.end += u;
        } else {
            let u = -d as usize;
            self.start -= u;
            self.end -= u;
        }
    }

    fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    fn intersects(&self, other: &Self) -> bool {
        max(self.start, other.start) <= min(self.end, other.end)
    }

    fn intersection(&self, other: &Self) -> Self {
        Self {
            start: max(self.start, other.start),
            end: min(self.end, other.end),
        }
    }

    fn empty(&self) -> bool {
        self.start == EMPTY_INDEX && self.end == EMPTY_INDEX
    }
}

#[allow(clippy::reversed_empty_ranges)]
pub const EMPTY_RANGE: Range<usize> = Range {
    start: EMPTY_INDEX,
    end: EMPTY_INDEX,
};

pub use sb::skia_textlayout_TextBaseline as TextBaseline;
variant_name!(TextBaseline::Alphabetic);

pub use sb::skia_textlayout_TextHeightBehavior as TextHeightBehavior;
variant_name!(TextHeightBehavior::DisableFirstAscent);

// m84: LineMetricStyle is declared but not used in the public API yet.
