//! Presenting toolbar — the controls shown while Preview runs a deck.
//!
//! A pill at the bottom of the viewport carrying `◀  3 / 6  ▶  ✕`: step
//! back, where you are, step forward, and leave. It stays visible rather
//! than appearing on hover, because a presenter needs to know the controls
//! are there without hunting for them, and the pill is small enough that a
//! slide loses nothing to it.
//!
//! Icons and numerals only, deliberately: nothing here needs translating,
//! so no locale key can be missing and no label can be shown in the wrong
//! language.
//!
//! Geometry is computed WITHOUT measuring text — the counter's width comes
//! from its character count — so paint and the host's hit-test derive the
//! same rects from the same inputs. Measurement only affects where the
//! label is centred inside its own segment, never where the segment is.

use op_editor_core::preview_slideshow::SlideshowToolbarButton;

use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout, Theme};

/// Every control is a real 44pt touch target, not a small desktop icon whose
/// hit rect only happens to be larger.
const HEIGHT: f32 = 44.0;
const RADIUS: f32 = 22.0;
const BUTTON_W: f32 = 44.0;
/// Floor for the counter segment; wider labels grow it (see [`counter_width`]).
const COUNTER_MIN_W: f32 = 62.0;
/// Upper-bound advance per counter character at [`FONT_SIZE`]. Digits and
/// `" / "` all measure under this, so the segment can never be too narrow
/// for its label.
const COUNTER_CHAR_W: f32 = 8.0;
const COUNTER_PAD_X: f32 = 14.0;
const BOTTOM_MARGIN: f32 = 16.0;
const ICON_SIZE: f32 = 18.0;
const ICON_STROKE: f32 = 1.6;
const FONT_SIZE: f32 = 13.0;
/// The board shows through the pill: it reads as an overlay on the slide
/// rather than a strip of chrome cutting into it.
const PILL_ALPHA: f32 = 0.92;
/// How far a disabled control fades. Visible enough to show the control
/// exists — the deck simply has nothing that way — without inviting a click.
const DISABLED_ALPHA: f32 = 0.35;

/// Width of the counter segment for a given label, from character count
/// alone so hit-test and paint agree without a backend.
fn counter_width(label: &str) -> f32 {
    COUNTER_MIN_W.max(label.chars().count() as f32 * COUNTER_CHAR_W + COUNTER_PAD_X * 2.0)
}

pub struct SlideshowToolbar<'a> {
    /// Already-formatted position, e.g. `"3 / 6"` — see
    /// `op_editor_core::preview_slideshow::SlideshowState::counter_label`.
    pub label: &'a str,
    /// Whether a board exists before / after the one on screen. Both come
    /// from the slideshow itself, so a control never looks live at an end of
    /// the deck where pressing it would do nothing.
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub hover: Option<SlideshowToolbarButton>,
    pub pressed: Option<SlideshowToolbarButton>,
}

impl SlideshowToolbar<'_> {
    /// Outer pill rect, centred on the bottom edge of `canvas`.
    pub fn pill_rect(canvas: Rect, label: &str) -> Rect {
        let width = BUTTON_W * 3.0 + counter_width(label);
        Rect {
            origin: Point2D::new(
                canvas.origin.x + (canvas.size.x - width) / 2.0,
                canvas.origin.y + canvas.size.y - HEIGHT - BOTTOM_MARGIN,
            ),
            size: Point2D::new(width, HEIGHT),
        }
    }

    /// The three control rects, left to right, with the counter between the
    /// step buttons. The counter is not a control and has no rect here — a
    /// press on it resolves to `None` and does nothing, which is what makes
    /// the middle of the pill safe to click.
    pub fn button_rects(canvas: Rect, label: &str) -> [(SlideshowToolbarButton, Rect); 3] {
        let pill = Self::pill_rect(canvas, label);
        let segment = |x: f32, w: f32| Rect {
            origin: Point2D::new(x, pill.origin.y),
            size: Point2D::new(w, HEIGHT),
        };
        let previous_x = pill.origin.x;
        let next_x = previous_x + BUTTON_W + counter_width(label);
        let exit_x = next_x + BUTTON_W;
        [
            (
                SlideshowToolbarButton::Previous,
                segment(previous_x, BUTTON_W),
            ),
            (SlideshowToolbarButton::Next, segment(next_x, BUTTON_W)),
            (SlideshowToolbarButton::Exit, segment(exit_x, BUTTON_W)),
        ]
    }

    /// Which control `point` lands on, if any.
    ///
    /// Ends of the deck are NOT filtered here: a press on a disabled step
    /// button still resolves, so the pill swallows it instead of letting it
    /// fall through and advance the slide underneath. Whether the press does
    /// anything is the caller's decision, taken from the same
    /// `can_go_back` / `can_go_forward` facts the paint uses.
    pub fn hit_test(canvas: Rect, label: &str, point: Point2D) -> Option<SlideshowToolbarButton> {
        Self::button_rects(canvas, label)
            .into_iter()
            .find_map(|(button, rect)| rect_contains(rect, point).then_some(button))
    }

    /// Whether `point` is anywhere on the pill, including the counter and
    /// the gaps. The presenting press path uses this to decide a press is
    /// toolbar business and must not advance the deck.
    pub fn contains(canvas: Rect, label: &str, point: Point2D) -> bool {
        rect_contains(Self::pill_rect(canvas, label), point)
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, canvas: Rect, theme: &Theme) {
        let pill = Self::pill_rect(canvas, self.label);
        cx.backend.fill_round_rect(
            pill,
            RADIUS,
            Color {
                a: PILL_ALPHA,
                ..theme.popover
            },
        );
        cx.backend
            .stroke_round_rect(pill, RADIUS, theme.border, 1.0);

        // Exit is intentionally a visible terminal segment. It remains a
        // calm neutral surface, but the divider and persistent wash make the
        // way out discoverable on touch without turning it into an alarm.
        let exit_rect = Self::button_rects(canvas, self.label)[2].1;
        cx.backend.fill_round_rect(
            inset(exit_rect, 4.0),
            RADIUS - 4.0,
            Color {
                a: 0.12,
                ..theme.button_hover
            },
        );
        cx.backend.stroke_line(
            Point2D::new(exit_rect.origin.x, exit_rect.origin.y + 10.0),
            Point2D::new(
                exit_rect.origin.x,
                exit_rect.origin.y + exit_rect.size.y - 10.0,
            ),
            theme.border,
            1.0,
        );

        for (button, rect) in Self::button_rects(canvas, self.label) {
            let enabled = self.enabled(button);
            if enabled && (self.pressed == Some(button) || self.hover == Some(button)) {
                cx.backend
                    .fill_round_rect(inset(rect, 4.0), RADIUS - 4.0, theme.button_hover);
            }
            let color = if enabled {
                theme.foreground
            } else {
                Color {
                    a: DISABLED_ALPHA,
                    ..theme.foreground
                }
            };
            draw_icon(
                cx.backend,
                icon_for(button),
                Point2D::new(
                    rect.origin.x + (rect.size.x - ICON_SIZE) / 2.0,
                    rect.origin.y + (rect.size.y - ICON_SIZE) / 2.0,
                ),
                ICON_SIZE,
                color,
                ICON_STROKE,
            );
        }

        let text_width = text_metrics::measure_chrome(cx.backend, self.label, FONT_SIZE);
        let counter_x = pill.origin.x + BUTTON_W + (counter_width(self.label) - text_width) / 2.0;
        let label = TextLayout::single_run(
            self.label,
            "system-ui",
            FONT_SIZE,
            theme.foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(
                counter_x,
                pill.origin.y + HEIGHT / 2.0 + FONT_SIZE / 2.0 - 1.5,
            ),
        );
    }

    /// Whether a control does anything right now. Exit always does.
    fn enabled(&self, button: SlideshowToolbarButton) -> bool {
        match button {
            SlideshowToolbarButton::Previous => self.can_go_back,
            SlideshowToolbarButton::Next => self.can_go_forward,
            SlideshowToolbarButton::Exit => true,
        }
    }
}

fn icon_for(button: SlideshowToolbarButton) -> Icon {
    match button {
        SlideshowToolbarButton::Previous => Icon::ChevronLeft,
        SlideshowToolbarButton::Next => Icon::ChevronRight,
        SlideshowToolbarButton::Exit => Icon::Close,
    }
}

fn rect_contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

fn inset(rect: Rect, by: f32) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + by, rect.origin.y + by),
        size: Point2D::new(rect.size.x - by * 2.0, rect.size.y - by * 2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Rect {
        Rect {
            origin: Point2D::new(100.0, 40.0),
            size: Point2D::new(1000.0, 700.0),
        }
    }

    fn midpoint(rect: Rect) -> Point2D {
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        )
    }

    #[test]
    fn the_pill_is_centred_on_the_bottom_edge() {
        let pill = SlideshowToolbar::pill_rect(canvas(), "3 / 6");
        assert_eq!(pill.size.y, HEIGHT);
        assert_eq!(pill.size.x, BUTTON_W * 3.0 + counter_width("3 / 6"));
        let pill_centre = pill.origin.x + pill.size.x / 2.0;
        assert!((pill_centre - 600.0).abs() < 0.5, "centre {pill_centre}");
        assert!((pill.origin.y - (740.0 - HEIGHT - BOTTOM_MARGIN)).abs() < 0.5);
    }

    #[test]
    fn every_control_has_a_true_44_point_touch_target() {
        for (_, rect) in SlideshowToolbar::button_rects(canvas(), "3 / 6") {
            assert!(rect.size.x >= 44.0, "touch width was {}", rect.size.x);
            assert!(rect.size.y >= 44.0, "touch height was {}", rect.size.y);
        }
    }

    #[test]
    fn the_toolbar_stays_inside_short_and_narrow_safe_local_stages() {
        for canvas in [
            Rect::xywh(0.0, 0.0, 320.0, 180.0),
            Rect::xywh(0.0, 0.0, 390.0, 763.0),
            Rect::xywh(0.0, 0.0, 1_024.0, 744.0),
        ] {
            let pill = SlideshowToolbar::pill_rect(canvas, "12 / 120");
            assert!(pill.origin.x >= canvas.origin.x);
            assert!(pill.origin.y >= canvas.origin.y);
            assert!(pill.origin.x + pill.size.x <= canvas.origin.x + canvas.size.x);
            assert!(pill.origin.y + pill.size.y <= canvas.origin.y + canvas.size.y);
        }
    }

    #[test]
    fn a_long_counter_widens_the_pill_without_moving_it_off_centre() {
        let short = SlideshowToolbar::pill_rect(canvas(), "3 / 6");
        let long = SlideshowToolbar::pill_rect(canvas(), "100 / 120");
        assert!(long.size.x > short.size.x, "a longer label needs more room");
        let centre = |rect: Rect| rect.origin.x + rect.size.x / 2.0;
        assert!((centre(long) - centre(short)).abs() < 0.5);
    }

    #[test]
    fn each_control_resolves_and_the_counter_resolves_to_nothing() {
        let label = "3 / 6";
        let rects = SlideshowToolbar::button_rects(canvas(), label);
        assert_eq!(rects[0].0, SlideshowToolbarButton::Previous);
        assert_eq!(rects[1].0, SlideshowToolbarButton::Next);
        assert_eq!(rects[2].0, SlideshowToolbarButton::Exit);
        for (button, rect) in rects {
            assert_eq!(
                SlideshowToolbar::hit_test(canvas(), label, midpoint(rect)),
                Some(button)
            );
        }

        // The counter sits between the two step buttons and is not a control.
        let pill = SlideshowToolbar::pill_rect(canvas(), label);
        let counter_centre = Point2D::new(
            pill.origin.x + BUTTON_W + counter_width(label) / 2.0,
            pill.origin.y + HEIGHT / 2.0,
        );
        assert_eq!(
            SlideshowToolbar::hit_test(canvas(), label, counter_centre),
            None,
            "pressing the counter must not act"
        );
        assert!(
            SlideshowToolbar::contains(canvas(), label, counter_centre),
            "but it is still the toolbar, so the press is the toolbar's"
        );
    }

    #[test]
    fn a_press_off_the_pill_belongs_to_the_board() {
        let label = "3 / 6";
        let away = Point2D::new(150.0, 100.0);
        assert_eq!(SlideshowToolbar::hit_test(canvas(), label, away), None);
        assert!(!SlideshowToolbar::contains(canvas(), label, away));
    }

    #[test]
    fn a_disabled_step_button_still_resolves_so_the_pill_swallows_it() {
        // Otherwise a press on the greyed-out "previous" at board 0 would
        // fall through to the board underneath and advance the deck.
        let label = "1 / 6";
        let previous = SlideshowToolbar::button_rects(canvas(), label)[0].1;
        assert_eq!(
            SlideshowToolbar::hit_test(canvas(), label, midpoint(previous)),
            Some(SlideshowToolbarButton::Previous)
        );
    }

    #[test]
    fn exit_paints_as_a_persistent_segment_even_without_hover() {
        use crate::widgets::test_capture_backend::CaptureBackend;

        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let toolbar = SlideshowToolbar {
            label: "3 / 6",
            can_go_back: true,
            can_go_forward: true,
            hover: None,
            pressed: None,
        };
        toolbar.paint(&mut cx, canvas(), &Theme::dark());

        let exit = SlideshowToolbar::button_rects(canvas(), "3 / 6")[2].1;
        let persistent_wash = inset(exit, 4.0);
        assert!(
            backend
                .round_fills
                .iter()
                .any(|(rect, _, _)| *rect == persistent_wash),
            "the exit target needs a visible surface before hover"
        );
        assert!(
            backend.svg_strokes.iter().any(|(_, origin, size, _, _)| {
                origin.x >= exit.origin.x
                    && origin.x + size <= exit.origin.x + exit.size.x
                    && origin.y >= exit.origin.y
                    && origin.y + size <= exit.origin.y + exit.size.y
            }),
            "the exit glyph must paint inside its touch target"
        );
    }
}
