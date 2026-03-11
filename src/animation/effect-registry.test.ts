import { describe, it, expect } from 'vitest'
import './effects/index'
import {
  getAllEffects,
  getEffectsByCategory,
  generateClipFromEffect,
} from './effect-registry'

describe('effect-registry', () => {
  it('registers all 11 effects after importing effects/index', () => {
    const all = getAllEffects()
    expect(all).toHaveLength(11)
  })

  it('generateClipFromEffect returns valid keyframes for fade-in', () => {
    const result = generateClipFromEffect('fade-in', 500)
    expect(result).not.toBeNull()
    expect(result!.duration).toBe(500)
    expect(result!.keyframes).toHaveLength(2)
    expect(result!.keyframes[0].offset).toBe(0)
    expect(result!.keyframes[0].properties.opacity).toBe(0)
    expect(result!.keyframes[1].offset).toBe(1)
    expect(result!.keyframes[1].properties.opacity).toBe(1)
  })

  it('generateClipFromEffect for slide-in with direction right has correct x offset', () => {
    const result = generateClipFromEffect('slide-in', 300, { direction: 'right' }, { x: 100, y: 50 })
    expect(result).not.toBeNull()
    expect(result!.duration).toBe(300)
    expect(result!.keyframes).toHaveLength(2)
    // Start x should be currentState.x + 300 = 400
    expect(result!.keyframes[0].properties.x).toBe(400)
    // End x should be currentState.x = 100
    expect(result!.keyframes[1].properties.x).toBe(100)
  })

  it('getEffectsByCategory returns 5 enter effects', () => {
    const enters = getEffectsByCategory('enter')
    expect(enters).toHaveLength(5)
    for (const e of enters) {
      expect(e.category).toBe('enter')
    }
  })

  it('getEffectsByCategory returns 4 exit effects', () => {
    const exits = getEffectsByCategory('exit')
    expect(exits).toHaveLength(4)
    for (const e of exits) {
      expect(e.category).toBe('exit')
    }
  })

  it('generateClipFromEffect returns null for unknown effect', () => {
    const result = generateClipFromEffect('unknown-effect')
    expect(result).toBeNull()
  })
})
