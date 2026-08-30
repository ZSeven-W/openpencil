//! One bounded animation timeline owned by a Preview session.

use crate::binding_overlay::apply_value;
use crate::invalidation::InvalidationKind;
use crate::session::PreviewSession;
use jian_core::action::animation_registry::{animatable_property_registry, AnimationInterpolate};
use jian_core::action::services::{
    AnimationDirection, AnimationFillMode, AnimationOutcome, AnimationProperty, AnimationRequest,
    AnimationSink, Easing,
};
use jian_core::binding::BindingTarget;
use jian_core::value::RuntimeValue;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

const FRAME_INTERVAL_MS: u64 = 16;
const MAX_TRACKS: usize = 256;

type TrackKey = (String, AnimationProperty);

#[derive(Clone)]
struct Track {
    request: AnimationRequest,
    from: serde_json::Value,
    base: Option<serde_json::Value>,
}

#[derive(Default)]
pub(crate) struct AnimationTimeline {
    pending: VecDeque<AnimationRequest>,
    tracks: Vec<Track>,
    overrides: BTreeMap<TrackKey, serde_json::Value>,
    override_invalidation: BTreeMap<TrackKey, InvalidationKind>,
    now_ms: u64,
    last_invalidation: InvalidationKind,
}

impl AnimationTimeline {
    pub(crate) fn next_deadline_ms(&self) -> Option<u64> {
        self.tracks
            .iter()
            .filter_map(|track| track_deadline(track, self.now_ms))
            .min()
    }
}

#[derive(Clone, Default)]
pub(crate) struct PreviewAnimationState {
    inner: Rc<RefCell<AnimationTimeline>>,
    trace: Rc<RefCell<Option<crate::debug_trace::PreviewDebugTrace>>>,
}

impl AnimationSink for PreviewAnimationState {
    fn request(&self, request: &AnimationRequest) -> AnimationOutcome {
        let mut inner = self.inner.borrow_mut();
        let key = (request.target.clone(), request.property.clone());
        let keys: BTreeSet<_> = inner
            .tracks
            .iter()
            .map(track_key)
            .chain(inner.pending.iter().map(request_key))
            .chain(inner.overrides.keys().cloned())
            .collect();
        if !keys.contains(&key) && keys.len() >= MAX_TRACKS {
            return AnimationOutcome::Rejected("animation timeline is full".to_owned());
        }
        inner.pending.retain(|pending| request_key(pending) != key);
        inner.pending.push_back(request.clone());
        drop(inner);
        if let Some(trace) = self.trace.borrow().as_ref() {
            trace.record_animation(
                &request.target,
                request.property.name(),
                request.requested_at_ms,
                "request",
            );
        }
        AnimationOutcome::Accepted
    }
}

impl PreviewAnimationState {
    pub(crate) fn set_trace(&self, trace: crate::debug_trace::PreviewDebugTrace) {
        *self.trace.borrow_mut() = Some(trace);
    }

    pub(crate) fn take_pending(&self) -> Vec<AnimationRequest> {
        self.inner.borrow_mut().pending.drain(..).collect()
    }

    pub(crate) fn admit(
        &self,
        mut request: AnimationRequest,
        from: serde_json::Value,
        base: Option<serde_json::Value>,
    ) {
        let key = request_key(&request);
        let mut inner = self.inner.borrow_mut();
        inner.tracks.retain(|track| track_key(track) != key);
        inner.overrides.remove(&key);
        inner.override_invalidation.remove(&key);
        request.from = Some(from.clone());
        inner.tracks.push(Track {
            request,
            from,
            base,
        });
    }

    pub(crate) fn tick(&self, now_ms: u64) -> InvalidationKind {
        let mut inner = self.inner.borrow_mut();
        inner.now_ms = inner.now_ms.max(now_ms);
        let now_ms = inner.now_ms;
        let tracks = std::mem::take(&mut inner.tracks);
        let mut retained = Vec::with_capacity(tracks.len());
        let mut invalidation = InvalidationKind::None;
        for track in tracks {
            let key = track_key(&track);
            let before = inner.overrides.get(&key).cloned();
            let sample = sample_track(&track, now_ms);
            match sample.value {
                Some(value) if track.base.as_ref() != Some(&value) => {
                    inner.overrides.insert(key.clone(), value);
                    let kind =
                        crate::invalidation::from_animation_property(&track.request.property);
                    inner.override_invalidation.insert(key.clone(), kind);
                }
                _ => {
                    inner.overrides.remove(&key);
                    inner.override_invalidation.remove(&key);
                }
            }
            let after = inner.overrides.get(&key);
            if before.as_ref() != after {
                invalidation = invalidation.merge(crate::invalidation::from_animation_property(
                    &track.request.property,
                ));
            }
            if !sample.finished {
                retained.push(track);
            }
        }
        inner.tracks = retained;
        inner.last_invalidation = invalidation;
        invalidation
    }

    pub(crate) fn next_deadline_ms(&self) -> Option<u64> {
        self.inner.borrow().next_deadline_ms()
    }

    pub(crate) fn apply_to_scene(&self, scene: &mut LayoutScene) {
        let overrides = self.inner.borrow().overrides.clone();
        let Some(page) = scene.pages.get_mut(scene.active_page_index) else {
            return;
        };
        for node in &mut page.children {
            apply_overrides(node, &overrides);
        }
    }

    pub(crate) fn has_visual_state(&self) -> bool {
        !self.inner.borrow().overrides.is_empty()
    }

    pub(crate) fn binding_values(&self) -> BTreeMap<(String, BindingTarget), serde_json::Value> {
        self.inner
            .borrow()
            .overrides
            .iter()
            .filter_map(|((target, property), value)| {
                property
                    .binding_target()
                    .map(|binding| ((target.clone(), binding), value.clone()))
            })
            .collect()
    }

    pub(crate) fn current_override(
        &self,
        target: &str,
        property: &AnimationProperty,
    ) -> Option<serde_json::Value> {
        self.inner
            .borrow()
            .overrides
            .get(&(target.to_owned(), property.clone()))
            .cloned()
    }

    pub(crate) fn cancel(
        &self,
        target: &str,
        property: &AnimationProperty,
    ) -> Option<InvalidationKind> {
        let key = (target.to_owned(), property.clone());
        let mut inner = self.inner.borrow_mut();
        let before_tracks = inner.tracks.len();
        let before_pending = inner.pending.len();
        inner.tracks.retain(|track| track_key(track) != key);
        inner.pending.retain(|request| request_key(request) != key);
        let removed_override = inner.overrides.remove(&key).is_some();
        let invalidation = inner.override_invalidation.remove(&key);
        let removed_track = inner.tracks.len() != before_tracks;
        let removed_pending = inner.pending.len() != before_pending;
        if removed_override || removed_track {
            let kind = invalidation
                .unwrap_or_else(|| crate::invalidation::from_animation_property(property));
            inner.last_invalidation = kind;
            Some(kind)
        } else if removed_pending {
            inner.last_invalidation = InvalidationKind::None;
            Some(InvalidationKind::None)
        } else {
            None
        }
    }

    pub(crate) fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.pending.clear();
        inner.tracks.clear();
        inner.overrides.clear();
        inner.override_invalidation.clear();
        inner.last_invalidation = InvalidationKind::None;
    }

    pub(crate) fn track_count(&self) -> usize {
        self.inner.borrow().tracks.len()
    }

    pub(crate) fn last_invalidation(&self) -> InvalidationKind {
        self.inner.borrow().last_invalidation
    }
}

struct TrackSample {
    value: Option<serde_json::Value>,
    finished: bool,
}

fn sample_track(track: &Track, now_ms: u64) -> TrackSample {
    let start = track
        .request
        .requested_at_ms
        .saturating_add(track.request.delay_ms);
    let total = track
        .request
        .duration_ms
        .saturating_mul(u64::from(track.request.iterations));
    let end = start.saturating_add(total);
    if now_ms < start {
        let value = matches!(
            track.request.fill_mode,
            AnimationFillMode::Backwards | AnimationFillMode::Both
        )
        .then(|| interpolate_track(track, directed_progress(track, 0, 0.0)));
        return TrackSample {
            value,
            finished: false,
        };
    }
    if now_ms >= end {
        let last_iteration = track.request.iterations.saturating_sub(1);
        let value = matches!(
            track.request.fill_mode,
            AnimationFillMode::Forwards | AnimationFillMode::Both
        )
        .then(|| interpolate_track(track, directed_progress(track, last_iteration, 1.0)));
        return TrackSample {
            value,
            finished: true,
        };
    }
    let elapsed = now_ms - start;
    let iteration = (elapsed / track.request.duration_ms) as u32;
    let local = (elapsed % track.request.duration_ms) as f32 / track.request.duration_ms as f32;
    TrackSample {
        value: Some(interpolate_track(
            track,
            directed_progress(track, iteration, local),
        )),
        finished: false,
    }
}

fn directed_progress(track: &Track, iteration: u32, local: f32) -> f32 {
    let reverse = match track.request.direction {
        AnimationDirection::Normal => false,
        AnimationDirection::Reverse => true,
        AnimationDirection::Alternate => iteration % 2 == 1,
        AnimationDirection::AlternateReverse => iteration.is_multiple_of(2),
    };
    let directed = if reverse { 1.0 - local } else { local };
    ease(track.request.easing, directed.clamp(0.0, 1.0))
}

fn ease(easing: Easing, progress: f32) -> f32 {
    match easing {
        Easing::Linear => progress,
        Easing::Ease => progress * progress * (3.0 - 2.0 * progress),
        Easing::EaseIn => progress * progress,
        Easing::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
        Easing::EaseInOut if progress < 0.5 => 2.0 * progress * progress,
        Easing::EaseInOut => 1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0,
    }
}

fn interpolate_track(track: &Track, progress: f32) -> serde_json::Value {
    let interpolate = animatable_property_registry()
        .get(track.request.property.name())
        .map(|entry| entry.interpolate)
        .unwrap_or(AnimationInterpolate::Discrete);
    match interpolate {
        AnimationInterpolate::Linear => {
            interpolate_number(&track.from, &track.request.to, progress)
        }
        AnimationInterpolate::ColorSrgb => {
            interpolate_color(&track.from, &track.request.to, progress)
        }
        AnimationInterpolate::Discrete => {
            if progress >= 1.0 {
                track.request.to.clone()
            } else {
                track.from.clone()
            }
        }
    }
}

fn interpolate_number(
    from: &serde_json::Value,
    to: &serde_json::Value,
    progress: f32,
) -> serde_json::Value {
    let from = from.as_f64().unwrap_or_default();
    let to = to.as_f64().unwrap_or_default();
    serde_json::Number::from_f64(from + (to - from) * f64::from(progress))
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

fn interpolate_color(
    from: &serde_json::Value,
    to: &serde_json::Value,
    progress: f32,
) -> serde_json::Value {
    let Some(from) = from.as_str().and_then(jian_core::scene::Color::from_hex) else {
        return from.clone();
    };
    let Some(to) = to.as_str().and_then(jian_core::scene::Color::from_hex) else {
        return serde_json::Value::String(color_hex(from));
    };
    let channel = |from: u8, to: u8| {
        (f32::from(from) + (f32::from(to) - f32::from(from)) * progress)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    serde_json::Value::String(color_hex(jian_core::scene::Color::rgba(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
        channel(from.a(), to.a()),
    )))
}

fn color_hex(color: jian_core::scene::Color) -> String {
    if color.a() == 255 {
        format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.r(),
            color.g(),
            color.b(),
            color.a()
        )
    }
}

fn track_deadline(track: &Track, now_ms: u64) -> Option<u64> {
    let start = track
        .request
        .requested_at_ms
        .saturating_add(track.request.delay_ms);
    let end = start.saturating_add(
        track
            .request
            .duration_ms
            .saturating_mul(u64::from(track.request.iterations)),
    );
    if now_ms < start {
        return Some(start);
    }
    if now_ms >= end {
        return None;
    }
    let elapsed = now_ms - start;
    let next_iteration = start
        .saturating_add(
            (elapsed / track.request.duration_ms + 1).saturating_mul(track.request.duration_ms),
        )
        .min(end);
    let interpolate = animatable_property_registry()
        .get(track.request.property.name())
        .map(|entry| entry.interpolate)
        .unwrap_or(AnimationInterpolate::Discrete);
    if interpolate == AnimationInterpolate::Discrete {
        Some(next_iteration)
    } else {
        Some(now_ms.saturating_add(FRAME_INTERVAL_MS).min(next_iteration))
    }
}

fn request_key(request: &AnimationRequest) -> TrackKey {
    (request.target.clone(), request.property.clone())
}

fn track_key(track: &Track) -> TrackKey {
    request_key(&track.request)
}

fn apply_overrides(node: &mut SceneNode, overrides: &BTreeMap<TrackKey, serde_json::Value>) {
    let mut node_overrides: Vec<_> = overrides
        .iter()
        .filter(|((target, _), _)| target == &node.id)
        .filter_map(|((_, property), value)| {
            property
                .binding_target()
                .map(|target| (target, RuntimeValue(value.clone())))
        })
        .collect();
    node_overrides.sort_by_key(|(target, _)| target.application_order());
    for (target, value) in node_overrides {
        apply_value(node, target, &value);
    }
    for child in &mut node.children {
        apply_overrides(child, overrides);
    }
}

fn sample_scene_property(
    scene: &LayoutScene,
    target: &str,
    property: &AnimationProperty,
) -> Option<serde_json::Value> {
    let node = scene.active_page()?.find(target)?;
    Some(match property {
        AnimationProperty::Opacity => serde_json::json!(node.opacity),
        AnimationProperty::X => serde_json::json!(node.bounds.origin.x),
        AnimationProperty::Y => serde_json::json!(node.bounds.origin.y),
        AnimationProperty::Rotation => serde_json::json!(node.rotation.to_degrees()),
        AnimationProperty::ScaleX | AnimationProperty::ScaleY => serde_json::json!(1.0),
        AnimationProperty::Fill => serde_json::Value::String(scene_color(node.fill?)),
        AnimationProperty::Stroke => serde_json::Value::String(scene_color(node.stroke?.color)),
        AnimationProperty::CornerRadius => serde_json::json!(node.corner_radius),
        AnimationProperty::Width => serde_json::json!(node.bounds.size.x),
        AnimationProperty::Height => serde_json::json!(node.bounds.size.y),
        AnimationProperty::ShaderUniform(_) | AnimationProperty::Custom(_) => return None,
    })
}

fn scene_color(color: op_editor_ui::Color) -> String {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    color_hex(jian_core::scene::Color::rgba(
        channel(color.r),
        channel(color.g),
        channel(color.b),
        channel(color.a),
    ))
}

impl PreviewSession {
    pub fn active_animation_track_count(&self) -> usize {
        self.animation.track_count()
    }

    pub fn last_animation_invalidation(&self) -> InvalidationKind {
        self.animation.last_invalidation()
    }

    pub(crate) fn admit_animation_requests(&mut self, now_ms: u64) -> InvalidationKind {
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
