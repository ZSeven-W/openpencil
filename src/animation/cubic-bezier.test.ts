import { describe, it, expect } from 'vitest'
import {
  createBezierEasing,
  resolveEasing,
  EASING_PRESETS,
} from './cubic-bezier'

describe('createBezierEasing', () => {
  it('linear (0,0,1,1) returns identity', () => {
    const linear = createBezierEasing(0, 0, 1, 1)
    expect(linear(0)).toBe(0)
    expect(linear(0.25)).toBeCloseTo(0.25, 5)
    expect(linear(0.5)).toBeCloseTo(0.5, 5)
    expect(linear(0.75)).toBeCloseTo(0.75, 5)
    expect(linear(1)).toBe(1)
  })

  it('easeInOut matches CSS reference values', () => {
    const easeInOut = createBezierEasing(0.42, 0, 0.58, 1)
    // CSS cubic-bezier(0.42, 0, 0.58, 1) reference values
    expect(easeInOut(0)).toBe(0)
    expect(easeInOut(1)).toBe(1)
    // At t=0.5 easeInOut should be approximately 0.5
    expect(easeInOut(0.5)).toBeCloseTo(0.5, 1)
    // Early values should be slower (below identity)
    expect(easeInOut(0.2)).toBeLessThan(0.2)
    // Late values should be faster (above identity)
    expect(easeInOut(0.8)).toBeGreaterThan(0.8)
  })

  it('boundary: easing(0) === 0 and easing(1) === 1 for all presets', () => {
    for (const [_name, points] of Object.entries(EASING_PRESETS)) {
      const fn = createBezierEasing(...points)
      expect(fn(0)).toBe(0)
      expect(fn(1)).toBe(1)
    }
  })

  it('bouncy preset handles y values > 1 (overshoot)', () => {
    const bouncy = createBezierEasing(...EASING_PRESETS.bouncy)
    expect(bouncy(0)).toBe(0)
    expect(bouncy(1)).toBe(1)
    // Bouncy [0.34, 1.56, 0.64, 1] should overshoot past 1.0 mid-curve
    let hasOvershoot = false
    for (let t = 0.1; t < 1; t += 0.01) {
      if (bouncy(t) > 1.0) {
        hasOvershoot = true
        break
      }
    }
    expect(hasOvershoot).toBe(true)
  })
})

describe('resolveEasing', () => {
  it('resolves named preset "ease" and caches it', () => {
    const fn1 = resolveEasing('ease')
    const fn2 = resolveEasing('ease')
    expect(fn1).toBe(fn2) // same reference = cached
    expect(typeof fn1).toBe('function')
    expect(fn1(0)).toBe(0)
    expect(fn1(1)).toBe(1)
  })

  it('resolves custom bezier tuple and caches it', () => {
    const fn1 = resolveEasing([0.42, 0, 0.58, 1])
    const fn2 = resolveEasing([0.42, 0, 0.58, 1])
    expect(fn1).toBe(fn2) // same reference = cached
    expect(fn1(0)).toBe(0)
    expect(fn1(1)).toBe(1)
  })

  it('falls back to linear for unknown preset name', () => {
    const fn = resolveEasing('nonexistent' as never)
    expect(fn(0.5)).toBe(0.5)
  })
})
