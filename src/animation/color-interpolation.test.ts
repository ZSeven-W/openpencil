import { describe, it, expect } from 'vitest'
import { parseHex, formatHex, srgbLerp } from './color-interpolation'

describe('parseHex', () => {
  it('parses 6-digit hex', () => {
    expect(parseHex('#ff0000')).toEqual([255, 0, 0])
    expect(parseHex('#00ff00')).toEqual([0, 255, 0])
    expect(parseHex('#0000ff')).toEqual([0, 0, 255])
    expect(parseHex('#808080')).toEqual([128, 128, 128])
  })

  it('parses 3-digit shorthand hex', () => {
    expect(parseHex('#f00')).toEqual([255, 0, 0])
    expect(parseHex('#0f0')).toEqual([0, 255, 0])
    expect(parseHex('#00f')).toEqual([0, 0, 255])
  })

  it('returns null for invalid hex', () => {
    expect(parseHex('red')).toBeNull()
    expect(parseHex('#gg0000')).toBeNull()
    expect(parseHex('')).toBeNull()
    expect(parseHex('#12345')).toBeNull()
  })
})

describe('formatHex', () => {
  it('formats RGB to hex string', () => {
    expect(formatHex(128, 128, 128)).toBe('#808080')
    expect(formatHex(255, 0, 0)).toBe('#ff0000')
    expect(formatHex(0, 255, 0)).toBe('#00ff00')
    expect(formatHex(0, 0, 255)).toBe('#0000ff')
  })

  it('clamps values to 0-255', () => {
    expect(formatHex(300, -10, 128)).toBe('#ff0080')
  })
})

describe('srgbLerp', () => {
  it('interpolates black to white at 0.5 to ~#808080', () => {
    const result = srgbLerp('#000000', '#ffffff', 0.5)
    // Allow ±1 for rounding: #7f7f7f or #808080
    const parsed = parseHex(result)
    expect(parsed).not.toBeNull()
    expect(parsed![0]).toBeGreaterThanOrEqual(127)
    expect(parsed![0]).toBeLessThanOrEqual(128)
    expect(parsed![1]).toBeGreaterThanOrEqual(127)
    expect(parsed![1]).toBeLessThanOrEqual(128)
    expect(parsed![2]).toBeGreaterThanOrEqual(127)
    expect(parsed![2]).toBeLessThanOrEqual(128)
  })

  it('returns start color at t=0', () => {
    expect(srgbLerp('#ff0000', '#0000ff', 0)).toBe('#ff0000')
  })

  it('returns end color at t=1', () => {
    expect(srgbLerp('#ff0000', '#0000ff', 1)).toBe('#0000ff')
  })

  it('returns nearest value for invalid hex input', () => {
    expect(srgbLerp('invalid', '#ffffff', 0.3)).toBe('invalid')
    expect(srgbLerp('invalid', '#ffffff', 0.7)).toBe('#ffffff')
    expect(srgbLerp('#ff0000', 'invalid', 0.3)).toBe('#ff0000')
    expect(srgbLerp('#ff0000', 'invalid', 0.7)).toBe('invalid')
  })
})
