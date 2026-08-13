//! Shared layout language for the settings modal: the per-tab opener and
//! the borderless list row.
//!
//! A tab opens with an **intro** — a section-sized title over at most one
//! muted line — and then goes straight into content. It used to open with
//! a 27 pt hero over two lines of copy, which cost a quarter of the modal
//! before the first setting; the tab strip already says which page you
//! are on, so the headline was saying it twice, loudly.
//!
//! Below the intro, settings are full-width rows separated by hairlines,
//! not tinted cards: label left, control right, two row-height constants
//! (one-line and two-line) from [`crate::widgets::agent_settings_metrics`].
//! Every surface in the modal uses the same two, so a provider row, an
//! MCP toggle and a System preference are visibly the same object.
//!
//! Everything here measures through [`fit_text`] /
//! [`measure_settings_text`], the modal-shaped names for
//! [`crate::widgets::text_metrics`] — never `RenderBackend::measure_text`,
//! which is family-blind. See that module for why.

use crate::theme::Theme;
use crate::widgets::agent_settings_metrics::{ROW_H_ONE_LINE, ROW_H_TWO_LINE, ROW_TEXT_GAP};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// The font family every string in this modal is DRAWN with. Measurement
/// has to name it too — see [`fit_text`]. Aliases the workspace-wide chrome
/// family so the modal cannot drift from the rest of the editor.
pub(super) const SETTINGS_FONT_FAMILY: &str = crate::widgets::text_metrics::CHROME_FONT_FAMILY;

/// Tab intro: a section-sized title with one optional muted line under
/// it. Deliberately the same size as a section title — the intro IS a
/// section header for the page, not a poster for it.
pub(super) const INTRO_TITLE_FONT: f32 = 15.0;
pub(super) const INTRO_DESC_FONT: f32 = 12.0;
const TOUCH_INTRO_TITLE_FONT: f32 = 17.0;
const TOUCH_INTRO_DESC_FONT: f32 = 14.0;
/// Clear space above the title's ascender, and below the block's last
/// descender before the first row.
const INTRO_TOP_PAD: f32 = 2.0;
const INTRO_BOTTOM_GAP: f32 = 16.0;
/// Space between the title's descender and the muted line's ascender.
const INTRO_LINE_GAP: f32 = 3.0;

pub(super) const ROW_LABEL_FONT: f32 = 14.0;
pub(super) const ROW_DESC_FONT: f32 = 12.0;
/// Section titles sit a step under row labels and take the muted colour:
/// they name a group, they do not compete with it.
pub(super) const SECTION_TITLE_FONT: f32 = 13.0;

/// Nominal vertical metrics as a fraction of font size. Row geometry has
/// to be computable without a backend (hit-test paths have no painter),
/// so the box maths uses these instead of querying real font metrics.
/// They run slightly generous, and the row's vertical pad carries the
/// slack.
pub(super) const ASCENT_RATIO: f32 = 0.78;
pub(super) const DESCENT_RATIO: f32 = 0.22;
/// Space between the label's descender and the second line's ascender.
/// This gap — not the box height — is what keeps a two-line row from
/// reading as one blob, which is the defect every past squeeze shipped.
const ROW_LINE_GAP: f32 = 3.0;

const LABEL_ASCENT: f32 = ROW_LABEL_FONT * ASCENT_RATIO;
const LABEL_DESCENT: f32 = ROW_LABEL_FONT * DESCENT_RATIO;
const LABEL_INK: f32 = LABEL_ASCENT + LABEL_DESCENT;
const DESC_ASCENT: f32 = ROW_DESC_FONT * ASCENT_RATIO;
const DESC_DESCENT: f32 = ROW_DESC_FONT * DESCENT_RATIO;
const DESC_INK: f32 = DESC_ASCENT + DESC_DESCENT;
#[derive(Debug, Clone, Copy, PartialEq)]
struct IntroMetrics {
    title_font: f32,
    desc_font: f32,
}

const fn intro_metrics(touch: bool) -> IntroMetrics {
    if touch {
        IntroMetrics {
            title_font: TOUCH_INTRO_TITLE_FONT,
            desc_font: TOUCH_INTRO_DESC_FONT,
        }
    } else {
        IntroMetrics {
            title_font: INTRO_TITLE_FONT,
            desc_font: INTRO_DESC_FONT,
        }
    }
}

const fn intro_title_baseline(metrics: IntroMetrics) -> f32 {
    INTRO_TOP_PAD + metrics.title_font * ASCENT_RATIO
}

const fn intro_desc_baseline(metrics: IntroMetrics) -> f32 {
    intro_title_baseline(metrics)
        + metrics.title_font * DESCENT_RATIO
        + INTRO_LINE_GAP
        + metrics.desc_font * ASCENT_RATIO
}

/// How many lines of text a row carries. The row's BOX HEIGHT follows
/// from this — one shared `ROW_HEIGHT` for both shapes is what let a
/// two-line row's second line escape its box and collide with the row
/// under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowLines {
    /// Label only.
    One,
    /// Label plus a description or status line.
    Two,
}

/// Vertical pad above the first line's ascender inside each row shape.
/// Each shape centres its own ink stack in its own box, so a one-line row
/// is not a two-line row with the bottom line missing — it is a shorter
/// row with its label optically centred.
const ONE_LINE_VPAD: f32 = (ROW_H_ONE_LINE - LABEL_INK) / 2.0;
const TWO_LINE_VPAD: f32 = (ROW_H_TWO_LINE - (LABEL_INK + ROW_LINE_GAP + DESC_INK)) / 2.0;

/// Baseline of the label on a TWO-line row, measured from the row's top.
pub(super) const ROW_LABEL_BASELINE: f32 = TWO_LINE_VPAD + LABEL_ASCENT;
/// Baseline of the label on a ONE-line row.
pub(super) const ROW_LABEL_BASELINE_ONE: f32 = ONE_LINE_VPAD + LABEL_ASCENT;
/// Baseline of a two-line row's second line.
pub(super) const ROW_SECOND_LINE_BASELINE: f32 =
    ROW_LABEL_BASELINE + LABEL_DESCENT + ROW_LINE_GAP + DESC_ASCENT;

/// Baseline of the row's first line for the shape it is.
pub(super) const fn row_label_baseline(lines: RowLines) -> f32 {
    match lines {
        RowLines::One => ROW_LABEL_BASELINE_ONE,
        RowLines::Two => ROW_LABEL_BASELINE,
    }
}

/// Box height for a row carrying `lines` lines. The numbers themselves
/// live in the modal's spacing scale so every surface shares them.
pub(super) const fn row_height(lines: RowLines) -> f32 {
    match lines {
        RowLines::One => ROW_H_ONE_LINE,
        RowLines::Two => ROW_H_TWO_LINE,
    }
}

/// Uniform stride for the lists whose rows are all single-line (the MCP
/// CLI toggles). Mixed lists must walk [`row_rect_in`] instead.
pub(super) const ROW_HEIGHT: f32 = ROW_H_ONE_LINE;
pub(super) use crate::widgets::agent_settings_metrics::{SECTION_GAP, SECTION_HEADER_H};
/// Footnote line under a row list (the `*` caveat under the CLI toggles).
pub(super) const FOOTNOTE_H: f32 = 24.0;
pub(super) const FOOTNOTE_FONT: f32 = 12.0;
/// Diameter of the leading dot on a row's status line.
const STATUS_DOT: f32 = 7.0;

/// Ellipsize `text` to `max_w` measured in the family the modal actually
/// paints with. The modal-shaped name for
/// [`crate::widgets::text_metrics::fit_chrome`], which carries the full
/// account of why the family-blind call is a trap.
pub(super) fn fit_text(cx: &mut PaintCx<'_>, text: &str, max_w: f32, font_size: f32) -> String {
    crate::widgets::text_metrics::fit_chrome(cx.backend, text, max_w, font_size)
}

/// Width of `text` in the family the modal paints with — the centring
/// twin of [`fit_text`]. Centring a label on a family-blind width leaves
/// it visibly off-centre wherever the drawn family is wider.
pub(super) fn measure_settings_text(cx: &mut PaintCx<'_>, text: &str, font_size: f32) -> f32 {
    crate::widgets::text_metrics::measure_chrome(cx.backend, text, font_size)
}

/// Height of a tab's intro block — the offset from the content viewport's
/// top to the first row below it. One source for paint, hit-test, and the
/// content-height walk on every tab.
pub(super) const fn tab_intro_height(has_desc: bool) -> f32 {
    tab_intro_height_for_ui(has_desc, false)
}

/// Density-aware intro height. Touch paint and all body offsets consume this
/// same calculation so increasing legibility cannot make the following rows
/// overlap the intro.
pub(super) const fn tab_intro_height_for_ui(has_desc: bool, touch: bool) -> f32 {
    let metrics = intro_metrics(touch);
    if has_desc {
        intro_desc_baseline(metrics) + metrics.desc_font * DESCENT_RATIO + INTRO_BOTTOM_GAP
    } else {
        intro_title_baseline(metrics) + metrics.title_font * DESCENT_RATIO + INTRO_BOTTOM_GAP
    }
}

/// Paint a tab's intro at the top of `content`: a section-sized title and
/// at most one muted line. Both are ellipsized to the content width so a
/// long translation can't run past the modal edge.
pub(super) fn paint_tab_intro(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    content: Rect,
    title: &str,
    desc: Option<&str>,
) {
    paint_tab_intro_for_ui(cx, theme, content, title, desc, false);
}

/// Density-aware tab intro paint. Desktop keeps the legacy 15/12 typography;
/// touch surfaces use 17/14 while sharing baselines with
/// [`tab_intro_height_for_ui`].
pub(super) fn paint_tab_intro_for_ui(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    content: Rect,
    title: &str,
    desc: Option<&str>,
    touch: bool,
) {
    let metrics = intro_metrics(touch);
    let title_text = fit_text(cx, title, content.size.x, metrics.title_font);
    let title_layout = TextLayout::single_run(
        &title_text,
        SETTINGS_FONT_FAMILY,
        metrics.title_font,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(
            content.origin.x,
            content.origin.y + intro_title_baseline(metrics),
        ),
    );
    let Some(desc) = desc else {
        return;
    };
    let text = fit_text(cx, desc, content.size.x, metrics.desc_font);
    let layout = TextLayout::single_run(
        &text,
        SETTINGS_FONT_FAMILY,
        metrics.desc_font,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            content.origin.x,
            content.origin.y + intro_desc_baseline(metrics),
        ),
    );
}

/// Section title inside the [`SECTION_HEADER_H`] band starting at `y` —
/// small, muted, vertically centred. One primitive so the four surfaces
/// that open a section (Agents providers, built-in agents, ACP agents,
/// MCP integrations) cannot disagree about its size or colour.
pub(super) fn paint_section_title(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    title: &str,
) {
    let layout = TextLayout::single_run(
        title,
        SETTINGS_FONT_FAMILY,
        SECTION_TITLE_FONT,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(x, section_title_baseline(y)));
}

/// Baseline a section title sits on, given the top of its header band.
pub(super) fn section_title_baseline(band_top: f32) -> f32 {
    let ink = SECTION_TITLE_FONT * (ASCENT_RATIO + DESCENT_RATIO);
    band_top + (SECTION_HEADER_H - ink) / 2.0 + SECTION_TITLE_FONT * ASCENT_RATIO
}

/// Full-width row `index` in a list of uniform single-line rows.
pub(super) fn row_rect(content: Rect, top: f32, index: usize) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, top + index as f32 * ROW_HEIGHT),
        size: Point2D::new(content.size.x, ROW_HEIGHT),
    }
}

/// Row `index` in a list whose rows differ in height. Sums the boxes
/// ahead of it, so paint and hit-test walk the same ladder and a row can
/// never be laid out at a height other than the one its content needs.
pub(super) fn row_rect_in(content: Rect, top: f32, kinds: &[RowLines], index: usize) -> Rect {
    let y = top
        + kinds
            .iter()
            .take(index)
            .map(|kind| row_height(*kind))
            .sum::<f32>();
    let height = kinds.get(index).copied().unwrap_or(RowLines::One);
    Rect {
        origin: Point2D::new(content.origin.x, y),
        size: Point2D::new(content.size.x, row_height(height)),
    }
}

/// Total height of a mixed-height row list.
pub(super) fn rows_block_height(kinds: &[RowLines]) -> f32 {
    kinds.iter().map(|kind| row_height(*kind)).sum()
}

/// How much horizontal room a row's text column has: from `text_x` to
/// the left edge of whatever the row reserves on the right, minus the
/// clear space between them.
///
/// Exported because a caller that paints an extra run beside the label
/// (the ACP row's transport badge) has to fit the label against the same
/// budget the label painter used — computing it twice, slightly
/// differently, is how the badge ends up floating away from the name it
/// belongs to.
pub(super) fn row_text_budget(row: Rect, text_x: f32, reserved: f32) -> f32 {
    (row.origin.x + row.size.x - reserved - ROW_TEXT_GAP - text_x).max(0.0)
}

/// Right-aligned, vertically centred control slot inside `row`.
pub(super) fn row_control_rect(row: Rect, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            row.origin.x + row.size.x - w,
            row.origin.y + (row.size.y - h) / 2.0,
        ),
        size: Point2D::new(w, h),
    }
}

/// Hairline along a row's bottom edge — the separator that replaces the
/// old per-setting card fill. Callers skip it on the last row of a list.
pub(super) fn paint_row_hairline(cx: &mut PaintCx<'_>, theme: &Theme, row: Rect) {
    let y = row.origin.y + row.size.y;
    cx.backend.stroke_line(
        Point2D::new(row.origin.x, y),
        Point2D::new(row.origin.x + row.size.x, y),
        theme.border,
        1.0,
    );
}

/// Row label, optionally with a muted description under it. `reserved` is
/// the width the row's control occupies on the right, so the text
/// ellipsizes before it collides.
///
/// Whether the row is a one-line or a two-line box is read off `desc`:
/// a described row takes the two-line baselines, a bare label is centred
/// in the shorter box. Painting a bare label on the two-line baseline is
/// what left one-line rows visibly top-heavy.
pub(super) fn paint_row_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    row: Rect,
    label: &str,
    desc: Option<&str>,
    reserved: f32,
) {
    paint_row_label_at(cx, theme, row, row.origin.x, label, desc, reserved);
}

/// [`paint_row_label`] with the text column pushed in to `text_x` — the
/// shape a row with a leading avatar needs.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_row_label_at(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    row: Rect,
    text_x: f32,
    label: &str,
    desc: Option<&str>,
    reserved: f32,
) {
    let lines = if desc.is_some() {
        RowLines::Two
    } else {
        RowLines::One
    };
    let budget = row_text_budget(row, text_x, reserved);
    let label_text = fit_text(cx, label, budget, ROW_LABEL_FONT);
    let label_layout = TextLayout::single_run(
        &label_text,
        SETTINGS_FONT_FAMILY,
        ROW_LABEL_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(text_x, row.origin.y + row_label_baseline(lines)),
    );
    if let Some(desc) = desc {
        let desc_text = fit_text(cx, desc, budget, ROW_DESC_FONT);
        let desc_layout = TextLayout::single_run(
            &desc_text,
            SETTINGS_FONT_FAMILY,
            ROW_DESC_FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &desc_layout,
            Point2D::new(text_x, row.origin.y + ROW_SECOND_LINE_BASELINE),
        );
    }
}

/// Label for a row whose second line the caller paints itself (a status
/// readout, which needs its own colour and leading dot). Pins the label
/// to the two-line baseline so it does not sit centred as if it were the
/// row's only line — which is what made the MCP server row's title and
/// its "Running" line collide.
pub(super) fn paint_row_label_above_status(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    row: Rect,
    label: &str,
    reserved: f32,
) {
    paint_row_label_above_status_at(cx, theme, row, row.origin.x, label, reserved);
}

/// [`paint_row_label_above_status`] with an indented text column.
pub(super) fn paint_row_label_above_status_at(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    row: Rect,
    text_x: f32,
    label: &str,
    reserved: f32,
) {
    let budget = row_text_budget(row, text_x, reserved);
    let text = fit_text(cx, label, budget, ROW_LABEL_FONT);
    let layout = TextLayout::single_run(
        &text,
        SETTINGS_FONT_FAMILY,
        ROW_LABEL_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(text_x, row.origin.y + ROW_LABEL_BASELINE),
    );
}

/// Leading-dot status line in a row's description slot. Used wherever a
/// row answers "is this thing on" — the MCP server row, the System
/// auto-update row, the built-in agent rows — so all read identically.
pub(super) fn paint_row_status_line(
    cx: &mut PaintCx<'_>,
    row: Rect,
    text: &str,
    color: crate::Color,
) {
    paint_row_status_line_at(cx, row, row.origin.x, text, color);
}

/// [`paint_row_status_line`] with an indented text column, fitted to
/// `budget` so a long status can't run under the row's right-hand
/// control.
pub(super) fn paint_row_status_line_at_fitted(
    cx: &mut PaintCx<'_>,
    row: Rect,
    text_x: f32,
    text: &str,
    color: crate::Color,
    reserved: f32,
) {
    let budget = (row_text_budget(row, text_x, reserved) - STATUS_DOT - 7.0).max(0.0);
    let text = fit_text(cx, text, budget, ROW_DESC_FONT);
    paint_row_status_line_at(cx, row, text_x, &text, color);
}

fn paint_row_status_line_at(
    cx: &mut PaintCx<'_>,
    row: Rect,
    text_x: f32,
    text: &str,
    color: crate::Color,
) {
    // Dot sits on the second line's optical centre; the text shares the
    // description baseline so a status row and a described row line up.
    cx.backend.fill_oval(
        Rect {
            origin: Point2D::new(
                text_x,
                row.origin.y + ROW_SECOND_LINE_BASELINE - STATUS_DOT + 1.0,
            ),
            size: Point2D::new(STATUS_DOT, STATUS_DOT),
        },
        color,
    );
    let layout = TextLayout::single_run(
        text,
        SETTINGS_FONT_FAMILY,
        ROW_DESC_FONT,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            text_x + STATUS_DOT + 7.0,
            row.origin.y + ROW_SECOND_LINE_BASELINE,
        ),
    );
}

/// Muted footnote under a row list.
pub(super) fn paint_footnote(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    content: Rect,
    y: f32,
    text: &str,
) {
    let text = fit_text(cx, text, content.size.x, FOOTNOTE_FONT);
    let layout = TextLayout::single_run(
        &text,
        SETTINGS_FONT_FAMILY,
        FOOTNOTE_FONT,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(content.origin.x, y + 18.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_hero_constant_matches_the_shared_tab_intro() {
        // The Agents tab publishes its intro height as a constant so host
        // tests can anchor to it; every other tab derives one from
        // `tab_intro_height`. They must be the same block.
        assert_eq!(
            crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT,
            tab_intro_height(true)
        );
    }

    #[test]
    fn a_described_intro_is_exactly_one_muted_line_taller() {
        assert_eq!(
            tab_intro_height(true) - tab_intro_height(false),
            INTRO_LINE_GAP + DESC_ASCENT + DESC_DESCENT
        );
    }

    #[test]
    fn intro_density_preserves_desktop_and_expands_touch_geometry() {
        assert_eq!(tab_intro_height_for_ui(true, false), tab_intro_height(true));
        assert_eq!(
            tab_intro_height_for_ui(false, false),
            tab_intro_height(false)
        );
        assert_eq!(tab_intro_height(true), 48.0);
        assert!((tab_intro_height_for_ui(true, true) - 52.0).abs() < 0.01);
        assert!(tab_intro_height_for_ui(false, true) > tab_intro_height(false));
    }

    #[test]
    fn both_row_shapes_centre_their_ink_in_their_own_box() {
        // A one-line row is not a two-line row with the bottom line
        // missing — each shape's ink stack sits optically centred in the
        // box it actually gets, which is what "紧凑不等于挤" means here.
        let one_top = ROW_LABEL_BASELINE_ONE - LABEL_ASCENT;
        let one_bottom = row_height(RowLines::One) - (ROW_LABEL_BASELINE_ONE + LABEL_DESCENT);
        assert!(
            (one_top - one_bottom).abs() < 0.01,
            "one-line row is off-centre: {one_top} above, {one_bottom} below"
        );

        let two_top = ROW_LABEL_BASELINE - LABEL_ASCENT;
        let two_bottom = row_height(RowLines::Two) - (ROW_SECOND_LINE_BASELINE + DESC_DESCENT);
        assert!(
            (two_top - two_bottom).abs() < 0.01,
            "two-line row is off-centre: {two_top} above, {two_bottom} below"
        );
    }

    #[test]
    fn the_two_lines_of_a_two_line_row_never_touch() {
        // The regression that keeps coming back is not "the box is too
        // short" — it is that the box got shorter while the baselines
        // stayed put, so the label's descender crossed the description's
        // ascender. Assert the ink gap directly, in the same units the
        // row box is built from.
        let label_ink_bottom = ROW_LABEL_BASELINE + LABEL_DESCENT;
        let desc_ink_top = ROW_SECOND_LINE_BASELINE - DESC_ASCENT;
        assert!(
            desc_ink_top - label_ink_bottom >= ROW_LINE_GAP - 0.01,
            "the two lines of a two-line row are {}px apart — under {ROW_LINE_GAP}px they read as one blob",
            desc_ink_top - label_ink_bottom
        );
        // …and the whole stack still lives inside the box.
        let ink_top = ROW_LABEL_BASELINE - LABEL_ASCENT;
        let ink_bottom = ROW_SECOND_LINE_BASELINE + DESC_DESCENT;
        assert!(ink_top >= 0.0);
        assert!(ink_bottom <= row_height(RowLines::Two));
    }
}

#[cfg(test)]
mod fit_tests {
    use super::*;
    use crate::widgets::agent_settings_panel::AgentSettingsPanel;
    use crate::widgets::{PaintCx, Widget};
    use crate::{Point2D, Rect, RenderBackend};
    use op_editor_core::EditorState;

    use crate::widgets::test_family_gap_backend::FamilyGapBackend;

    #[test]
    fn fit_text_measures_in_the_family_it_will_be_drawn_in() {
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let long = "A".repeat(200);
        let fitted = fit_text(&mut cx, &long, 100.0, 13.0);

        assert!(fitted.ends_with('…'), "an over-wide line must ellipsize");
        let painted = cx
            .backend
            .measure_text_family(&fitted, 13.0, SETTINGS_FONT_FAMILY);
        assert!(
            painted <= 100.0,
            "fitted width {painted} must fit the 100px column in the PAINTED family"
        );
    }

    #[test]
    fn touch_intro_paint_uses_17_and_14_point_metrics_inside_its_height() {
        let content = Rect::xywh(12.0, 20.0, 320.0, 200.0);
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_tab_intro_for_ui(
            &mut cx,
            &crate::theme::Theme::dark(),
            content,
            "Title",
            Some("Description"),
            true,
        );

        assert_eq!(backend.runs.len(), 2);
        assert_eq!(backend.runs[0].font_size, TOUCH_INTRO_TITLE_FONT);
        assert_eq!(backend.runs[1].font_size, TOUCH_INTRO_DESC_FONT);
        let desc_ink_bottom = backend.runs[1].origin.y + TOUCH_INTRO_DESC_FONT * DESCENT_RATIO;
        assert!(
            desc_ink_bottom
                <= content.origin.y + tab_intro_height_for_ui(true, true) - INTRO_BOTTOM_GAP
        );
    }

    #[test]
    fn agents_hero_lines_fit_the_content_column() {
        // The provider roll is the longest string the modal generates and
        // the one that shipped sheared in half; assert every hero line
        // lands inside the content column measured the way it is painted.
        let mut state = EditorState::default();
        state.editor_ui.locale = op_i18n::Locale::EnUs;
        let panel = AgentSettingsPanel::for_editor(&state);
        let rect = panel.rect(1200.0, 800.0);
        let content = crate::widgets::agent_settings_panel::content_viewport(rect);
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        let hero: Vec<(String, f32)> = backend
            .runs
            .iter()
            .filter(|run| {
                (run.font_size - INTRO_DESC_FONT).abs() < 0.01
                    && (run.origin.x - content.origin.x).abs() < 0.01
                    && run.origin.y <= content.origin.y + tab_intro_height(true)
            })
            .map(|run| (run.text.clone(), run.font_size))
            .collect();
        assert!(
            !hero.is_empty(),
            "the Agents intro should paint a muted line"
        );
        for (text, size) in hero {
            let w = backend.measure_text_family(&text, size, SETTINGS_FONT_FAMILY);
            assert!(
                w <= content.size.x,
                "hero line {text:?} is {w}px wide in a {}px column",
                content.size.x
            );
        }
    }

    #[test]
    fn a_row_with_a_status_line_uses_the_two_line_baselines() {
        // The MCP server row draws its status line itself. If its label
        // took the single-line centred baseline the two lines collided —
        // that is the squeeze the shipped build showed.
        let row = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(400.0, row_height(RowLines::Two)),
        };
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let theme = crate::theme::Theme::dark();

        paint_row_label_above_status(&mut cx, &theme, row, "MCP Server", 0.0);
        paint_row_status_line(&mut cx, row, "Running", theme.status_success);

        let label_y = backend.runs[0].origin.y;
        let status_y = backend.runs[1].origin.y;
        let leading = status_y - label_y;
        assert!(
            leading >= ROW_LABEL_FONT,
            "label and status baselines are {leading}px apart — under one label line-height they read as one blob"
        );
        // Both lines have to live inside the row box, ascender to descender.
        assert!(label_y - ROW_LABEL_FONT >= row.origin.y);
        assert!(status_y + ROW_DESC_FONT * 0.3 <= row.origin.y + row.size.y);
    }

    #[test]
    fn described_rows_and_status_rows_share_one_rhythm() {
        let row = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(400.0, row_height(RowLines::Two)),
        };
        let theme = crate::theme::Theme::dark();

        let mut a = FamilyGapBackend::default();
        let mut cx = PaintCx { backend: &mut a };
        paint_row_label(&mut cx, &theme, row, "Label", Some("Description"), 0.0);

        let mut b = FamilyGapBackend::default();
        let mut cx = PaintCx { backend: &mut b };
        paint_row_label_above_status(&mut cx, &theme, row, "Label", 0.0);
        paint_row_status_line(&mut cx, row, "Running", theme.status_success);

        assert_eq!(
            a.runs[0].origin.y, b.runs[0].origin.y,
            "label baselines must agree"
        );
        assert_eq!(
            a.runs[1].origin.y, b.runs[1].origin.y,
            "second-line baselines must agree"
        );
    }
}
