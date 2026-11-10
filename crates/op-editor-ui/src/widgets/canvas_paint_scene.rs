//! The scene the canvas paints this frame when it differs from the layout
//! scene: mid layout transition, and at the motion timeline's playhead.
//!
//! While the timeline shows a clip, the canvas paints the projected screen
//! as it looks under the playhead: the clip is sampled with the same track
//! sampler Preview runs and applied through `jian-scene`'s motion adapter.
//! Only the scope root's subtree animates; the document is never written,
//! so closing the panel restores the plain paint and exports stay static.
//! Both hosts call [`canvas_paint_scene`] instead of building it inline.

use crate::layout_scene::LayoutScene;
use crate::widgets::canvas_layout_transition::CanvasLayoutTransition;
use crate::widgets::host_canvas_geometry::motion_timeline_shown;
use jian_core::motion::{Clip, MotionOverrides};
use jian_scene::layout_scene_motion::apply_motion_overrides;
use op_editor_core::EditorState;

/// `None` when the canvas paints `base` as it is.
pub fn canvas_paint_scene(
    state: &EditorState,
    base: &LayoutScene,
    transition: Option<&CanvasLayoutTransition>,
    now_ms: u64,
) -> Option<LayoutScene> {
    let mut scene = transition
        .filter(|transition| transition.is_active(now_ms))
        .map(|transition| {
            let mut scene = base.clone();
            transition.apply_to_scene(&mut scene, now_ms);
            scene
        });
    let overrides = motion_playhead_overrides(state, scene.as_ref().unwrap_or(base));
    if !overrides.is_empty() {
        apply_motion_overrides(scene.get_or_insert_with(|| base.clone()), &overrides);
    }
    scene
}

/// The timeline's clip sampled at the playhead against `view`: empty when
/// the panel is hidden, has no clip, or the clip leaves every value at
/// its base.
pub fn motion_playhead_overrides(state: &EditorState, view: &LayoutScene) -> MotionOverrides {
    if !motion_timeline_shown(state) {
        return MotionOverrides::new();
    }
    let (Some(root), Some(trigger)) = (
        state.motion_timeline_scope(),
        state.editor_ui.motion_timeline.clip_trigger(),
    ) else {
        return MotionOverrides::new();
    };
    let playhead = state.editor_ui.motion_timeline.playhead_ms.max(0.0).round() as u64;
    Clip::build(root, &trigger).sample(playhead, view)
}

/// Whether the canvas changes every frame on its own: the timeline is
/// playing. Hosts keep frames coming and blit caches stand aside.
pub fn motion_timeline_playing(state: &EditorState) -> bool {
    motion_timeline_shown(state) && state.editor_ui.motion_timeline.playing
}

#[cfg(test)]
#[path = "canvas_paint_scene_tests.rs"]
mod tests;
