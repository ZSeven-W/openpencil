import type {
  AnimatableProperties,
  AnimatableValue,
  AnimationClipData,
  AnimationTrack,
  EasingPreset,
} from '@/types/animation'
import { resolveEasing } from './cubic-bezier'
import { getPropertyDescriptor } from './property-descriptors'

// ============================================================
// v1 Interpolation (deprecated — kept for Phase 1 compatibility)
// ============================================================

// --- Easing Functions ---

function easeOut(t: number): number {
  return 1 - (1 - t) ** 3
}

function easeIn(t: number): number {
  return t ** 3
}

function easeInOut(t: number): number {
  return t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2
}

function bounce(t: number): number {
  const n1 = 7.5625
  const d1 = 2.75
  if (t < 1 / d1) return n1 * t * t
  if (t < 2 / d1) return n1 * (t -= 1.5 / d1) * t + 0.75
  if (t < 2.5 / d1) return n1 * (t -= 2.25 / d1) * t + 0.9375
  return n1 * (t -= 2.625 / d1) * t + 0.984375
}

const easingFunctions: Record<EasingPreset, (t: number) => number> = {
  smooth: easeOut,
  snappy: easeInOut,
  bouncy: bounce,
  gentle: easeIn,
  linear: (t: number) => t,
}

/** @deprecated Use resolveEasing from cubic-bezier.ts */
export function getEasingFunction(
  preset: EasingPreset,
): (t: number) => number {
  return easingFunctions[preset]
}

// --- Interpolation ---

/** @deprecated Use v2 interpolateClip */
export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t
}

/** @deprecated Use v2 interpolateClip */
export function interpolateProperties(
  from: Partial<AnimatableProperties>,
  to: Partial<AnimatableProperties>,
  t: number,
): Partial<AnimatableProperties> {
  const result: Partial<AnimatableProperties> = {}
  const keys = new Set([
    ...Object.keys(from),
    ...Object.keys(to),
  ]) as Set<keyof AnimatableProperties>

  for (const key of keys) {
    const a = from[key]
    const b = to[key]
    if (a !== undefined && b !== undefined) {
      result[key] = lerp(a, b, t)
    } else if (b !== undefined) {
      result[key] = b
    } else if (a !== undefined) {
      result[key] = a
    }
  }
  return result
}

// --- Track Interpolation ---

/**
 * Find the two keyframes surrounding a given time and interpolate between them.
 * Keyframes must be sorted by time (store invariant).
 * @deprecated Use v2 interpolateClip
 */
export function getInterpolatedProperties(
  track: AnimationTrack,
  time: number,
): Partial<AnimatableProperties> | null {
  const { keyframes } = track
  if (keyframes.length === 0) return null

  // Adjust time relative to track start
  const trackTime = time - track.startDelay
  if (trackTime < 0) return null

  // Before first keyframe — return first keyframe's properties
  if (trackTime <= keyframes[0].time) {
    return keyframes[0].properties
  }

  // After last keyframe — return last keyframe's properties
  const last = keyframes[keyframes.length - 1]
  if (trackTime >= last.time) {
    return last.properties
  }

  // Find surrounding keyframes (binary search)
  let lo = 0
  let hi = keyframes.length - 1
  while (lo < hi - 1) {
    const mid = (lo + hi) >> 1
    if (keyframes[mid].time <= trackTime) lo = mid
    else hi = mid
  }

  const from = keyframes[lo]
  const to = keyframes[hi]
  const segmentDuration = to.time - from.time
  if (segmentDuration === 0) return to.properties

  const rawT = (trackTime - from.time) / segmentDuration
  const easedT = getEasingFunction(to.easing)(rawT)

  return interpolateProperties(from.properties, to.properties, easedT)
}

// ============================================================
// v2 Interpolation Engine
// ============================================================

export interface TrackBuffer {
  values: Record<string, AnimatableValue>
  prevKeyframeIdx: number
}

export function createTrackBuffer(): TrackBuffer {
  return { values: {}, prevKeyframeIdx: 0 }
}

/**
 * Interpolate a clip's animated values at a given time.
 * Returns null if time is outside clip bounds (unless extrapolate === 'hold').
 */
export function interpolateClip(
  clip: AnimationClipData,
  timeMs: number,
  buffer?: TrackBuffer,
): Record<string, AnimatableValue> | null {
  const { startTime, duration, keyframes } = clip
  if (keyframes.length === 0) return null

  // Calculate local time as ms offset within clip
  const localTime = timeMs - startTime
  if (localTime < 0) {
    if (clip.extrapolate === 'hold') {
      return { ...keyframes[0].properties }
    }
    return null
  }
  if (localTime > duration) {
    if (clip.extrapolate === 'hold') {
      return { ...keyframes[keyframes.length - 1].properties }
    }
    return null
  }

  const offset = duration > 0 ? localTime / duration : 0

  // Binary search for the upper keyframe
  let lo = 0
  let hi = keyframes.length - 1
  while (lo < hi) {
    const mid = (lo + hi) >> 1
    if (keyframes[mid].offset < offset) lo = mid + 1
    else hi = mid
  }

  const upperIdx = lo
  const lowerIdx = Math.max(0, upperIdx - 1)

  if (buffer) buffer.prevKeyframeIdx = lowerIdx

  const lower = keyframes[lowerIdx]
  const upper = keyframes[upperIdx]

  // Same keyframe or at exact keyframe position
  if (lowerIdx === upperIdx || lower.offset === upper.offset) {
    const result = buffer?.values ?? {}
    for (const [key, val] of Object.entries(lower.properties)) {
      result[key] = val
    }
    return result
  }

  // Interpolate between keyframes
  const segmentT = (offset - lower.offset) / (upper.offset - lower.offset)
  const easingFn = resolveEasing(lower.easing)
  const easedT = easingFn(Math.max(0, Math.min(1, segmentT)))

  const result = buffer?.values ?? {}

  // Interpolate each property
  const allKeys = new Set([
    ...Object.keys(lower.properties),
    ...Object.keys(upper.properties),
  ])
  for (const key of allKeys) {
    const fromVal = lower.properties[key]
    const toVal = upper.properties[key]

    if (fromVal === undefined || toVal === undefined) {
      result[key] = (fromVal ?? toVal)!
      continue
    }

    const desc = getPropertyDescriptor(key)
    if (desc) {
      result[key] = desc.interpolate(fromVal, toVal, easedT) as AnimatableValue
    } else if (typeof fromVal === 'number' && typeof toVal === 'number') {
      result[key] = fromVal + (toVal - fromVal) * easedT
    } else {
      result[key] = easedT < 0.5 ? fromVal : toVal
    }
  }

  return result
}
