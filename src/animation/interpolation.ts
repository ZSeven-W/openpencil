import type {
  AnimatableValue,
  AnimationClipData,
} from '@/types/animation'
import { resolveEasing } from './cubic-bezier'
import { getPropertyDescriptor } from './property-descriptors'

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
    for (const key in lower.properties) {
      result[key] = lower.properties[key]
    }
    return result
  }

  // Interpolate between keyframes
  const segmentT = (offset - lower.offset) / (upper.offset - lower.offset)
  const easingFn = resolveEasing(lower.easing)
  const easedT = easingFn(Math.max(0, Math.min(1, segmentT)))

  const result = buffer?.values ?? {}

  // Interpolate each property — iterate both keyframes directly to avoid Set allocation.
  // Lower properties first, then upper properties for any keys not already visited.
  for (const key in lower.properties) {
    const fromVal = lower.properties[key]
    const toVal = upper.properties[key]

    if (toVal === undefined) {
      result[key] = fromVal
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
  for (const key in upper.properties) {
    if (key in lower.properties) continue
    result[key] = upper.properties[key]
  }

  return result
}
