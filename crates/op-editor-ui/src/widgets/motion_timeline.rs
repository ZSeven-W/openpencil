//! Motion timeline panel: one screen's animations for one trigger laid out
//! on absolute time, docked below the canvas.
//!
//! [`MotionTimelineView`] is what the panel shows, read from the editor
//! state each frame: the projection (with an in-flight drag applied as a
//! preview, so the strip follows the pointer before anything is written),
//! the playhead and the header controls. `motion_timeline_layout` walks it
//! once for both paint and hit-testing; `motion_timeline_paint` draws it.
//! Every state change goes through `op_editor_core`'s shared timeline
//! transitions, so the hosts only add input plumbing.

use crate::widgets::motion_timeline_rows;
use crate::widgets::property_panel_interactions::tr;
use op_editor_core::editor_ui_state::{MotionTimelineDrag, MotionTimelineTab, TimelineKeyframeRef};
use op_editor_core::motion_timeline_projection::{manual_ids, TimelineStrip};
use op_editor_core::motion_timeline_scope::{motion_node_label, MotionScreen};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{EditorState, Locale};

pub use crate::widgets::motion_timeline_layout::{TimelineHit, TimelineLayout};
pub use crate::widgets::motion_timeline_paint::paint_motion_timeline;

/// Paint the panel when it is shown: each host's whole paint arm, in the
/// editor's theme (as the StatusBar reads it).
pub fn paint(
    backend: &mut dyn crate::RenderBackend,
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) {
    if let Some(rect) =
        crate::widgets::host_canvas_geometry::motion_timeline_rect(state, viewport_w, viewport_h)
    {
        let theme = crate::widgets::editor_state_ext::theme_for(&state.editor_ui);
        let view = MotionTimelineView::for_editor(state);
        paint_motion_timeline(
            &mut crate::widgets::PaintCx { backend },
            &theme,
            rect,
            &view,
        );
    }
}

/// What a lane shows.
#[derive(Debug, Clone, PartialEq)]
pub enum TimelineLane {
    /// One animation, drag-previewed; `expanded` shows its property
    /// sub-tracks below it.
    Strip {
        strip: TimelineStrip,
        expanded: bool,
        /// It animates at least one property, so it can expand.
        expandable: bool,
    },
    /// One property of an expanded animation: diamonds only at the
    /// keyframes that hold it.
    Property {
        strip: TimelineStrip,
        keyframes: Vec<usize>,
    },
    /// The selected node, which has no motion on this tab yet.
    Empty { node_id: String },
}

/// One row: its label, its depth in the scope's tree, and its lane.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineRow {
    pub label: String,
    /// Tree depth below the scope root, for the label indent.
    pub depth: usize,
    pub lane: TimelineLane,
}

impl TimelineRow {
    /// The animation the row belongs to, if any.
    pub fn strip(&self) -> Option<&TimelineStrip> {
        match &self.lane {
            TimelineLane::Strip { strip, .. } | TimelineLane::Property { strip, .. } => Some(strip),
            TimelineLane::Empty { .. } => None,
        }
    }

    /// The node the row belongs to.
    pub fn node_id(&self) -> &str {
        match &self.lane {
            TimelineLane::Strip { strip, .. } | TimelineLane::Property { strip, .. } => {
                &strip.node_id
            }
            TimelineLane::Empty { node_id } => node_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionTimelineView {
    pub locale: Locale,
    /// Name of the projected root.
    pub scope_label: Option<String>,
    pub scope_id: Option<String>,
    /// The screens the header dropdown lists, when it is open.
    pub scope_menu: Option<Vec<MotionScreen>>,
    /// Every strip of the tab, drag-previewed, in document order.
    pub strips: Vec<TimelineStrip>,
    pub rows: Vec<TimelineRow>,
    /// The clip's length, stretched to cover a strip being dragged past it.
    pub clip_ms: f64,
    pub playhead_ms: f64,
    pub zoom: f32,
    pub tab: MotionTimelineTab,
    pub manual_id: Option<String>,
    /// Manual ids under the scope, for the manual tab.
    pub manual_ids: Vec<String>,
    pub playing: bool,
    pub looping: bool,
    pub selection: Vec<TimelineKeyframeRef>,
}

impl MotionTimelineView {
    pub fn for_editor(state: &EditorState) -> Self {
        let timeline = &state.editor_ui.motion_timeline;
        let scope = state.motion_timeline_scope();
        let projection = state.motion_timeline_projection();
        let mut strips = projection.strips;
        if let Some(drag) = &timeline.drag {
            preview_drag(&mut strips, drag);
        }
        let clip_ms = strips
            .iter()
            .filter_map(|strip| strip.end_ms)
            .fold(projection.clip_ms, f64::max);
        let rows = scope
            .map(|root| motion_timeline_rows::rows(state, root, &strips))
            .unwrap_or_default();
        Self {
            locale: state.editor_ui.locale,
            scope_label: scope.map(motion_node_label),
            scope_id: scope.map(|root| root.id_str().to_owned()),
            scope_menu: timeline
                .scope_menu_open
                .then(|| state.motion_timeline_screens()),
            strips,
            rows,
            clip_ms,
            playhead_ms: timeline.playhead_ms,
            zoom: timeline.zoom,
            tab: timeline.tab,
            manual_id: timeline.manual_id.clone(),
            manual_ids: scope.map(manual_ids).unwrap_or_default(),
            playing: timeline.playing,
            looping: timeline.looping,
            selection: timeline.selection.clone(),
        }
    }

    pub fn is_selected(&self, key: &TimelineKeyframeRef) -> bool {
        self.selection.contains(key)
    }

    /// Label of a header tab; the manual tab names its clip.
    pub fn tab_label(&self, tab: MotionTimelineTab) -> String {
        match tab {
            MotionTimelineTab::Mount => {
                tr(self.locale, ("motion.trigger.mount", "On load")).to_owned()
            }
            MotionTimelineTab::InView => {
                tr(self.locale, ("motion.trigger.inView", "In view")).to_owned()
            }
            MotionTimelineTab::Manual => {
                let manual = tr(self.locale, ("motion.trigger.manual", "Manual"));
                match (self.tab, self.manual_id.as_deref()) {
                    (MotionTimelineTab::Manual, Some(id)) => format!("{manual}: {id}"),
                    _ => manual.to_owned(),
                }
            }
        }
    }

    /// Shown while the playhead is past zero: the canvas shows the frame
    /// under it, but canvas edits still change the static layout.
    pub fn static_hint(&self) -> Option<&'static str> {
        (self.playhead_ms > 0.0 && !self.strips.is_empty()).then(|| {
            tr(
                self.locale,
                ("motion.timeline.staticHint", "Editing the static layout"),
            )
        })
    }

    /// Why the lanes are empty, if they are.
    pub fn empty_message(&self) -> Option<&'static str> {
        if !self.strips.is_empty() {
            return None;
        }
        Some(
            if self.tab == MotionTimelineTab::Manual && self.manual_id.is_none() {
                tr(
                    self.locale,
                    ("motion.timeline.pickId", "Choose a motion id"),
                )
            } else {
                tr(
                    self.locale,
                    ("motion.timeline.empty", "No motion on this screen"),
                )
            },
        )
    }
}

/// Apply an in-flight drag to the strips it touches, so paint shows where
/// the release would put them.
fn preview_drag(strips: &mut [TimelineStrip], drag: &MotionTimelineDrag) {
    match drag {
        MotionTimelineDrag::Strip {
            node_id,
            animation,
            delta_ms,
        } => {
            if let Some(strip) = find(strips, node_id, *animation) {
                let delta = delta_ms.max(-strip.start_ms);
                strip.start_ms += delta;
                strip.end_ms = strip.end_ms.map(|end| end + delta);
                for time in &mut strip.keyframe_ms {
                    *time += delta;
                }
            }
        }
        MotionTimelineDrag::StripEnd {
            node_id,
            animation,
            end_ms,
        } => {
            if let Some(strip) = find(strips, node_id, *animation) {
                let new_iteration = (end_ms - strip.start_ms).max(1.0);
                let scale = new_iteration / strip.iteration_ms.max(1.0);
                for time in &mut strip.keyframe_ms {
                    *time = strip.start_ms + (*time - strip.start_ms) * scale;
                }
                strip.end_ms = strip
                    .end_ms
                    .map(|end| strip.start_ms + (end - strip.start_ms) * scale);
                strip.iteration_ms = new_iteration;
            }
        }
        MotionTimelineDrag::Keyframes { keys, delta_ms } => {
            for key in keys {
                if let Some(strip) = find(strips, &key.node_id, key.animation) {
                    if let Some(time) = strip.keyframe_ms.get_mut(key.keyframe) {
                        *time = (*time + delta_ms).max(0.0);
                    }
                }
            }
        }
        MotionTimelineDrag::Playhead | MotionTimelineDrag::Height { .. } => {}
    }
}

fn find<'a>(
    strips: &'a mut [TimelineStrip],
    node_id: &str,
    animation: usize,
) -> Option<&'a mut TimelineStrip> {
    strips
        .iter_mut()
        .find(|strip| strip.node_id == node_id && strip.animation == animation)
}

#[cfg(test)]
#[path = "motion_timeline_tests.rs"]
mod tests;
