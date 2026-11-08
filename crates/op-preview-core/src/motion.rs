//! P1 document motion in Preview: the session glue around jian-core's
//! motion triggers (mount, in-view, transitions, the switch tween), which
//! run against the Preview scene and timeline.

use jian_ops_schema::motion::MotionPreference;
use op_editor_ui::layout_scene::LayoutScene;

impl crate::session::PreviewSession {
    pub fn set_motion_preference(&mut self, host: MotionPreference) {
        self.host_motion_preference = host;
        let lifecycle_started = self.motion.lifecycle_started();
        self.animation.clear();
        self.motion.reset_screen();
        self.motion
            .set_initial_lifecycle_values(&self.runtime, &self.scene, &self.animation);
        if lifecycle_started {
            self.begin_lifecycle(self.last_now_ms);
        }
    }

    pub fn effective_motion_preference(&self) -> MotionPreference {
        let document = self
            .reset_seed
            .document
            .motion
            .unwrap_or(MotionPreference::Full);
        document.more_reduced(self.host_motion_preference)
    }

    pub(crate) fn start_mount_animations(&self, now_ms: u64) {
        self.motion.start_mount_animations(
            &self.runtime,
            &self.scene,
            &self.animation,
            self.effective_motion_preference(),
            now_ms,
        );
    }

    /// Admit lifecycle animations only after the host has put the preview on
    /// its steady surface. Before this edge, lifecycle declarations paint at
    /// their first keyframe and in-view observation is disabled.
    pub fn begin_lifecycle(&mut self, now_ms: u64) {
        if !self.motion.begin_lifecycle() {
            return;
        }
        let now_ms = self.last_now_ms.max(now_ms);
        self.last_now_ms = now_ms;
        self.motion
            .set_initial_lifecycle_values(&self.runtime, &self.scene, &self.animation);
        self.start_mount_animations(now_ms);
        let scene = self.overlay_runtime_state_without_animation(&self.scene);
        self.motion.observe_toggle_progress(
            &self.runtime,
            &scene,
            &self.animation,
            self.effective_motion_preference(),
            now_ms,
        );
    }

    pub(crate) fn observe_motion(&self, scene: &LayoutScene) {
        if !self.motion.lifecycle_started() {
            return;
        }
        let effective = self.effective_motion_preference();
        self.motion.observe_transitions(
            &self.runtime,
            scene,
            &self.animation,
            effective,
            self.last_now_ms,
        );
        self.motion.observe_toggle_progress(
            &self.runtime,
            scene,
            &self.animation,
            effective,
            self.last_now_ms,
        );
        self.motion.observe_in_view(
            &self.runtime,
            scene,
            &self.animation,
            effective,
            self.last_now_ms,
        );
    }

    /// Re-read switch `checked` after runtime input so the intrinsic
    /// toggle tween starts at the event timestamp, not the next paint.
    pub(crate) fn sync_toggle_progress(&self) {
        if !self.has_switch_widgets || !self.motion.lifecycle_started() {
            return;
        }
        let scene = self.overlay_runtime_state_without_animation(&self.scene);
        self.motion.observe_toggle_progress(
            &self.runtime,
            &scene,
            &self.animation,
            self.effective_motion_preference(),
            self.last_now_ms,
        );
    }
}
