//! One layout walk of the motion timeline panel, shared by paint and
//! hit-testing so they cannot drift apart.
//!
//! ```text
//! ┌ Home ───────┬ [On load] [In view] [Manual] ─── 0.42 s / 1.20 s ▶ ⟳ − + ┐
//! │             │ 0     0.5 s    1 s     1.5 s                             │ ruler
//! │ Header      │    ◆━━━━━━━━━◆                                           │
//! │ Card · 1    │          ◆━━━━━━━◆━━━━◆                                  │
//! └─────────────┴──────────────────▲────────────────────────────────────────┘
//!                                playhead
//! ```

use crate::widgets::motion_timeline::MotionTimelineView;
use crate::{Point2D, Rect};
use op_editor_core::editor_ui_state::{MotionTimelineTab, TimelineKeyframeRef};

pub const HEADER_H: f32 = 32.0;
pub const RULER_H: f32 = 20.0;
pub const ROW_H: f32 = 26.0;
pub const LABEL_W: f32 = 150.0;
/// Space between the lane's left edge and time zero.
pub const TRACK_PAD: f32 = 12.0;
pub const STRIP_H: f32 = 12.0;
pub const KEYFRAME_R: f32 = 5.0;
const TAB_W: f32 = 96.0;
const BUTTON_W: f32 = 28.0;
const TIME_W: f32 = 120.0;
/// Width of the grab zone just past a strip's end. It starts beyond the
/// last keyframe's diamond: dragging that keyframe retimes the end with
/// the other keyframes held, while the grip stretches them all.
const END_HANDLE_W: f32 = 8.0;
/// How far from a keyframe's centre a press still picks it.
const KEYFRAME_HIT: f32 = KEYFRAME_R + 1.0;

const TABS: [MotionTimelineTab; 3] = [
    MotionTimelineTab::Mount,
    MotionTimelineTab::InView,
    MotionTimelineTab::Manual,
];

/// What a point on the panel is.
#[derive(Debug, Clone, PartialEq)]
pub enum TimelineHit {
    Tab(MotionTimelineTab),
    PlayPause,
    Loop,
    ZoomOut,
    ZoomIn,
    /// The ruler: scrubs the playhead.
    Ruler,
    Keyframe(TimelineKeyframeRef),
    /// The grab zone at the end of a strip's first iteration.
    StripEnd {
        node_id: String,
        animation: usize,
    },
    Strip {
        node_id: String,
        animation: usize,
    },
    /// Anywhere else on the panel.
    Background,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowLayout {
    pub label: Rect,
    pub lane: Rect,
    /// The whole run (to the clip's end for endless loops).
    pub strip: Rect,
    /// The first iteration, the part a drag moves or stretches.
    pub iteration: Rect,
    pub end_handle: Rect,
    /// Keyframe centres of the first iteration.
    pub keyframes: Vec<Point2D>,
    pub loops: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineLayout {
    pub rect: Rect,
    pub header: Rect,
    pub scope: Rect,
    pub tabs: Vec<(MotionTimelineTab, Rect)>,
    pub time: Rect,
    pub play: Rect,
    pub looping: Rect,
    pub zoom_out: Rect,
    pub zoom_in: Rect,
    pub ruler: Rect,
    /// The label column under the header.
    pub labels: Rect,
    /// The lanes area under the ruler.
    pub lanes: Rect,
    pub rows: Vec<RowLayout>,
    zoom: f32,
}

impl TimelineLayout {
    pub fn new(rect: Rect, view: &MotionTimelineView) -> Self {
        let x = rect.origin.x;
        let y = rect.origin.y;
        let w = rect.size.x;
        let body_h = (rect.size.y - HEADER_H).max(0.0);
        let track_x = x + LABEL_W;
        let track_w = (w - LABEL_W).max(0.0);
        let header = Rect::xywh(x, y, w, HEADER_H);
        let button_y = y + (HEADER_H - 24.0) / 2.0;
        let right = x + w - 8.0;
        let zoom_in = Rect::xywh(right - BUTTON_W, button_y, BUTTON_W, 24.0);
        let zoom_out = Rect::xywh(zoom_in.origin.x - BUTTON_W, button_y, BUTTON_W, 24.0);
        let looping = Rect::xywh(zoom_out.origin.x - BUTTON_W - 8.0, button_y, BUTTON_W, 24.0);
        let play = Rect::xywh(looping.origin.x - BUTTON_W, button_y, BUTTON_W, 24.0);
        let time = Rect::xywh(play.origin.x - TIME_W, button_y, TIME_W, 24.0);
        let tabs = TABS
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                (
                    *tab,
                    Rect::xywh(
                        track_x + 8.0 + index as f32 * TAB_W,
                        button_y,
                        TAB_W - 4.0,
                        24.0,
                    ),
                )
            })
            .collect();
        let ruler = Rect::xywh(track_x, y + HEADER_H, track_w, RULER_H);
        let lanes = Rect::xywh(
            track_x,
            y + HEADER_H + RULER_H,
            track_w,
            (body_h - RULER_H).max(0.0),
        );
        let mut layout = Self {
            rect,
            header,
            scope: Rect::xywh(x, y, LABEL_W, HEADER_H),
            tabs,
            time,
            play,
            looping,
            zoom_out,
            zoom_in,
            ruler,
            labels: Rect::xywh(x, y + HEADER_H, LABEL_W, body_h),
            lanes,
            rows: Vec::new(),
            zoom: view.zoom,
        };
        layout.rows = view
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let row_y = lanes.origin.y + index as f32 * ROW_H;
                let lane = Rect::xywh(track_x, row_y, track_w, ROW_H);
                let center_y = row_y + ROW_H / 2.0;
                let strip = &row.strip;
                let start_x = layout.x_for_ms(strip.start_ms);
                let iteration_end_x = layout.x_for_ms(strip.start_ms + strip.iteration_ms);
                let run_end_x = layout.x_for_ms(
                    strip
                        .end_ms
                        .unwrap_or(view.clip_ms.max(strip.start_ms + strip.iteration_ms)),
                );
                let strip_y = center_y - STRIP_H / 2.0;
                RowLayout {
                    label: Rect::xywh(x, row_y, LABEL_W, ROW_H),
                    lane,
                    strip: Rect::xywh(start_x, strip_y, (run_end_x - start_x).max(1.0), STRIP_H),
                    iteration: Rect::xywh(
                        start_x,
                        strip_y,
                        (iteration_end_x - start_x).max(1.0),
                        STRIP_H,
                    ),
                    end_handle: Rect::xywh(
                        iteration_end_x + KEYFRAME_HIT,
                        strip_y,
                        END_HANDLE_W,
                        STRIP_H,
                    ),
                    keyframes: strip
                        .keyframe_ms
                        .iter()
                        .map(|time| Point2D::new(layout.x_for_ms(*time), center_y))
                        .collect(),
                    loops: strip.end_ms.is_none(),
                }
            })
            .collect();
        layout
    }

    /// Screen x of time `ms`.
    pub fn x_for_ms(&self, ms: f64) -> f32 {
        self.lanes.origin.x + TRACK_PAD + ms as f32 * self.zoom
    }

    /// Time under screen x, never before zero.
    pub fn ms_for_x(&self, x: f32) -> f64 {
        (f64::from(x - self.lanes.origin.x - TRACK_PAD) / f64::from(self.zoom.max(f32::EPSILON)))
            .max(0.0)
    }

    /// Pixels → milliseconds at the current zoom (for snap distances).
    pub fn ms_per_px(&self) -> f64 {
        1.0 / f64::from(self.zoom.max(f32::EPSILON))
    }

    pub fn hit(&self, point: Point2D, view: &MotionTimelineView) -> Option<TimelineHit> {
        if !self.rect.contains(point) {
            return None;
        }
        for (tab, rect) in &self.tabs {
            if rect.contains(point) {
                return Some(TimelineHit::Tab(*tab));
            }
        }
        for (rect, hit) in [
            (self.play, TimelineHit::PlayPause),
            (self.looping, TimelineHit::Loop),
            (self.zoom_out, TimelineHit::ZoomOut),
            (self.zoom_in, TimelineHit::ZoomIn),
        ] {
            if rect.contains(point) {
                return Some(hit);
            }
        }
        if self.ruler.contains(point) {
            return Some(TimelineHit::Ruler);
        }
        for (row, layout) in view.rows.iter().zip(&self.rows) {
            if !layout.lane.contains(point) {
                continue;
            }
            // Keyframes sit on the strip; they win, then the end handle.
            for (keyframe, center) in layout.keyframes.iter().enumerate() {
                if (center.x - point.x).abs() <= KEYFRAME_HIT
                    && (center.y - point.y).abs() <= KEYFRAME_HIT
                {
                    return Some(TimelineHit::Keyframe(TimelineKeyframeRef {
                        node_id: row.strip.node_id.clone(),
                        animation: row.strip.animation,
                        keyframe,
                    }));
                }
            }
            if layout.end_handle.contains(point) {
                return Some(TimelineHit::StripEnd {
                    node_id: row.strip.node_id.clone(),
                    animation: row.strip.animation,
                });
            }
            if layout.iteration.contains(point) {
                return Some(TimelineHit::Strip {
                    node_id: row.strip.node_id.clone(),
                    animation: row.strip.animation,
                });
            }
        }
        Some(TimelineHit::Background)
    }
}
