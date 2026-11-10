//! Shared state transitions of the motion timeline panel: which root it
//! projects, opening and closing, tabs, scrubbing and the play clock. Both
//! hosts call these; they only add hit-testing and paint.

use crate::editor_ui_state::{MotionTimelineState, MotionTimelineTab};
use crate::motion_timeline_projection::{manual_ids, project, TimelineProjection};
use crate::pen_node_ext::PenNodeExt;
use crate::walkers::find_node;
use crate::{EditorState, NodeId};
use jian_core::motion::ClipTrigger;
use jian_ops_schema::node::PenNode;

/// Frame interval the playing timeline asks the host for.
pub const MOTION_TIMELINE_FRAME_MS: u64 = 16;

impl EditorState {
    /// The root the timeline projects: the panel's explicit scope, else the
    /// top-level node holding the selection, else the first top-level node
    /// on the page that animates.
    pub fn motion_timeline_scope(&self) -> Option<&PenNode> {
        let children = self.active_children();
        if let Some(root) = self.editor_ui.motion_timeline.scope_root.as_deref() {
            if let Some(node) = find_node(children, &NodeId::new(root)) {
                return Some(node);
            }
        }
        let anchor = &self.selection.anchor;
        if anchor.is_real() {
            if let Some(root) = children.iter().find(|root| {
                root.id_str() == anchor.as_str()
                    || find_node(std::slice::from_ref(*root), anchor).is_some()
            }) {
                return Some(root);
            }
        }
        children.iter().find(|root| animates(root))
    }

    /// The current tab's projection of the scope, empty when there is
    /// nothing to project (no scope, or the manual tab without an id).
    pub fn motion_timeline_projection(&self) -> TimelineProjection {
        let timeline = &self.editor_ui.motion_timeline;
        match (self.motion_timeline_scope(), timeline.clip_trigger()) {
            (Some(root), Some(trigger)) => project(root, &trigger),
            _ => TimelineProjection::default(),
        }
    }

    /// Open or close the panel. Opening on a tab with nothing to show
    /// moves to the first tab that has motion (on load, in view, then the
    /// first manual id), so the panel never opens empty beside motion.
    pub fn toggle_motion_timeline(&mut self) {
        let opening = !self.editor_ui.motion_timeline.open;
        self.editor_ui.motion_timeline.open = opening;
        if !opening {
            let timeline = &mut self.editor_ui.motion_timeline;
            stop(timeline);
            timeline.drag = None;
            return;
        }
        if self.motion_timeline_projection().strips.is_empty() {
            if let Some((tab, manual_id)) = self.first_motion_timeline_tab() {
                self.editor_ui.motion_timeline.show(tab, manual_id);
            }
        }
    }

    fn first_motion_timeline_tab(&self) -> Option<(MotionTimelineTab, Option<String>)> {
        let root = self.motion_timeline_scope()?;
        [
            (MotionTimelineTab::Mount, ClipTrigger::Mount),
            (MotionTimelineTab::InView, ClipTrigger::InView),
        ]
        .into_iter()
        .find(|(_, trigger)| !project(root, trigger).strips.is_empty())
        .map(|(tab, _)| (tab, None))
        .or_else(|| {
            manual_ids(root)
                .into_iter()
                .next()
                .map(|id| (MotionTimelineTab::Manual, Some(id)))
        })
    }

    pub fn set_motion_timeline_tab(&mut self, tab: MotionTimelineTab, manual_id: Option<String>) {
        self.editor_ui.motion_timeline.show(tab, manual_id);
    }

    /// Move the playhead to `ms`, kept inside the clip. While playing,
    /// playback continues from there.
    pub fn scrub_motion_playhead(&mut self, ms: f64, now_ms: u64) {
        let clip_ms = self.motion_timeline_projection().clip_ms;
        let timeline = &mut self.editor_ui.motion_timeline;
        timeline.playhead_ms = ms.clamp(0.0, clip_ms.max(0.0));
        if timeline.playing {
            timeline.play_clock = Some((now_ms, timeline.playhead_ms));
        }
    }

    /// Play from the playhead (from the start once it reached the end), or
    /// pause where it is.
    pub fn toggle_motion_playback(&mut self, now_ms: u64) {
        let clip_ms = self.motion_timeline_projection().clip_ms;
        let timeline = &mut self.editor_ui.motion_timeline;
        if timeline.playing {
            stop(timeline);
            return;
        }
        if clip_ms <= 0.0 {
            return;
        }
        if timeline.playhead_ms >= clip_ms {
            timeline.playhead_ms = 0.0;
        }
        timeline.playing = true;
        timeline.play_clock = Some((now_ms, timeline.playhead_ms));
    }

    pub fn toggle_motion_loop(&mut self) {
        let timeline = &mut self.editor_ui.motion_timeline;
        timeline.looping = !timeline.looping;
    }

    /// Advance a playing timeline to `now_ms`: wrap at the end when
    /// looping, otherwise stop there. Returns whether the playhead moved.
    pub fn tick_motion_timeline(&mut self, now_ms: u64) -> bool {
        let clip_ms = self.motion_timeline_projection().clip_ms;
        let timeline = &mut self.editor_ui.motion_timeline;
        let Some((started_at, from_ms)) = timeline.play_clock.filter(|_| timeline.playing) else {
            return false;
        };
        let before = timeline.playhead_ms;
        let mut playhead = from_ms + now_ms.saturating_sub(started_at) as f64;
        if clip_ms <= 0.0 {
            stop(timeline);
            playhead = 0.0;
        } else if playhead >= clip_ms {
            if timeline.looping {
                playhead %= clip_ms;
            } else {
                playhead = clip_ms;
                stop(timeline);
            }
        }
        timeline.playhead_ms = playhead;
        playhead != before
    }

    /// When a playing timeline next needs a frame.
    pub fn motion_timeline_deadline(&self, now_ms: u64) -> Option<u64> {
        let timeline = &self.editor_ui.motion_timeline;
        (timeline.open && timeline.playing).then(|| now_ms + MOTION_TIMELINE_FRAME_MS)
    }
}

fn stop(timeline: &mut MotionTimelineState) {
    timeline.playing = false;
    timeline.play_clock = None;
}

fn animates(node: &PenNode) -> bool {
    node.motion_declarations()
        .1
        .is_some_and(|animations| !animations.is_empty())
        || node.children().into_iter().flatten().any(animates)
}

#[cfg(test)]
#[path = "motion_timeline_transitions_tests.rs"]
mod tests;
