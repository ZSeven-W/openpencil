import { describe, it, expect } from 'vitest'
import {
  lerp,
  getEasingFunction,
  interpolateProperties,
  getInterpolatedProperties,
} from './interpolation'
import type { AnimationTrack } from '@/types/animation'

describe('lerp', () => {
  it('interpolates between two numbers', () => {
    expect(lerp(0, 100, 0)).toBe(0)
    expect(lerp(0, 100, 0.5)).toBe(50)
    expect(lerp(0, 100, 1)).toBe(100)
  })

  it('handles negative values', () => {
    expect(lerp(-100, 100, 0.5)).toBe(0)
  })
})

describe('getEasingFunction', () => {
  it('returns a function for each preset', () => {
    const presets = ['smooth', 'snappy', 'bouncy', 'gentle', 'linear'] as const
    for (const preset of presets) {
      const fn = getEasingFunction(preset)
      expect(typeof fn).toBe('function')
      // All easing functions should return 0 at t=0 and 1 at t=1
      expect(fn(0)).toBe(0)
      expect(fn(1)).toBeCloseTo(1, 4)
    }
  })

  it('linear returns t unchanged', () => {
    const fn = getEasingFunction('linear')
    expect(fn(0.25)).toBe(0.25)
    expect(fn(0.5)).toBe(0.5)
    expect(fn(0.75)).toBe(0.75)
  })
})

describe('interpolateProperties', () => {
  it('interpolates matching properties', () => {
    const result = interpolateProperties(
      { x: 0, y: 0, opacity: 0 },
      { x: 100, y: 200, opacity: 1 },
      0.5,
    )
    expect(result.x).toBe(50)
    expect(result.y).toBe(100)
    expect(result.opacity).toBe(0.5)
  })

  it('handles partial properties', () => {
    const result = interpolateProperties(
      { x: 0 },
      { x: 100, y: 200 },
      0.5,
    )
    expect(result.x).toBe(50)
    expect(result.y).toBe(200) // only in 'to', so use 'to' value
  })
})

describe('getInterpolatedProperties', () => {
  const makeTrack = (keyframes: AnimationTrack['keyframes']): AnimationTrack => ({
    nodeId: 'test',
    keyframes,
    phases: { in: { start: 0, duration: 500 }, while: { start: 500, duration: 4000 }, out: { start: 4500, duration: 500 } },
    startDelay: 0,
  })

  it('returns null for empty tracks', () => {
    const track = makeTrack([])
    expect(getInterpolatedProperties(track, 250)).toBeNull()
  })

  it('returns first keyframe properties before first keyframe time', () => {
    const track = makeTrack([
      { id: '1', time: 100, properties: { x: 50 }, easing: 'linear' },
      { id: '2', time: 500, properties: { x: 200 }, easing: 'linear' },
    ])
    const result = getInterpolatedProperties(track, 0)
    expect(result?.x).toBe(50)
  })

  it('returns last keyframe properties after last keyframe time', () => {
    const track = makeTrack([
      { id: '1', time: 0, properties: { x: 0 }, easing: 'linear' },
      { id: '2', time: 500, properties: { x: 100 }, easing: 'linear' },
    ])
    const result = getInterpolatedProperties(track, 1000)
    expect(result?.x).toBe(100)
  })

  it('interpolates between keyframes with linear easing', () => {
    const track = makeTrack([
      { id: '1', time: 0, properties: { x: 0 }, easing: 'linear' },
      { id: '2', time: 1000, properties: { x: 100 }, easing: 'linear' },
    ])
    const result = getInterpolatedProperties(track, 500)
    expect(result?.x).toBe(50)
  })

  it('respects startDelay', () => {
    const track: AnimationTrack = {
      ...makeTrack([
        { id: '1', time: 0, properties: { x: 0 }, easing: 'linear' },
        { id: '2', time: 1000, properties: { x: 100 }, easing: 'linear' },
      ]),
      startDelay: 500,
    }
    // At time 500, track time is 0 → should return first keyframe
    const result = getInterpolatedProperties(track, 500)
    expect(result?.x).toBe(0)

    // Before startDelay → null
    expect(getInterpolatedProperties(track, 200)).toBeNull()
  })

  it('handles single keyframe', () => {
    const track = makeTrack([
      { id: '1', time: 500, properties: { opacity: 0.5 }, easing: 'linear' },
    ])
    const result = getInterpolatedProperties(track, 500)
    expect(result?.opacity).toBe(0.5)
  })
})
