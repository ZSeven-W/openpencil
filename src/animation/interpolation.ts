import type {
  AnimatableProperties,
  AnimationTrack,
  EasingPreset,
} from '@/types/animation'

// --- Easing Functions ---
// Each takes t in [0,1] and returns eased t in [0,1]

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

export function getEasingFunction(preset: EasingPreset): (t: number) => number {
  return easingFunctions[preset]
}

// --- Interpolation ---

export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t
}

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
