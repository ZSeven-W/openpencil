import { describe, it, expect } from 'vitest'
import { interpolateClip, createTrackBuffer } from './interpolation'
import type { AnimationClipData } from '@/types/animation'

function makeClip(overrides: Partial<AnimationClipData> = {}): AnimationClipData {
  return {
    id: 'clip-1',
    kind: 'animation',
    startTime: 0,
    duration: 1000,
    keyframes: [
      { id: 'kf-1', offset: 0, properties: { x: 0, y: 0, opacity: 0 }, easing: 'linear' },
      { id: 'kf-2', offset: 1, properties: { x: 100, y: 200, opacity: 1 }, easing: 'linear' },
    ],
    ...overrides,
  }
}

describe('interpolateClip', () => {
  it('returns null before clip start', () => {
    const clip = makeClip({ startTime: 500 })
    expect(interpolateClip(clip, 0)).toBeNull()
  })

  it('returns null after clip end (no extrapolate)', () => {
    const clip = makeClip({ startTime: 0, duration: 1000 })
    expect(interpolateClip(clip, 1500)).toBeNull()
  })

  it('returns first keyframe at start', () => {
    const clip = makeClip()
    const result = interpolateClip(clip, 0)
    expect(result?.x).toBe(0)
    expect(result?.opacity).toBe(0)
  })

  it('returns last keyframe at end', () => {
    const clip = makeClip()
    const result = interpolateClip(clip, 1000)
    expect(result?.x).toBe(100)
    expect(result?.opacity).toBe(1)
  })

  it('interpolates at midpoint with linear easing', () => {
    const clip = makeClip()
    const result = interpolateClip(clip, 500)
    expect(result?.x).toBe(50)
    expect(result?.y).toBe(100)
    expect(result?.opacity).toBe(0.5)
  })

  it('returns null for empty keyframes', () => {
    const clip = makeClip({ keyframes: [] })
    expect(interpolateClip(clip, 500)).toBeNull()
  })

  it('holds first value when extrapolate is hold and before start', () => {
    const clip = makeClip({ startTime: 500, extrapolate: 'hold' })
    const result = interpolateClip(clip, 0)
    expect(result?.x).toBe(0)
  })

  it('holds last value when extrapolate is hold and after end', () => {
    const clip = makeClip({ extrapolate: 'hold' })
    const result = interpolateClip(clip, 2000)
    expect(result?.x).toBe(100)
  })

  it('uses track buffer for reuse', () => {
    const clip = makeClip()
    const buffer = createTrackBuffer()
    const result = interpolateClip(clip, 500, buffer)
    expect(result).toBe(buffer.values)
    expect(buffer.prevKeyframeIdx).toBe(0)
  })

  it('handles single keyframe', () => {
    const clip = makeClip({
      keyframes: [
        { id: 'kf-1', offset: 0.5, properties: { opacity: 0.5 }, easing: 'linear' },
      ],
    })
    const result = interpolateClip(clip, 500)
    expect(result?.opacity).toBe(0.5)
  })
})
