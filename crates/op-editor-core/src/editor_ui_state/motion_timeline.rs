//! UI state of the motion timeline panel: whether it is open, which screen
//! and trigger it projects, the playhead, the keyframe selection and the
//! gesture in flight.
//!
//! None of it is saved. A drag edits [`MotionTimelineState::drag`] only;
//! releasing it submits one motion command, so every gesture is one undo
//! step and an abandoned drag leaves the document untouched.

use jian_core::motion::ClipTrigger;

/// Which trigger's animations the timeline lays out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionTimelineTab {
    #[default]
    Mount,
    InView,
    /// Manual animations sharing [`MotionTimelineState::manual_id`].
    Manual,
}

/// One keyframe on the timeline: node, animation index, keyframe index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineKeyframeRef {
    pub node_id: String,
    pub animation: usize,
    pub keyframe: usize,
}

/// The gesture in flight, as a draft the panel paints until release.
#[derive(Debug, Clone, PartialEq)]
pub enum MotionTimelineDrag {
    /// Moving a whole strip by `delta_ms`.
    Strip {
        node_id: String,
        animation: usize,
        delta_ms: f64,
    },
    /// Dragging a strip's first-iteration end to `end_ms`.
    StripEnd {
        node_id: String,
        animation: usize,
        end_ms: f64,
    },
    /// Moving the selected keyframes by `delta_ms`.
    Keyframes {
        keys: Vec<TimelineKeyframeRef>,
        delta_ms: f64,
    },
    /// Scrubbing the playhead.
    Playhead,
    /// Resizing the panel from its top edge: the pointer's y and the
    /// panel height when the drag started.
    Height { pointer_y: f32, height: f32 },
}

/// Where a drag started: the pointer's time and the time of what it
/// holds (a strip's start, a keyframe, an end). The held time follows the
/// pointer's travel, so grabbing a strip off-centre does not jump it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineDragAnchor {
    pub pointer_ms: f64,
    pub held_ms: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionTimelineState {
    pub open: bool,
    /// Panel height in logical pixels.
    pub height: f32,
    /// Node whose subtree the timeline projects; `None` follows the active
    /// screen.
    pub scope_root: Option<String>,
    /// The selection anchor when `scope_root` was picked: selecting a node
    /// in another screen afterwards follows the selection again.
    pub scope_anchor: Option<String>,
    /// The header's screen dropdown is open.
    pub scope_menu_open: bool,
    /// Lanes (node id, animation index) showing their property sub-tracks.
    pub expanded: Vec<(String, usize)>,
    pub tab: MotionTimelineTab,
    /// The manual animation id the manual tab shows.
    pub manual_id: Option<String>,
    pub playhead_ms: f64,
    pub playing: bool,
    /// While playing: the host time and playhead playback started from, so
    /// the playhead follows the clock instead of accumulating frame steps.
    pub play_clock: Option<(u64, f64)>,
    pub looping: bool,
    /// Horizontal scale in pixels per millisecond.
    pub zoom: f32,
    pub selection: Vec<TimelineKeyframeRef>,
    pub drag: Option<MotionTimelineDrag>,
    pub drag_anchor: Option<TimelineDragAnchor>,
}

impl MotionTimelineState {
    pub const DEFAULT_HEIGHT: f32 = 220.0;
    /// The design's drag range (§5.3: 160–360 px).
    pub const MIN_HEIGHT: f32 = 160.0;
    pub const MAX_HEIGHT: f32 = 360.0;
    /// One second spans 250 px by default.
    pub const DEFAULT_ZOOM: f32 = 0.25;
    pub const MIN_ZOOM: f32 = 0.02;
    pub const MAX_ZOOM: f32 = 2.0;

    /// The trigger the current tab projects; `None` on the manual tab
    /// before an id is chosen.
    pub fn clip_trigger(&self) -> Option<ClipTrigger> {
        match self.tab {
            MotionTimelineTab::Mount => Some(ClipTrigger::Mount),
            MotionTimelineTab::InView => Some(ClipTrigger::InView),
            MotionTimelineTab::Manual => self.manual_id.clone().map(ClipTrigger::Manual),
        }
    }

    pub fn set_height(&mut self, height: f32) {
        self.height = height.clamp(Self::MIN_HEIGHT, Self::MAX_HEIGHT);
    }

    /// Show or hide the property sub-tracks of one lane.
    pub fn toggle_expanded(&mut self, node_id: &str, animation: usize) {
        let lane = (node_id.to_owned(), animation);
        match self.expanded.iter().position(|open| *open == lane) {
            Some(at) => {
                self.expanded.remove(at);
            }
            None => self.expanded.push(lane),
        }
    }

    pub fn is_expanded(&self, node_id: &str, animation: usize) -> bool {
        self.expanded
            .iter()
            .any(|(id, index)| id == node_id && *index == animation)
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// Switching what the timeline shows drops the selection and any drag,
    /// which refer to the previous projection, and rewinds the playhead.
    pub fn show(&mut self, tab: MotionTimelineTab, manual_id: Option<String>) {
        self.tab = tab;
        self.manual_id = manual_id;
        self.selection.clear();
        self.drag = None;
        self.playhead_ms = 0.0;
        self.playing = false;
        self.play_clock = None;
    }
}

impl Default for MotionTimelineState {
    fn default() -> Self {
        Self {
            open: false,
            height: Self::DEFAULT_HEIGHT,
            scope_root: None,
            scope_anchor: None,
            scope_menu_open: false,
            expanded: Vec::new(),
            tab: MotionTimelineTab::default(),
            manual_id: None,
            playhead_ms: 0.0,
            playing: false,
            play_clock: None,
            looping: false,
            zoom: Self::DEFAULT_ZOOM,
            selection: Vec::new(),
            drag: None,
            drag_anchor: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_map_to_the_trigger_they_project() {
        let mut state = MotionTimelineState::default();
        assert_eq!(state.clip_trigger(), Some(ClipTrigger::Mount));
        state.show(MotionTimelineTab::Manual, None);
        assert_eq!(state.clip_trigger(), None);
        state.show(MotionTimelineTab::Manual, Some("intro".into()));
        assert_eq!(
            state.clip_trigger(),
            Some(ClipTrigger::Manual("intro".into()))
        );
    }

    #[test]
    fn switching_tabs_drops_the_old_selection_and_drag() {
        let mut state = MotionTimelineState {
            playhead_ms: 300.0,
            playing: true,
            selection: vec![TimelineKeyframeRef {
                node_id: "a".into(),
                animation: 0,
                keyframe: 1,
            }],
            drag: Some(MotionTimelineDrag::Playhead),
            ..MotionTimelineState::default()
        };
        state.show(MotionTimelineTab::InView, None);
        assert!(state.selection.is_empty() && state.drag.is_none());
        assert_eq!((state.playhead_ms, state.playing), (0.0, false));
    }

    #[test]
    fn lanes_expand_and_collapse() {
        let mut state = MotionTimelineState::default();
        state.toggle_expanded("a", 1);
        assert!(state.is_expanded("a", 1) && !state.is_expanded("a", 0));
        state.toggle_expanded("a", 1);
        assert!(!state.is_expanded("a", 1));
    }

    #[test]
    fn height_and_zoom_stay_in_range() {
        let mut state = MotionTimelineState::default();
        state.set_height(10.0);
        state.set_zoom(99.0);
        assert_eq!(state.height, MotionTimelineState::MIN_HEIGHT);
        assert_eq!(state.zoom, MotionTimelineState::MAX_ZOOM);
        state.set_height(900.0);
        assert_eq!(state.height, 360.0);
        assert!((160.0..=360.0).contains(&MotionTimelineState::DEFAULT_HEIGHT));
    }
}
