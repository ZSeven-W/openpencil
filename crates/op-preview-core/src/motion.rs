//! P1 document-motion declarations layered on the existing preview timeline.

use crate::animation::{sample_scene_property, PreviewAnimationState};
use jian_core::action::animation_registry::animatable_property_registry;
use jian_core::action::services::{
    AnimationDirection, AnimationFillMode, AnimationProperty, AnimationRequest, Easing,
};
use jian_ops_schema::motion::{
    MotionPreference, MotionTrigger, NodeAnimation, NodeAnimationFillMode, P1_MOTION_PROPERTIES,
};
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};
use op_editor_ui::Rect;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

const MOTION_TRACK_BUDGET: usize = 32;

pub(crate) fn load_warnings(document: &jian_ops_schema::PenDocument) -> Vec<String> {
    let mut warnings = Vec::new();
    collect_load_warnings(&document.children, &mut warnings);
    if let Some(pages) = &document.pages {
        for page in pages {
            collect_load_warnings(&page.children, &mut warnings);
        }
    }
    warnings
}

fn collect_load_warnings(nodes: &[jian_ops_schema::node::PenNode], warnings: &mut Vec<String>) {
    for node in nodes {
        let (transition, animations) = node.motion_declarations();
        if let Some(transition) = transition {
            for property in transition.properties.as_deref().into_iter().flatten() {
                if !P1_MOTION_PROPERTIES.contains(&property.as_str()) {
                    warnings.push(format!(
                        "preview: motion property {property} on node {} is ignored for transition",
                        jian_core::document::tree::node_schema_id(node)
                    ));
                }
            }
        }
        if let Some(animations) = animations {
            for animation in animations {
                for keyframe in &animation.keyframes {
                    for property in keyframe.values.keys() {
                        if !P1_MOTION_PROPERTIES.contains(&property.as_str()) {
                            warnings.push(format!(
                                "preview: motion property {property} on node {} is ignored for animation",
                                jian_core::document::tree::node_schema_id(node)
                            ));
                        }
                    }
                }
            }
        }
        if let Some(children) = node.children_ref() {
            collect_load_warnings(children, warnings);
        }
    }
}

#[derive(Default)]
struct MotionInner {
    lifecycle_started: bool,
    observed: BTreeMap<(String, AnimationProperty), Value>,
    fired_in_view: BTreeSet<String>,
    visible_in_view: BTreeSet<String>,
    animation_tracks: BTreeSet<(String, AnimationProperty)>,
}

#[derive(Clone, Default)]
pub(crate) struct PreviewMotionState {
    inner: Rc<RefCell<MotionInner>>,
}

impl PreviewMotionState {
    pub(crate) fn reset_screen(&self) {
        *self.inner.borrow_mut() = MotionInner::default();
    }

    pub(crate) fn begin_lifecycle(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.lifecycle_started {
            return false;
        }
        inner.lifecycle_started = true;
        true
    }

    pub(crate) fn lifecycle_started(&self) -> bool {
        self.inner.borrow().lifecycle_started
    }

    fn prune_tracks(&self, animation: &PreviewAnimationState) {
        self.inner
            .borrow_mut()
            .animation_tracks
            .retain(|(target, property)| animation.has_active(target, property));
    }

    fn track_budget_available(
        &self,
        animation: &PreviewAnimationState,
        target: &str,
        property: &AnimationProperty,
    ) -> bool {
        self.prune_tracks(animation);
        let inner = self.inner.borrow();
        inner
            .animation_tracks
            .contains(&(target.to_owned(), property.clone()))
            || inner.animation_tracks.len() < MOTION_TRACK_BUDGET
    }

    fn remember_track(&self, target: &str, property: &AnimationProperty) {
        self.inner
            .borrow_mut()
            .animation_tracks
            .insert((target.to_owned(), property.clone()));
    }

    pub(crate) fn observe_transitions(
        &self,
        runtime: &jian_core::Runtime,
        scene: &LayoutScene,
        animation: &PreviewAnimationState,
        effective: MotionPreference,
        now_ms: u64,
    ) {
        let declarations = node_declarations(runtime, true);
        for (target, transition, _) in declarations {
            let Some(transition) = transition else {
                continue;
            };
            let Some(node) = scene.active_page().and_then(|page| page.find(&target)) else {
                continue;
            };
            let properties: Vec<&str> = transition
                .properties
                .as_ref()
                .map(|properties| properties.iter().map(String::as_str).collect())
                .unwrap_or_else(|| P1_MOTION_PROPERTIES.to_vec());
            for name in properties {
                if !P1_MOTION_PROPERTIES.contains(&name) {
                    tracing::warn!(target = %target, property = %name, "preview motion property rejected");
                    continue;
                }
                let Some(descriptor) = animatable_property_registry().get(name) else {
                    continue;
                };
                let property = AnimationProperty::from_registered(name, descriptor.apply);
                let Some(desired) = scene_property(node, &property) else {
                    continue;
                };
                let key = (target.clone(), property.clone());
                let previous = self
                    .inner
                    .borrow_mut()
                    .observed
                    .insert(key, desired.clone());
                let Some(previous) = previous else {
                    continue;
                };
                if previous == desired {
                    continue;
                }
                let current = animation
                    .current_override(&target, &property)
                    .unwrap_or(previous);
                let request = AnimationRequest {
                    target: target.clone(),
                    property: property.clone(),
                    from: Some(current.clone()),
                    to: desired.clone(),
                    stops: None,
                    duration_ms: if effective == MotionPreference::Reduced {
                        0
                    } else {
                        transition.duration_ms
                    },
                    delay_ms: 0,
                    easing: easing_from_schema(&transition.easing),
                    iterations: 1,
                    direction: AnimationDirection::Normal,
                    fill_mode: AnimationFillMode::Forwards,
                    requested_at_ms: now_ms,
                };
                if effective == MotionPreference::Reduced {
                    animation.set_instant(
                        &target,
                        &property,
                        desired,
                        Some(&scene_property(node, &property).unwrap_or(Value::Null)),
                    );
                } else {
                    let _ = animation.start_immediate(request, current, Some(desired), now_ms);
                }
            }
        }
    }

    pub(crate) fn observe_in_view(
        &self,
        runtime: &jian_core::Runtime,
        scene: &LayoutScene,
        animation: &PreviewAnimationState,
        effective: MotionPreference,
        now_ms: u64,
    ) {
        if !self.lifecycle_started() {
            return;
        }
        let Some(page) = scene.active_page() else {
            return;
        };
        let Some(viewport) = page.children.first().map(|node| node.bounds) else {
            return;
        };
        let declarations = node_declarations(runtime, false);
        let mut visible_now = BTreeSet::new();
        for (target, _, animations) in declarations {
            let Some(node) = page.find(&target) else {
                continue;
            };
            if !intersects_viewport(node.bounds, viewport) {
                continue;
            }
            for (index, declaration) in animations.iter().enumerate() {
                if declaration.trigger != MotionTrigger::InView {
                    continue;
                }
                let key = format!("{target}:{index}");
                visible_now.insert(key.clone());
                let once = declaration.once;
                let should_fire = if once {
                    !self.inner.borrow().fired_in_view.contains(&key)
                } else {
                    !self.inner.borrow().visible_in_view.contains(&key)
                };
                if !should_fire {
                    continue;
                }
                if once {
                    self.inner.borrow_mut().fired_in_view.insert(key);
                }
                self.start_node_animation(
                    target.as_str(),
                    declaration,
                    scene,
                    animation,
                    effective,
                    now_ms,
                );
            }
        }
        self.inner.borrow_mut().visible_in_view = visible_now;
    }

    pub(crate) fn start_mount_animations(
        &self,
        runtime: &jian_core::Runtime,
        scene: &LayoutScene,
        animation: &PreviewAnimationState,
        effective: MotionPreference,
        now_ms: u64,
    ) {
        for (target, _, animations) in node_declarations(runtime, false) {
            for declaration in animations {
                if declaration.trigger == MotionTrigger::Mount {
                    self.start_node_animation(
                        &target,
                        &declaration,
                        scene,
                        animation,
                        effective,
                        now_ms,
                    );
                }
            }
        }
    }

    pub(crate) fn set_initial_lifecycle_values(
        &self,
        runtime: &jian_core::Runtime,
        scene: &LayoutScene,
        animation: &PreviewAnimationState,
    ) {
        for (target, _, animations) in node_declarations(runtime, false) {
            for declaration in animations {
                if !matches!(
                    declaration.trigger,
                    MotionTrigger::Mount | MotionTrigger::InView
                ) {
                    continue;
                }
                self.set_node_animation_initial_values(&target, &declaration, scene, animation);
            }
        }
    }

    fn set_node_animation_initial_values(
        &self,
        target: &str,
        declaration: &NodeAnimation,
        scene: &LayoutScene,
        animation: &PreviewAnimationState,
    ) {
        let Some(_node) = scene.active_page().and_then(|page| page.find(target)) else {
            return;
        };
        let mut by_property: BTreeMap<String, Vec<(f32, Value)>> = BTreeMap::new();
        for keyframe in &declaration.keyframes {
            for (property, value) in &keyframe.values {
                by_property
                    .entry(property.clone())
                    .or_default()
                    .push((keyframe.offset, value.clone()));
            }
        }
        for (name, stops) in by_property {
            if !P1_MOTION_PROPERTIES.contains(&name.as_str()) {
                continue;
            }
            let Some(descriptor) = animatable_property_registry().get(&name) else {
                continue;
            };
            let property = AnimationProperty::from_registered(&name, descriptor.apply);
            let Some(base) = sample_scene_property(scene, target, &property) else {
                continue;
            };
            let Some((_, first)) = stops.first() else {
                continue;
            };
            animation.set_instant(target, &property, first.clone(), Some(&base));
        }
    }

    fn start_node_animation(
        &self,
        target: &str,
        declaration: &NodeAnimation,
        scene: &LayoutScene,
        animation: &PreviewAnimationState,
        effective: MotionPreference,
        now_ms: u64,
    ) {
        let Some(_node) = scene.active_page().and_then(|page| page.find(target)) else {
            return;
        };
        let mut by_property: BTreeMap<String, Vec<(f32, Value)>> = BTreeMap::new();
        for keyframe in &declaration.keyframes {
            for (property, value) in &keyframe.values {
                by_property
                    .entry(property.clone())
                    .or_default()
                    .push((keyframe.offset, value.clone()));
            }
        }
        for (name, stops) in by_property {
            if !P1_MOTION_PROPERTIES.contains(&name.as_str()) {
                tracing::warn!(target = %target, property = %name, "preview motion property rejected");
                continue;
            }
            let Some(descriptor) = animatable_property_registry().get(&name) else {
                continue;
            };
            let property = AnimationProperty::from_registered(&name, descriptor.apply);
            let Some(base) = sample_scene_property(scene, target, &property) else {
                continue;
            };
            let Some(first) = stops.first() else {
                continue;
            };
            let final_value = stops.last().expect("first stop exists").1.clone();
            if effective == MotionPreference::Reduced {
                if declaration.fill_mode == NodeAnimationFillMode::Forwards {
                    animation.set_instant(target, &property, final_value, Some(&base));
                }
                continue;
            }
            if !self.track_budget_available(animation, target, &property) {
                tracing::warn!(target = %target, property = %name, "preview motion track budget exceeded");
                continue;
            }
            let request = AnimationRequest {
                target: target.to_owned(),
                property: property.clone(),
                from: Some(first.1.clone()),
                to: final_value,
                stops: Some(stops),
                duration_ms: declaration.duration_ms,
                delay_ms: 0,
                easing: easing_from_schema(&declaration.easing),
                iterations: declaration.iterations.max(1),
                direction: AnimationDirection::Normal,
                fill_mode: match declaration.fill_mode {
                    NodeAnimationFillMode::Forwards => AnimationFillMode::Forwards,
                    NodeAnimationFillMode::None => AnimationFillMode::None,
                },
                requested_at_ms: now_ms.saturating_add(declaration.delay_ms),
            };
            if animation.start_immediate(request, base.clone(), Some(base), now_ms)
                && animation.has_active(target, &property)
            {
                self.remember_track(target, &property);
            }
        }
    }
}

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
        self.motion.observe_in_view(
            &self.runtime,
            scene,
            &self.animation,
            effective,
            self.last_now_ms,
        );
    }
}

fn node_declarations(
    runtime: &jian_core::Runtime,
    transitions_only: bool,
) -> Vec<(
    String,
    Option<jian_ops_schema::motion::Transition>,
    Vec<NodeAnimation>,
)> {
    let mut result = Vec::new();
    let Some(document) = runtime.document.as_ref() else {
        return result;
    };
    for key in document.tree.keys_top_down() {
        let node = &document.tree.nodes[key].schema;
        let (transition, animations) = node.motion_declarations();
        let transition = transition.cloned();
        let animations = animations.cloned().unwrap_or_default();
        if (transitions_only && transition.is_some())
            || (!transitions_only && (transition.is_some() || !animations.is_empty()))
        {
            result.push((
                jian_core::document::tree::node_schema_id(node).to_owned(),
                transition,
                animations,
            ));
        }
    }
    result
}

fn scene_property(node: &SceneNode, property: &AnimationProperty) -> Option<Value> {
    Some(match property {
        AnimationProperty::Opacity => serde_json::json!(node.opacity),
        AnimationProperty::TranslateX | AnimationProperty::TranslateY => serde_json::json!(0.0),
        AnimationProperty::Rotation => serde_json::json!(node.rotation.to_degrees()),
        AnimationProperty::ScaleX | AnimationProperty::ScaleY => serde_json::json!(1.0),
        AnimationProperty::Fill => serde_json::Value::String(scene_color(node.fill?)),
        AnimationProperty::Stroke => serde_json::Value::String(scene_color(node.stroke?.color)),
        AnimationProperty::CornerRadius => serde_json::json!(node.corner_radius),
        _ => return None,
    })
}

fn scene_color(color: op_editor_ui::Color) -> String {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    if color.a >= 0.999 {
        format!(
            "#{:02x}{:02x}{:02x}",
            channel(color.r),
            channel(color.g),
            channel(color.b)
        )
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            channel(color.r),
            channel(color.g),
            channel(color.b),
            channel(color.a)
        )
    }
}

fn easing_from_schema(easing: &jian_ops_schema::motion::Easing) -> Easing {
    match easing {
        jian_ops_schema::motion::Easing::Linear => Easing::Linear,
        jian_ops_schema::motion::Easing::Ease => Easing::Ease,
        jian_ops_schema::motion::Easing::EaseIn => Easing::EaseIn,
        jian_ops_schema::motion::Easing::EaseOut => Easing::EaseOut,
        jian_ops_schema::motion::Easing::EaseInOut => Easing::EaseInOut,
        jian_ops_schema::motion::Easing::Standard => Easing::Standard,
        jian_ops_schema::motion::Easing::Emphasized => Easing::Emphasized,
        jian_ops_schema::motion::Easing::EmphasizedDecelerate => Easing::EmphasizedDecelerate,
        jian_ops_schema::motion::Easing::EmphasizedAccelerate => Easing::EmphasizedAccelerate,
        jian_ops_schema::motion::Easing::CubicBezier(a, b, c, d) => {
            Easing::CubicBezier(*a, *b, *c, *d)
        }
    }
}

fn intersects_viewport(node: Rect, viewport: Rect) -> bool {
    if node.size.y <= 0.0 || node.size.x <= 0.0 {
        return false;
    }
    let left = node.origin.x.max(viewport.origin.x);
    let right = (node.origin.x + node.size.x).min(viewport.origin.x + viewport.size.x);
    let top = node.origin.y.max(viewport.origin.y);
    let bottom = (node.origin.y + node.size.y).min(viewport.origin.y + viewport.size.y);
    right > left && bottom > top && (bottom - top) / node.size.y >= 0.1
}
