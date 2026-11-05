//! `TopBar`'s window-control (traffic-light) dot cluster — geometry +
//! hit-test. Split out of `top_bar.rs` to keep that file under the
//! repo's 800-line cap (same rationale as the `top_bar_paint` sibling).

use crate::widgets::top_bar::*;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

/// A window-control dot in the TopBar's left cluster. Resolved by
/// [`TopBar::window_control_at`]; the desktop runner maps each onto
/// the matching winit `Window` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControl {
    /// Red dot — close the window / quit.
    Close,
    /// Yellow dot — minimise the window.
    Minimize,
    /// Green dot — toggle maximised.
    Maximize,
}

impl TopBar {
    /// Left-edge reservation for the window controls. Collapses to
    /// `0` in fullscreen on macOS — the native traffic lights hide
    /// then, so the gap would be dead space. Other platforms keep
    /// the custom-dot cluster's inset in every mode.
    pub(super) fn left_inset_for(fullscreen: bool) -> f32 {
        if fullscreen && cfg!(target_os = "macos") {
            0.0
        } else {
            TRAFFIC_CLUSTER_W
        }
    }

    pub(super) fn left_inset(&self) -> f32 {
        if self.show_traffic_controls {
            Self::left_inset_for(self.fullscreen)
        } else {
            0.0
        }
    }

    /// Bounds of the 3-dot window-control cluster — the host's
    /// cursor-move handler uses this to drive `topbar_traffic_hover`
    /// (the glyph reveal).
    pub fn traffic_cluster_rect(top_bar_rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(top_bar_rect.origin.x + PAD, top_bar_rect.origin.y),
            size: Point2D::new(TRAFFIC_STEP * 2.0 + TRAFFIC_DOT, top_bar_rect.size.y),
        }
    }

    /// Resolve a press on the left-edge window-control dots.
    /// `None` for a press anywhere else (including the app's own
    /// buttons). The desktop runner consults this before its normal
    /// TopBar hit-test so a dot click drives the window, not the app.
    pub fn window_control_at(&self, rect: Rect, point: Point2D) -> Option<WindowControl> {
        // macOS uses the native traffic-light buttons — the custom
        // dots (and this hit-test) exist only for Windows / Linux.
        // Returning `None` here also avoids a false positive in
        // macOS fullscreen, where the left inset collapses and the
        // app's own icons would otherwise sit in the dot region.
        if !self.show_traffic_controls || cfg!(target_os = "macos") {
            return None;
        }
        if !(rect).contains(point) {
            return None;
        }
        let cy = rect.origin.y + rect.size.y / 2.0;
        let first_cx = rect.origin.x + PAD + TRAFFIC_DOT / 2.0;
        for (i, ctl) in [
            WindowControl::Close,
            WindowControl::Minimize,
            WindowControl::Maximize,
        ]
        .into_iter()
        .enumerate()
        {
            let dot_cx = first_cx + i as f32 * TRAFFIC_STEP;
            // Square slop around the dot — adjacent zones tile
            // without overlap (±TRAFFIC_STEP/2 in x).
            if (point.x - dot_cx).abs() <= TRAFFIC_STEP / 2.0
                && (point.y - cy).abs() <= rect.size.y / 2.0
            {
                return Some(ctl);
            }
        }
        None
    }
}

/// Paint the Windows / Linux window-control dots at the left edge of
/// `bar_rect`, vertically centred on it, matching
/// [`TopBar::window_control_at`]'s hit zones for the same rect. Shared by
/// the canvas TopBar and Home's own header, so the borderless window keeps
/// its close / minimise / maximise controls on every screen (issue #268).
pub fn paint_window_control_dots(cx: &mut PaintCx<'_>, bar_rect: Rect, hover: bool) {
    let rect = bar_rect;
    let center_y = rect.origin.y + rect.size.y / 2.0;
    let traffic = [
        Color {
            r: 1.0,
            g: 0.373,
            b: 0.341,
            a: 1.0,
        }, // #FF5F57
        Color {
            r: 0.996,
            g: 0.737,
            b: 0.18,
            a: 1.0,
        }, // #FEBC2E
        Color {
            r: 0.157,
            g: 0.784,
            b: 0.251,
            a: 1.0,
        }, // #28C840
    ];
    let first_dot_cx = rect.origin.x + PAD + TRAFFIC_DOT / 2.0;
    for (i, color) in traffic.into_iter().enumerate() {
        let dot_cx = first_dot_cx + i as f32 * TRAFFIC_STEP;
        cx.backend.fill_oval(
            Rect {
                origin: Point2D::new(dot_cx - TRAFFIC_DOT / 2.0, center_y - TRAFFIC_DOT / 2.0),
                size: Point2D::new(TRAFFIC_DOT, TRAFFIC_DOT),
            },
            color,
        );
        // Hovering the cluster reveals each dot's glyph
        // (macOS-style): ✕ close, − minimise, + maximise.
        if hover {
            let glyph = Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.55,
            };
            let r = 3.0_f32;
            let lw = 1.3_f32;
            match i {
                0 => {
                    cx.backend.stroke_line(
                        Point2D::new(dot_cx - r, center_y - r),
                        Point2D::new(dot_cx + r, center_y + r),
                        glyph,
                        lw,
                    );
                    cx.backend.stroke_line(
                        Point2D::new(dot_cx - r, center_y + r),
                        Point2D::new(dot_cx + r, center_y - r),
                        glyph,
                        lw,
                    );
                }
                1 => {
                    cx.backend.stroke_line(
                        Point2D::new(dot_cx - r, center_y),
                        Point2D::new(dot_cx + r, center_y),
                        glyph,
                        lw,
                    );
                }
                _ => {
                    cx.backend.stroke_line(
                        Point2D::new(dot_cx - r, center_y),
                        Point2D::new(dot_cx + r, center_y),
                        glyph,
                        lw,
                    );
                    cx.backend.stroke_line(
                        Point2D::new(dot_cx, center_y - r),
                        Point2D::new(dot_cx, center_y + r),
                        glyph,
                        lw,
                    );
                }
            }
        }
    }
}
