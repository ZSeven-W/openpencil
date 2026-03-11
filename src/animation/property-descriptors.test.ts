import { describe, it, expect } from 'vitest'
import {
  registerPropertyDescriptor,
  getPropertyDescriptor,
  getAllPropertyDescriptors,
  lerp,
  srgbLerp,
} from './property-descriptors'

describe('property-descriptors', () => {
  describe('registerPropertyDescriptor + getPropertyDescriptor', () => {
    it('should register and retrieve a property descriptor', () => {
      const desc = getPropertyDescriptor('x')
      expect(desc).toBeDefined()
      expect(desc!.key).toBe('x')
      expect(desc!.type).toBe('number')
    })

    it('should return undefined for unregistered keys', () => {
      expect(getPropertyDescriptor('nonexistent.property')).toBeUndefined()
    })

    it('should overwrite a descriptor when re-registered with same key', () => {
      registerPropertyDescriptor({
        key: '__test_overwrite',
        type: 'number',
        default: 0,
        interpolate: lerp,
      })
      registerPropertyDescriptor({
        key: '__test_overwrite',
        type: 'number',
        default: 99,
        interpolate: lerp,
      })
      const desc = getPropertyDescriptor('__test_overwrite')
      expect(desc!.default).toBe(99)
    })
  })

  describe('lerp', () => {
    it('should interpolate linearly at midpoint', () => {
      expect(lerp(0, 100, 0.5)).toBe(50)
    })

    it('should return from value at t=0', () => {
      expect(lerp(10, 90, 0)).toBe(10)
    })

    it('should return to value at t=1', () => {
      expect(lerp(10, 90, 1)).toBe(90)
    })

    it('should handle negative values', () => {
      expect(lerp(-50, 50, 0.5)).toBe(0)
    })
  })

  describe('srgbLerp', () => {
    it('should interpolate black to white at midpoint', () => {
      const result = srgbLerp('#000000', '#ffffff', 0.5)
      // 128 rounds to 0x80
      expect(result).toBe('#808080')
    })

    it('should return from color at t=0', () => {
      expect(srgbLerp('#ff0000', '#0000ff', 0)).toBe('#ff0000')
    })

    it('should return to color at t=1', () => {
      expect(srgbLerp('#ff0000', '#0000ff', 1)).toBe('#0000ff')
    })

    it('should handle shorthand hex codes', () => {
      const result = srgbLerp('#000', '#fff', 0.5)
      expect(result).toBe('#808080')
    })

    it('should fall back gracefully for invalid hex', () => {
      expect(srgbLerp('invalid', '#ffffff', 0.3)).toBe('invalid')
      expect(srgbLerp('invalid', '#ffffff', 0.7)).toBe('#ffffff')
    })
  })

  describe('all 22 properties registered', () => {
    it('should have exactly 22 registered properties', () => {
      const all = getAllPropertyDescriptors()
      // Filter out any test descriptors we added
      const coreDescriptors = all.filter(
        (d) => !d.key.startsWith('__test'),
      )
      expect(coreDescriptors.length).toBe(22)
    })

    it('should include all expected property keys', () => {
      const expectedKeys = [
        'x',
        'y',
        'scaleX',
        'scaleY',
        'rotation',
        'opacity',
        'fill.color',
        'stroke.color',
        'strokeWidth',
        'cornerRadius',
        'blur',
        'shadow.offsetX',
        'shadow.offsetY',
        'shadow.blur',
        'shadow.color',
        'fontSize',
        'letterSpacing',
        'lineHeight',
        'text.fill.color',
        'sourceStart',
        'sourceEnd',
        'playbackRate',
      ]

      for (const key of expectedKeys) {
        expect(
          getPropertyDescriptor(key),
          `Expected property "${key}" to be registered`,
        ).toBeDefined()
      }
    })
  })

  describe('nodeTypes filtering', () => {
    it('should restrict text properties to text nodes', () => {
      const textProps = ['fontSize', 'letterSpacing', 'lineHeight', 'text.fill.color']
      for (const key of textProps) {
        const desc = getPropertyDescriptor(key)
        expect(desc).toBeDefined()
        expect(desc!.nodeTypes).toContain('text')
      }
    })

    it('should not restrict transform properties to specific node types', () => {
      const transformProps = ['x', 'y', 'scaleX', 'scaleY', 'rotation', 'opacity']
      for (const key of transformProps) {
        const desc = getPropertyDescriptor(key)
        expect(desc).toBeDefined()
        expect(desc!.nodeTypes).toBeUndefined()
      }
    })

    it('should not restrict visual properties to specific node types', () => {
      const visualProps = ['fill.color', 'stroke.color', 'strokeWidth', 'cornerRadius', 'blur']
      for (const key of visualProps) {
        const desc = getPropertyDescriptor(key)
        expect(desc).toBeDefined()
        expect(desc!.nodeTypes).toBeUndefined()
      }
    })
  })

  describe('interpolation via descriptors', () => {
    it('should use lerp for number properties', () => {
      const desc = getPropertyDescriptor('opacity')!
      const result = desc.interpolate(0, 1, 0.5)
      expect(result).toBe(0.5)
    })

    it('should use srgbLerp for color properties', () => {
      const desc = getPropertyDescriptor('fill.color')!
      const result = desc.interpolate('#000000', '#ffffff', 0.5)
      expect(result).toBe('#808080')
    })
  })
})
