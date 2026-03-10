import { describe, it, expect } from 'vitest'
import { generatePresetKeyframes } from './presets'
import type { AnimatableProperties } from '@/types/animation'

const defaultState: AnimatableProperties = {
  x: 540,
  y: 960,
  scaleX: 1,
  scaleY: 1,
  rotation: 0,
  opacity: 1,
}

const totalDuration = 5000

describe('generatePresetKeyframes', () => {
  it('generates fade preset with in/while/out phases', () => {
    const result = generatePresetKeyframes('fade', defaultState, totalDuration)

    expect(result.phases.in.start).toBe(0)
    expect(result.phases.in.duration).toBeGreaterThan(0)
    expect(result.phases.while.duration).toBeGreaterThan(0)
    expect(result.phases.out.duration).toBeGreaterThan(0)

    // Phase durations should sum to total
    const totalPhases =
      result.phases.in.duration +
      result.phases.while.duration +
      result.phases.out.duration
    expect(totalPhases).toBeCloseTo(totalDuration, 0)

    // Should have keyframes
    expect(result.keyframes.length).toBeGreaterThan(0)

    // First keyframe should have opacity 0 (fade in starts invisible)
    expect(result.keyframes[0].properties.opacity).toBe(0)

    // Last keyframe should have opacity 0 (fade out ends invisible)
    const lastKf = result.keyframes[result.keyframes.length - 1]
    expect(lastKf.properties.opacity).toBe(0)
  })

  it('generates slide preset with direction', () => {
    const result = generatePresetKeyframes('slide', defaultState, totalDuration, {
      direction: 'left',
    })

    expect(result.keyframes.length).toBeGreaterThan(0)

    // First keyframe should have x offset to the left (negative of current position)
    const firstKf = result.keyframes[0]
    expect(firstKf.properties.x).toBeDefined()
    expect(firstKf.properties.x!).toBeLessThan(defaultState.x)
  })

  it('generates slide preset from right', () => {
    const result = generatePresetKeyframes('slide', defaultState, totalDuration, {
      direction: 'right',
    })

    const firstKf = result.keyframes[0]
    expect(firstKf.properties.x!).toBeGreaterThan(defaultState.x)
  })

  it('generates scale preset', () => {
    const result = generatePresetKeyframes('scale', defaultState, totalDuration)

    // First keyframe: scaled to 0
    expect(result.keyframes[0].properties.scaleX).toBe(0)
    expect(result.keyframes[0].properties.scaleY).toBe(0)

    // Last keyframe: scaled to 0 (exit)
    const lastKf = result.keyframes[result.keyframes.length - 1]
    expect(lastKf.properties.scaleX).toBe(0)
    expect(lastKf.properties.scaleY).toBe(0)
  })

  it('generates bounce preset', () => {
    const result = generatePresetKeyframes('bounce', defaultState, totalDuration)

    expect(result.keyframes.length).toBeGreaterThan(0)

    // Bounce in should overshoot (scale > 1 at some point)
    const hasOvershoot = result.keyframes.some(
      (kf) => kf.properties.scaleX !== undefined && kf.properties.scaleX > 1,
    )
    expect(hasOvershoot).toBe(true)
  })

  it('respects easing config', () => {
    const result = generatePresetKeyframes('fade', defaultState, totalDuration, {
      easing: 'bouncy',
    })

    // First keyframe should use the specified easing
    expect(result.keyframes[0].easing).toBe('bouncy')
  })

  it('keyframes are sorted by time', () => {
    const presets = ['fade', 'slide', 'scale', 'bounce'] as const
    for (const preset of presets) {
      const result = generatePresetKeyframes(preset, defaultState, totalDuration)
      for (let i = 1; i < result.keyframes.length; i++) {
        expect(result.keyframes[i].time).toBeGreaterThanOrEqual(
          result.keyframes[i - 1].time,
        )
      }
    }
  })
})
