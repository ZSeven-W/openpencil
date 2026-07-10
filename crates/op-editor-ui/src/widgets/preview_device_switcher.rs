//! Floating Preview device switcher shared by paint and hit-testing.

use op_editor_core::PreviewDeviceKind;

use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout, Theme};

const PILL_W: f32 = 216.0;
const PILL_H: f32 = 28.0;
const SEGMENT_W: f32 = 72.0;
const RADIUS: f32 = 12.0;
const TOP_GAP: f32 = 8.0;
const ORDER: [PreviewDeviceKind; 3] = [
    PreviewDeviceKind::Phone,
    PreviewDeviceKind::Desktop,
    PreviewDeviceKind::Canvas,
];

pub struct PreviewDeviceSwitcher<'a> {
    /// Labels in Phone, Desktop, Canvas order.
    pub labels: [&'a str; 3],
    pub selected: Option<PreviewDeviceKind>,
    pub hover: Option<PreviewDeviceKind>,
    pub pressed: Option<PreviewDeviceKind>,
}

impl PreviewDeviceSwitcher<'_> {
    pub fn pill_rect(canvas: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                canvas.origin.x + (canvas.size.x - PILL_W) / 2.0,
                canvas.origin.y + TOP_GAP,
            ),
            size: Point2D::new(PILL_W, PILL_H),
        }
    }

    pub fn segment_rects(canvas: Rect) -> [(PreviewDeviceKind, Rect); 3] {
        let pill = Self::pill_rect(canvas);
        let segment = |index: usize| Rect {
            origin: Point2D::new(pill.origin.x + SEGMENT_W * index as f32, pill.origin.y),
            size: Point2D::new(SEGMENT_W, PILL_H),
        };
        [
            (ORDER[0], segment(0)),
            (ORDER[1], segment(1)),
            (ORDER[2], segment(2)),
        ]
    }

    pub fn hit_test(canvas: Rect, point: Point2D) -> Option<PreviewDeviceKind> {
        Self::segment_rects(canvas)
            .into_iter()
            .find_map(|(kind, rect)| {
                (point.x >= rect.origin.x
                    && point.x <= rect.origin.x + rect.size.x
                    && point.y >= rect.origin.y
                    && point.y <= rect.origin.y + rect.size.y)
                    .then_some(kind)
            })
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, canvas: Rect, theme: &Theme) {
        let pill = Self::pill_rect(canvas);
        cx.backend.fill_round_rect(pill, RADIUS, theme.card);
        cx.backend
            .stroke_round_rect(pill, RADIUS, theme.border, 1.0);
        for (index, (kind, rect)) in Self::segment_rects(canvas).into_iter().enumerate() {
            let wash = if self.selected == Some(kind) {
                Some(theme.accent)
            } else if self.pressed == Some(kind) || self.hover == Some(kind) {
                Some(theme.muted)
            } else {
                None
            };
            if let Some(color) = wash {
                cx.backend.fill_round_rect(rect, RADIUS, color);
            }

            let label_text = self.labels[index];
            let text_width = cx.backend.measure_text(label_text, 12.0);
            let label = TextLayout::single_run(
                label_text,
                "system-ui",
                12.0,
                theme.foreground.to_jian(),
                Point2D::new(0.0, 0.0),
            );
            let text_x = rect.origin.x + (rect.size.x - text_width) / 2.0;
            let baseline_y = rect.origin.y + rect.size.y / 2.0 + 4.5;
            cx.backend
                .draw_text(&label, Point2D::new(text_x, baseline_y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point2D, Rect};
    use op_editor_core::PreviewDeviceKind;

    fn canvas() -> Rect {
        Rect {
            origin: Point2D::new(100.0, 40.0),
            size: Point2D::new(1000.0, 700.0),
        }
    }

    #[test]
    fn pill_is_216x28_centered_8px_below_top() {
        let pill = PreviewDeviceSwitcher::pill_rect(canvas());
        assert_eq!(pill.size.x, 216.0);
        assert_eq!(pill.size.y, 28.0);
        assert!((pill.origin.x - (100.0 + (1000.0 - 216.0) / 2.0)).abs() < 0.5);
        assert_eq!(pill.origin.y, 48.0);
    }

    #[test]
    fn three_segments_of_72_and_hit_test() {
        let segments = PreviewDeviceSwitcher::segment_rects(canvas());
        assert_eq!(segments.len(), 3);
        for (_, rect) in &segments {
            assert_eq!(rect.size.x, 72.0);
            assert_eq!(rect.size.y, 28.0);
        }
        assert_eq!(segments[0].0, PreviewDeviceKind::Phone);
        assert_eq!(segments[1].0, PreviewDeviceKind::Desktop);
        assert_eq!(segments[2].0, PreviewDeviceKind::Canvas);
        let midpoint = |rect: Rect| {
            Point2D::new(
                rect.origin.x + rect.size.x / 2.0,
                rect.origin.y + rect.size.y / 2.0,
            )
        };
        assert_eq!(
            PreviewDeviceSwitcher::hit_test(canvas(), midpoint(segments[1].1)),
            Some(PreviewDeviceKind::Desktop)
        );
        assert_eq!(
            PreviewDeviceSwitcher::hit_test(canvas(), Point2D::new(0.0, 0.0)),
            None
        );
    }
}
