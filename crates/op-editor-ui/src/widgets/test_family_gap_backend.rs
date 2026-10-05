//! Test-only `RenderBackend` whose named-family measurement is 40% wider
//! than its family-blind one — the shape of the real native gap, where
//! `measure_text` resolves the bundled Roboto but a `system-ui` run paints
//! as `.AppleSystemUIFont` (SF Pro).
//!
//! Every other test backend measures blind and family-aware identically, so
//! a widget that fits / centres / sizes against `RenderBackend::measure_text`
//! looks perfect under them and shears on a real machine. Painting a panel
//! into this backend reproduces the machine: a fitter that measured blind
//! emits a string too wide for its column, and
//! [`PaintedRun::width_in_paint_family`] catches it.
//!
//! The cross-panel guard in `text_metrics_paint_tests` is the intended
//! consumer — assert every captured run fits the column it was painted into.

use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

/// Advance as a fraction of font size for the backend's *default* typeface —
/// what `measure_text` reports. Matches jian's own fallback heuristic
/// (`Painter::measure_text`: `font_size * 0.55` per ASCII char, full size for
/// everything else) so the blind number here is the number a font-less
/// environment really produces.
const BLIND_ASCII_RATIO: f32 = 0.55;
/// Same, for any *named* family — what `draw_text` will actually paint.
/// Deliberately 40% wider, the shape of the Roboto → SF Pro gap.
const FAMILY_ASCII_RATIO: f32 = 0.77;
/// Non-ASCII advances are ~1em, and they are the SAME in both faces on
/// purpose: a CJK / Cyrillic / Devanagari glyph resolves through the same
/// system fallback whichever Latin family the run names, so there is no gap
/// to model there. The Roboto → SF Pro gap is a Latin-advance phenomenon,
/// and inflating non-Latin here would manufacture failures no machine shows.
const WIDE_RATIO: f32 = 1.0;

fn width(text: &str, font_size: f32, ascii_ratio: f32) -> f32 {
    text.chars()
        .map(|c| {
            font_size
                * if c.is_ascii() {
                    ascii_ratio
                } else {
                    WIDE_RATIO
                }
        })
        .sum()
}

/// One `draw_text` call, with everything needed to re-measure it the way it
/// was painted.
#[derive(Debug, Clone)]
pub(crate) struct PaintedRun {
    pub(crate) text: String,
    pub(crate) family: String,
    pub(crate) font_size: f32,
    pub(crate) font_weight: u16,
    /// Origin passed to `draw_text` — x is the run's left edge.
    pub(crate) origin: Point2D,
    /// Innermost clip in effect when the run was drawn, if any. A run wider
    /// than this is SHEARED on screen — glyphs cut mid-stroke with no
    /// ellipsis, which is the exact symptom this whole guard exists for.
    pub(crate) clip: Option<Rect>,
    /// Advance ratio the painting backend charges a named family.
    painted_ascii_ratio: f32,
    weight_gap: f32,
}

impl PaintedRun {
    /// Width this run occupies on screen, measured in the family it names.
    pub(crate) fn width_in_paint_family(&self) -> f32 {
        let ratio = if self.family.is_empty() {
            BLIND_ASCII_RATIO
        } else {
            self.painted_ascii_ratio
        };
        width(&self.text, self.font_size, ratio) * weight_factor(self.font_weight, self.weight_gap)
    }

    /// Right edge of the painted run.
    pub(crate) fn right_edge(&self) -> f32 {
        self.origin.x + self.width_in_paint_family()
    }

    /// Whether the run spills past `container` or past its own clip. Both
    /// read the same on screen: text that does not fit the box it was put in.
    fn spills(&self, container: Rect) -> bool {
        let (mut left, mut right) = (container.origin.x, container.origin.x + container.size.x);
        if let Some(clip) = self.clip {
            left = left.max(clip.origin.x);
            right = right.min(clip.origin.x + clip.size.x);
        }
        self.origin.x < left - 0.01 || self.right_edge() > right + 0.01
    }
}

pub(crate) struct FamilyGapBackend {
    /// Every `draw_text`, in paint order.
    pub(crate) runs: Vec<PaintedRun>,
    /// Live clip stack — `save` / `restore` / `clip_rect` are modelled so a
    /// run's recorded clip is the one actually in effect when it was drawn.
    clip_stack: Vec<Option<Rect>>,
    /// Advance ratio charged for a NAMED family. `FAMILY_ASCII_RATIO` is the
    /// real machine; `BLIND_ASCII_RATIO` is the control where the two faces
    /// agree, which is what every other test backend models.
    family_ascii_ratio: f32,
    weight_gap: f32,
}

impl Default for FamilyGapBackend {
    fn default() -> Self {
        Self {
            runs: Vec::new(),
            clip_stack: vec![None],
            family_ascii_ratio: FAMILY_ASCII_RATIO,
            weight_gap: 0.0,
        }
    }
}

impl FamilyGapBackend {
    /// Model a wider bold face while leaving regular-family metrics intact.
    pub(crate) fn weighted() -> Self {
        Self {
            weight_gap: 0.001,
            ..Self::default()
        }
    }

    /// The control: named families measure exactly like the default face, so
    /// nothing a family-aware fitter does can change the outcome. Paint a
    /// panel into this and into [`Self::default`] and diff — see
    /// `text_metrics_paint_tests`.
    pub(crate) fn uniform() -> Self {
        Self {
            family_ascii_ratio: BLIND_ASCII_RATIO,
            ..Self::default()
        }
    }

    /// Runs that do not fit the box they were painted into — either
    /// `container` or, where one is in effect, their own clip.
    pub(crate) fn overflowing(&self, container: Rect) -> Vec<&PaintedRun> {
        self.runs
            .iter()
            .filter(|run| run.spills(container))
            .collect()
    }

    fn clip(&self) -> Option<Rect> {
        self.clip_stack.last().copied().flatten()
    }
}

impl RenderBackend for FamilyGapBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, point: Point2D) {
        for run in layout.runs() {
            self.runs.push(PaintedRun {
                text: run.content.clone(),
                family: run.font_family.clone(),
                font_size: run.font_size,
                font_weight: run.font_weight,
                origin: Point2D::new(point.x + run.origin.x, point.y + run.origin.y),
                clip: self.clip(),
                painted_ascii_ratio: self.family_ascii_ratio,
                weight_gap: self.weight_gap,
            });
        }
    }
    fn clip_rect(&mut self, rect: Rect) {
        let merged = match self.clip() {
            Some(current) => intersect_x(current, rect),
            None => rect,
        };
        if let Some(top) = self.clip_stack.last_mut() {
            *top = Some(merged);
        }
    }
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        width(text, font_size, BLIND_ASCII_RATIO)
    }
    fn measure_text_family(&mut self, text: &str, font_size: f32, family: &str) -> f32 {
        let ratio = if family.is_empty() {
            BLIND_ASCII_RATIO
        } else {
            self.family_ascii_ratio
        };
        width(text, font_size, ratio)
    }
    fn measure_text_family_styled(
        &mut self,
        text: &str,
        font_size: f32,
        family: &str,
        weight: u16,
        _: bool,
    ) -> f32 {
        self.measure_text_family(text, font_size, family) * weight_factor(weight, self.weight_gap)
    }
    fn save(&mut self) {
        self.clip_stack.push(self.clip());
    }
    fn restore(&mut self) {
        if self.clip_stack.len() > 1 {
            self.clip_stack.pop();
        }
    }
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn weight_factor(weight: u16, gap: f32) -> f32 {
    1.0 + weight.saturating_sub(400) as f32 * gap
}

/// Horizontal intersection of two clips. Only x matters here — every guard
/// this backend serves asks "does the text fit its column".
fn intersect_x(a: Rect, b: Rect) -> Rect {
    let left = a.origin.x.max(b.origin.x);
    let right = (a.origin.x + a.size.x).min(b.origin.x + b.size.x);
    Rect::xywh(left, a.origin.y, (right - left).max(0.0), a.size.y)
}
