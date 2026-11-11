//! One layout walk of the motion timeline panel, shared by paint and
//! hit-testing so they cannot drift apart.
//!
//! ```text
//! ╞═════════════╪═══════════════════ resize band ═══════════════════════════╡
//! ┌ Home ▾ ─────┬ [On load] [In view] [Manual] ─── 0.42 s / 1.20 s ▶ ⟳ − + ┐
//! │             │ 0     0.5 s    1 s     1.5 s                             │ ruler
//! │   Header    │    ◆━━━━━━━━━◆                                           │
//! │ ▾ Card · 1  │          ◆━━━━━━━◆━━━━◆                                  │
//! │     opacity │          ◆               ◆                               │ sub-track
//! └─────────────┴──────────────────▲────────────────────────────────────────┘
//!                                playhead
//! ```
//!
//! The screen dropdown opens upward from the scope button, over the
//! canvas, so it is walked here too and wins every hit while open.

use crate::widgets::motion_timeline::{MotionTimelineView, TimelineLane, TimelineRow};
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
/// Half the height of the resize band on the panel's top edge.
pub const RESIZE_BAND: f32 = 3.0;
/// Label indent per tree level.
pub const DEPTH_INDENT: f32 = 12.0;
/// The expand chevron in front of a lane's label.
const CHEVRON_W: f32 = 14.0;
pub const MENU_ROW_H: f32 = 26.0;
const MENU_W: f32 = 220.0;

const TABS: [MotionTimelineTab; 3] = [
    MotionTimelineTab::Mount,
    MotionTimelineTab::InView,
    MotionTimelineTab::Manual,
];

/// What a point on the panel is.
#[derive(Debug, Clone, PartialEq)]
pub enum TimelineHit {
    /// The band along the top edge: drags the panel's height.
    ResizeEdge,
    /// The screen name in the header: opens the screen dropdown.
    Scope,
    /// A screen in the open dropdown.
    ScopeItem(String),
    /// The chevron of a lane with property sub-tracks.
    Expand {
        node_id: String,
        animation: usize,
    },
    /// A row's label: selects its node.
    Label(String),
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
    /// Where the label text starts, after the indent and chevron.
    pub label_x: f32,
    /// The expand chevron, for lanes with sub-tracks.
    pub chevron: Option<Rect>,
    pub lane: Rect,
    /// The whole run (to the clip's end for endless loops).
    pub strip: Rect,
    /// The first iteration, the part a drag moves or stretches.
    pub iteration: Rect,
    pub end_handle: Rect,
    /// Keyframe centres of the first iteration, with each one's index in
    /// the animation (a sub-track shows only the keyframes holding its
    /// property).
    pub keyframes: Vec<(usize, Point2D)>,
    pub loops: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineLayout {
    pub rect: Rect,
    pub resize_edge: Rect,
    pub header: Rect,
    pub scope: Rect,
    /// The open screen dropdown, rising from the scope button: its frame
    /// and one rect per screen.
    pub scope_menu: Option<(Rect, Vec<(String, Rect)>)>,
    pub tabs: Vec<(MotionTimelineTab, Rect)>,
    /// The free header span between the tabs and the time readout.
    pub hint: Rect,
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
        let tabs_end = track_x + 8.0 + TABS.len() as f32 * TAB_W;
        let hint = Rect::xywh(
            tabs_end + 8.0,
            button_y,
            (time.origin.x - tabs_end - 16.0).max(0.0),
            24.0,
        );
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
        let scope = Rect::xywh(x, y, LABEL_W, HEADER_H);
        let scope_menu = view.scope_menu.as_ref().map(|screens| {
            let menu_h = screens.len() as f32 * MENU_ROW_H + 8.0;
            let frame = Rect::xywh(x + 4.0, y - 4.0 - menu_h, MENU_W, menu_h);
            let rows = screens
                .iter()
                .enumerate()
                .map(|(index, screen)| {
                    let row_y = frame.origin.y + 4.0 + index as f32 * MENU_ROW_H;
                    (
                        screen.id.clone(),
                        Rect::xywh(frame.origin.x + 4.0, row_y, MENU_W - 8.0, MENU_ROW_H),
                    )
                })
                .collect();
            (frame, rows)
        });
        let mut layout = Self {
            rect,
            resize_edge: Rect::xywh(x, y - RESIZE_BAND, w, RESIZE_BAND * 2.0),
            header,
            scope,
            scope_menu,
            tabs,
            hint,
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
            .map(|(index, row)| layout.row(index, row, view.clip_ms))
            .collect();
        layout
    }

    fn row(&self, index: usize, row: &TimelineRow, clip_ms: f64) -> RowLayout {
        let x = self.rect.origin.x;
        let row_y = self.lanes.origin.y + index as f32 * ROW_H;
        let lane = Rect::xywh(self.lanes.origin.x, row_y, self.lanes.size.x, ROW_H);
        let center_y = row_y + ROW_H / 2.0;
        let indent_x = x + 8.0 + row.depth as f32 * DEPTH_INDENT;
        let chevron = matches!(
            row.lane,
            TimelineLane::Strip {
                expandable: true,
                ..
            }
        )
        .then(|| Rect::xywh(indent_x, row_y, CHEVRON_W, ROW_H));
        let mut out = RowLayout {
            label: Rect::xywh(x, row_y, LABEL_W, ROW_H),
            label_x: indent_x + CHEVRON_W + 2.0,
            chevron,
            lane,
            strip: Rect::xywh(0.0, 0.0, 0.0, 0.0),
            iteration: Rect::xywh(0.0, 0.0, 0.0, 0.0),
            end_handle: Rect::xywh(0.0, 0.0, 0.0, 0.0),
            keyframes: Vec::new(),
            loops: false,
        };
        let Some(strip) = row.strip() else {
            return out;
        };
        let start_x = self.x_for_ms(strip.start_ms);
        let iteration_end_x = self.x_for_ms(strip.start_ms + strip.iteration_ms);
        let run_end_x = self.x_for_ms(
            strip
                .end_ms
                .unwrap_or(clip_ms.max(strip.start_ms + strip.iteration_ms)),
        );
        let strip_y = center_y - STRIP_H / 2.0;
        out.strip = Rect::xywh(start_x, strip_y, (run_end_x - start_x).max(1.0), STRIP_H);
        out.iteration = Rect::xywh(
            start_x,
            strip_y,
            (iteration_end_x - start_x).max(1.0),
            STRIP_H,
        );
        out.loops = strip.end_ms.is_none();
        let at = |keyframe: usize| {
            strip
                .keyframe_ms
                .get(keyframe)
                .map(|time| (keyframe, Point2D::new(self.x_for_ms(*time), center_y)))
        };
        match &row.lane {
            TimelineLane::Strip { .. } => {
                out.end_handle = Rect::xywh(
                    iteration_end_x + KEYFRAME_HIT,
                    strip_y,
                    END_HANDLE_W,
                    STRIP_H,
                );
                out.keyframes = (0..strip.keyframe_ms.len()).filter_map(at).collect();
            }
            TimelineLane::Property { keyframes, .. } => {
                out.keyframes = keyframes.iter().copied().filter_map(at).collect();
            }
            TimelineLane::Empty { .. } => {}
        }
        out
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
        if let Some((frame, rows)) = &self.scope_menu {
            if let Some((id, _)) = rows.iter().find(|(_, rect)| rect.contains(point)) {
                return Some(TimelineHit::ScopeItem(id.clone()));
            }
            if frame.contains(point) {
                return Some(TimelineHit::Background);
            }
        }
        if self.resize_edge.contains(point) {
            return Some(TimelineHit::ResizeEdge);
        }
        if !self.rect.contains(point) {
            return None;
        }
        if self.scope.contains(point) {
            return Some(TimelineHit::Scope);
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
            if layout
                .chevron
                .is_some_and(|chevron| chevron.contains(point))
            {
                if let Some(strip) = row.strip() {
                    return Some(TimelineHit::Expand {
                        node_id: strip.node_id.clone(),
                        animation: strip.animation,
                    });
                }
            }
            if layout.label.contains(point) {
                return Some(TimelineHit::Label(row.node_id().to_owned()));
            }
            if !layout.lane.contains(point) {
                continue;
            }
            let Some(strip) = row.strip() else {
                break;
            };
            // Keyframes sit on the strip; they win, then the end handle.
            for (keyframe, center) in &layout.keyframes {
                if (center.x - point.x).abs() <= KEYFRAME_HIT
                    && (center.y - point.y).abs() <= KEYFRAME_HIT
                {
                    return Some(TimelineHit::Keyframe(TimelineKeyframeRef {
                        node_id: strip.node_id.clone(),
                        animation: strip.animation,
                        keyframe: *keyframe,
                    }));
                }
            }
            // A sub-track only picks keyframes.
            if !matches!(row.lane, TimelineLane::Strip { .. }) {
                break;
            }
            if layout.end_handle.contains(point) {
                return Some(TimelineHit::StripEnd {
                    node_id: strip.node_id.clone(),
                    animation: strip.animation,
                });
            }
            if layout.iteration.contains(point) {
                return Some(TimelineHit::Strip {
                    node_id: strip.node_id.clone(),
                    animation: strip.animation,
                });
            }
        }
        Some(TimelineHit::Background)
    }
}
