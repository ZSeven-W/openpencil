//! One bounded animation timeline owned by a Preview session: Jian's
//! motion timeline plus Preview's debug-trace hook and scene painting.

use crate::invalidation::InvalidationKind;
use crate::session::PreviewSession;
use jian_core::action::animation_registry::animatable_property_registry;
use jian_core::action::services::AnimationProperty;
use jian_core::motion::{MotionActionSink, MotionObserver, MotionTimeline};
use jian_scene::layout_scene_motion::{apply_motion_overrides, sample_scene_property};
use op_editor_ui::layout_scene::LayoutScene;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Default)]
pub(crate) struct PreviewAnimationState {
    pub(crate) sink: MotionActionSink,
}

impl Deref for PreviewAnimationState {
    type Target = MotionTimeline;

    fn deref(&self) -> &MotionTimeline {
        &self.sink.timeline
    }
}

/// Records accepted animation requests in the Preview debug trace and logs
/// the motion declarations the triggers had to skip.
struct SessionObserver(crate::debug_trace::PreviewDebugTrace);

impl MotionObserver for SessionObserver {
    fn animation_requested(&self, target: &str, property: &str, at_ms: u64) {
        self.0.record_animation(target, property, at_ms, "request");
    }

    fn motion_warning(&self, message: &str) {
        tracing::warn!("preview: {message}");
    }
}

impl PreviewAnimationState {
    pub(crate) fn clear(&self) {
        self.sink.clear();
    }

    pub(crate) fn set_trace(&self, trace: crate::debug_trace::PreviewDebugTrace) {
        self.sink
            .timeline
            .set_observer(Rc::new(SessionObserver(trace)));
    }

    pub(crate) fn apply_to_scene(&self, scene: &mut LayoutScene) {
        apply_motion_overrides(scene, &self.sink.timeline.overrides());
    }
}

impl PreviewSession {
    pub fn active_animation_track_count(&self) -> usize {
        self.animation.track_count()
    }

    pub fn last_animation_invalidation(&self) -> InvalidationKind {
        self.animation.last_invalidation()
    }

    pub(crate) fn admit_animation_requests(&mut self, now_ms: u64) -> InvalidationKind {
        let structured = self.admit_structured_requests(now_ms);
        let manual = self.admit_motion_controls();
        if manual != InvalidationKind::None {
            let values = self.animation.binding_values();
            self.apply_invalidation_with_values(manual, &values);
        }
        structured.merge(manual)
    }

    fn admit_structured_requests(&mut self, now_ms: u64) -> InvalidationKind {
        let animation = self.animation.clone();
        let requests = animation.take_pending();
        if requests.is_empty() {
            return InvalidationKind::None;
        }
        let base_scene = self.overlay_runtime_state_without_animation(&self.scene);
        let current_scene = self.overlay_runtime_state(&self.scene);
        for request in requests {
            let target_exists = base_scene
                .active_page()
                .and_then(|page| page.find(&request.target))
                .is_some();
            if !target_exists {
                self.warnings.push(format!(
                    "AnimationTargetMissing: '{}' property '{}'",
                    request.target,
                    request.property.name()
                ));
                continue;
            }
            let base = sample_scene_property(&base_scene, &request.target, &request.property);
            let current = animation
                .current_override(&request.target, &request.property)
                .or_else(|| {
                    sample_scene_property(&current_scene, &request.target, &request.property)
                });
            let from = request.from.clone().or(current);
            let Some(from) = from else {
                self.warnings.push(format!(
                    "AnimationValueMissing: '{}' property '{}'",
                    request.target,
                    request.property.name()
                ));
                continue;
            };
            animation.admit(request, from, base);
        }
        let invalidation = animation.tick(now_ms);
        let values = animation.binding_values();
        self.apply_invalidation_with_values(invalidation, &values);
        invalidation
    }

    pub(crate) fn tick_animation(&mut self, now_ms: u64) -> InvalidationKind {
        let admission = self.admit_animation_requests(now_ms);
        if admission != InvalidationKind::None {
            return admission;
        }
        let animation = self.animation.clone();
        let tick = animation.tick(now_ms);
        let invalidation = admission.merge(tick);
        let values = animation.binding_values();
        self.apply_invalidation_with_values(invalidation, &values);
        invalidation
    }

    fn admit_motion_controls(&mut self) -> InvalidationKind {
        let controls = self.animation.sink.take_controls();
        if controls.is_empty() {
            return InvalidationKind::None;
        }
        let base = self.overlay_runtime_state_without_animation(&self.scene);
        let effective = self.effective_motion_preference();
        let before = self.animation.overrides();
        for (control, at_ms) in controls {
            let count = self.motion.control(
                &control,
                &self.runtime,
                &base,
                &self.animation,
                effective,
                at_ms,
            );
            if count == 0 {
                self.warnings.push(format!(
                    "play_motion: no manual animation '{}' in subtree '{}'",
                    control.animation, control.target,
                ));
            }
        }
        let after = self.animation.overrides();
        before
            .keys()
            .chain(after.keys())
            .filter(|key| before.get(*key) != after.get(*key))
            .filter_map(|(_, property)| property.invalidation())
            .fold(InvalidationKind::None, InvalidationKind::merge)
    }

    pub fn cancel_animation(&mut self, target: &str, property: &str) -> bool {
        let Some(descriptor) = animatable_property_registry().get(property) else {
            return false;
        };
        let property = AnimationProperty::from_registered(property, descriptor.apply);
        let animation = self.animation.clone();
        let Some(invalidation) = animation.cancel(target, &property) else {
            return false;
        };
        let values = animation.binding_values();
        self.apply_invalidation_with_values(invalidation, &values);
        true
    }
}

/// Read-only access to the sampling path for the motion golden-vector
/// generator (`tests_motion_golden.rs`). It forwards to the production
/// functions unchanged, so the recorded vectors are today's behaviour.
#[cfg(all(test, not(target_os = "windows")))]
pub(crate) mod golden_access {
    use super::PreviewAnimationState;
    use jian_core::action::services::{AnimationRequest, Easing};

    pub(crate) fn ease(easing: Easing, progress: f32) -> f32 {
        jian_core::motion::ease(easing, progress)
    }

    /// Sample one request the way `tick` samples an admitted track.
    pub(crate) fn sample(
        request: &AnimationRequest,
        now_ms: u64,
    ) -> (Option<serde_json::Value>, bool) {
        jian_core::motion::sample_request(request, now_ms)
    }

    /// Every live override as `(target, property name, value)`.
    pub(crate) fn overrides(
        state: &PreviewAnimationState,
    ) -> Vec<(String, String, serde_json::Value)> {
        state
            .overrides()
            .into_iter()
            .map(|((target, property), value)| (target, property.name().to_owned(), value))
            .collect()
    }
}
