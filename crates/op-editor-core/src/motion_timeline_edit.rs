//! Timeline gestures written back onto one animation.
//!
//! Each function takes an animation and returns the edited copy, keeping the
//! schema's invariants (offsets ascend from 0 to 1) and changing nothing the
//! gesture does not touch. Keyframe offsets live in eased progress, so a
//! keyframe at time `t` has offset `E((t − s) / d)`; when a gesture moves
//! the start or the end, the other keyframes stay at their times and only
//! their offsets are recomputed. Gestures run only when the person edits:
//! opening, saving or playing a document rewrites nothing.

use crate::motion_timeline_projection::keyframe_ms;
use jian_core::action::animation_registry::{animatable_property_registry, AnimationInterpolate};
use jian_core::action::services::Easing as RuntimeEasing;
use jian_core::motion::ease;
use jian_core::motion::interpolate::interpolate_stops;
use jian_ops_schema::events::ExtraJson;
use jian_ops_schema::motion::{Easing, Keyframe, NodeAnimation};
use serde_json::Value;
use std::collections::BTreeMap;

/// Drag the whole strip: `delayMs += delta`, never below zero.
pub fn move_strip(animation: &NodeAnimation, delta_ms: i64) -> NodeAnimation {
    let mut edited = animation.clone();
    edited.delay_ms = (animation.delay_ms as i64).saturating_add(delta_ms).max(0) as u64;
    edited
}

/// Drag the first iteration's end to `end_ms`: the duration changes, the
/// offsets do not, so every keyframe time scales with it.
pub fn resize_strip(animation: &NodeAnimation, end_ms: f64) -> NodeAnimation {
    let mut edited = animation.clone();
    edited.duration_ms = duration_between(animation.delay_ms as f64, end_ms);
    edited
}

/// Drag keyframe `index` to `time_ms`. A middle keyframe stays between its
/// neighbours; the first or last one moves the start or the end while the
/// others keep their times.
pub fn move_keyframe(animation: &NodeAnimation, index: usize, time_ms: f64) -> NodeAnimation {
    let mut times = keyframe_times(animation);
    let Some(last) = times.len().checked_sub(1) else {
        return animation.clone();
    };
    if index > last {
        return animation.clone();
    }
    let low = if index == 0 { 0.0 } else { times[index - 1] };
    let high = if index == last {
        f64::MAX
    } else {
        times[index + 1]
    };
    times[index] = time_ms.clamp(low, high);
    retime_keyframes(animation, &times)
}

/// Shift several keyframes of one animation together by `delta_ms`. When
/// the selection holds every keyframe this is a strip move.
pub fn shift_keyframes(
    animation: &NodeAnimation,
    indices: &[usize],
    delta_ms: f64,
) -> NodeAnimation {
    let count = animation.keyframes.len();
    if count > 0 && (0..count).all(|index| indices.contains(&index)) {
        return move_strip(animation, delta_ms.round() as i64);
    }
    let original = keyframe_times(animation);
    let mut times = original.clone();
    for &index in indices.iter().filter(|&&index| index < count) {
        times[index] = (original[index] + delta_ms).max(0.0);
    }
    // Moved keyframes stop at the nearest keyframe that stayed put.
    for index in 0..count {
        if indices.contains(&index) {
            let low = (0..index)
                .rev()
                .find(|other| !indices.contains(other))
                .map_or(0.0, |other| times[other]);
            let high = (index + 1..count)
                .find(|other| !indices.contains(other))
                .map_or(f64::MAX, |other| times[other]);
            times[index] = times[index].clamp(low, high);
        }
    }
    retime_keyframes(animation, &times)
}

/// Add a keyframe at `time_ms` holding the values the animation has there.
/// Inside the strip the curve is unchanged; outside it the strip grows to
/// reach `time_ms`.
pub fn insert_keyframe(animation: &NodeAnimation, time_ms: f64) -> NodeAnimation {
    let start = animation.delay_ms as f64;
    let duration = animation.duration_ms as f64;
    let mut times = keyframe_times(animation);
    let mut keyframes = animation.keyframes.clone();
    if time_ms < start {
        keyframes.insert(0, keyframe(0.0, sample_values(animation, 0.0)));
        times.insert(0, time_ms.max(0.0));
    } else if time_ms > start + duration {
        keyframes.push(keyframe(1.0, sample_values(animation, 1.0)));
        times.push(time_ms);
    } else {
        let progress = ((time_ms - start) / duration.max(1.0)) as f32;
        let offset = ease(
            RuntimeEasing::from(&animation.easing),
            progress.clamp(0.0, 1.0),
        );
        if animation
            .keyframes
            .iter()
            .any(|existing| (existing.offset - offset).abs() < 1e-6)
        {
            return animation.clone();
        }
        let mut edited = animation.clone();
        let at = edited
            .keyframes
            .iter()
            .position(|existing| existing.offset > offset)
            .unwrap_or(edited.keyframes.len());
        edited
            .keyframes
            .insert(at, keyframe(offset, sample_values(animation, offset)));
        return edited;
    }
    let mut grown = animation.clone();
    grown.keyframes = keyframes;
    retime_keyframes(&grown, &times)
}

/// Remove keyframe `index`. Removing the first or last hands its role to
/// the neighbour, which keeps its time. `None` when only two keyframes
/// remain: deleting one deletes the animation.
pub fn delete_keyframe(animation: &NodeAnimation, index: usize) -> Option<NodeAnimation> {
    let count = animation.keyframes.len();
    if count <= 2 {
        return None;
    }
    if index >= count {
        return Some(animation.clone());
    }
    let mut times = keyframe_times(animation);
    let mut edited = animation.clone();
    edited.keyframes.remove(index);
    times.remove(index);
    if index != 0 && index != count - 1 {
        return Some(edited);
    }
    Some(retime_keyframes(&edited, &times))
}

/// Change the curve while every keyframe stays at its time:
/// `offset' = E'(E⁻¹(offset))`.
pub fn set_easing_keep_times(animation: &NodeAnimation, easing: Easing) -> NodeAnimation {
    let times = keyframe_times(animation);
    let mut edited = animation.clone();
    edited.easing = easing;
    let start = animation.delay_ms as f64;
    let duration = (animation.duration_ms as f64).max(1.0);
    let curve = RuntimeEasing::from(&edited.easing);
    let last = edited.keyframes.len().saturating_sub(1);
    for (index, keyframe) in edited.keyframes.iter_mut().enumerate() {
        if index != 0 && index != last {
            let progress = ((times[index] - start) / duration).clamp(0.0, 1.0) as f32;
            keyframe.offset = ease(curve, progress);
        }
    }
    normalize_offsets(&mut edited.keyframes);
    edited
}

/// Every property's value at eased progress `offset`, the way Preview
/// interpolates: a property moves only between the keyframes that list it
/// and holds its first and last values outside them.
pub fn sample_values(animation: &NodeAnimation, offset: f32) -> BTreeMap<String, Value> {
    let mut stops: BTreeMap<&str, Vec<(f32, Value)>> = BTreeMap::new();
    for keyframe in &animation.keyframes {
        for (property, value) in &keyframe.values {
            stops
                .entry(property.as_str())
                .or_default()
                .push((keyframe.offset, value.clone()));
        }
    }
    stops
        .into_iter()
        .map(|(property, stops)| {
            let kind = animatable_property_registry()
                .get(property)
                .map_or(AnimationInterpolate::Discrete, |entry| entry.interpolate);
            (property.to_owned(), interpolate_stops(&stops, kind, offset))
        })
        .collect()
}

/// Rebuild an animation from one time per keyframe: the first time is the
/// new start, the last the new end of the first iteration, and the keyframes
/// in between take the offsets their times map to. When start and duration
/// do not change, keyframes whose times did not change keep their offsets
/// exactly instead of round-tripping through the curve.
pub fn retime_keyframes(animation: &NodeAnimation, times_ms: &[f64]) -> NodeAnimation {
    let count = animation.keyframes.len();
    if count < 2 || times_ms.len() != count {
        return animation.clone();
    }
    let original = keyframe_times(animation);
    let mut times = times_ms.to_vec();
    for index in 1..count {
        times[index] = times[index].max(times[index - 1]);
    }
    let start = times[0].max(0.0).round();
    let duration = duration_between(start, times[count - 1]);
    let mut edited = animation.clone();
    edited.delay_ms = start as u64;
    edited.duration_ms = duration;
    let span_kept = edited.delay_ms == animation.delay_ms && duration == animation.duration_ms;
    let curve = RuntimeEasing::from(&animation.easing);
    for index in 1..count - 1 {
        if span_kept && (times[index] - original[index]).abs() < 1e-9 {
            continue;
        }
        let progress = ((times[index] - start) / duration as f64).clamp(0.0, 1.0) as f32;
        edited.keyframes[index].offset = ease(curve, progress);
    }
    normalize_offsets(&mut edited.keyframes);
    edited
}

/// Each keyframe's time in the first iteration.
fn keyframe_times(animation: &NodeAnimation) -> Vec<f64> {
    animation
        .keyframes
        .iter()
        .map(|keyframe| keyframe_ms(animation, keyframe.offset))
        .collect()
}

/// First at 0, last at 1, ascending in between (a non-monotone curve can
/// map a later time to a smaller offset; the keyframe then stops at its
/// neighbour).
fn normalize_offsets(keyframes: &mut [Keyframe]) {
    let Some(last) = keyframes.len().checked_sub(1) else {
        return;
    };
    let mut previous = 0.0_f32;
    for (index, keyframe) in keyframes.iter_mut().enumerate() {
        keyframe.offset = if index == 0 {
            0.0
        } else if index == last {
            1.0
        } else {
            keyframe.offset.clamp(previous, 1.0)
        };
        previous = keyframe.offset;
    }
}

fn duration_between(start_ms: f64, end_ms: f64) -> u64 {
    (end_ms - start_ms).round().max(1.0) as u64
}

fn keyframe(offset: f32, values: BTreeMap<String, Value>) -> Keyframe {
    Keyframe {
        offset,
        values,
        extra: ExtraJson::default(),
    }
}

#[cfg(test)]
#[path = "motion_timeline_edit_tests.rs"]
mod tests;
