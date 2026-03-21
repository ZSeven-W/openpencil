import { describe, it, expect } from 'vitest'
import type { PenNode } from '@zseven-w/pen-types'
import type { VariableDefinition } from '@zseven-w/pen-types'
import {
  isVariableRef,
  getDefaultTheme,
  resolveVariableRef,
  resolveColorRef,
  resolveNumericRef,
  resolveNodeForCanvas,
} from '../variables/resolve'

const vars: Record<string, VariableDefinition> = {
  'primary': { type: 'color', value: '#3b82f6' },
  'spacing': { type: 'number', value: 16 },
  'themed-color': {
    type: 'color',
    value: [
      { value: '#ffffff', theme: { 'Theme-1': 'Light' } },
      { value: '#1a1a1a', theme: { 'Theme-1': 'Dark' } },
    ],
  },
}

describe('variables/resolve', () => {
  describe('isVariableRef', () => {
    it('returns true for $ prefixed strings', () => {
      expect(isVariableRef('$primary')).toBe(true)
    })

    it('returns false for regular strings', () => {
      expect(isVariableRef('#ff0000')).toBe(false)
    })

    it('returns false for non-strings', () => {
      expect(isVariableRef(42)).toBe(false)
      expect(isVariableRef(undefined)).toBe(false)
    })
  })

  describe('getDefaultTheme', () => {
    it('returns first value of each theme axis', () => {
      const themes = { 'Theme-1': ['Light', 'Dark'], 'Size': ['Compact', 'Regular'] }
      expect(getDefaultTheme(themes)).toEqual({
        'Theme-1': 'Light',
        'Size': 'Compact',
      })
    })

    it('returns empty for undefined themes', () => {
      expect(getDefaultTheme(undefined)).toEqual({})
    })
  })

  describe('resolveVariableRef', () => {
    it('resolves a simple color variable', () => {
      expect(resolveVariableRef('$primary', vars)).toBe('#3b82f6')
    })

    it('resolves a number variable', () => {
      expect(resolveVariableRef('$spacing', vars)).toBe(16)
    })

    it('returns undefined for missing variable', () => {
      expect(resolveVariableRef('$missing', vars)).toBeUndefined()
    })

    it('resolves themed values with matching theme', () => {
      expect(resolveVariableRef('$themed-color', vars, { 'Theme-1': 'Dark' })).toBe('#1a1a1a')
    })

    it('falls back to first value when no theme match', () => {
      expect(resolveVariableRef('$themed-color', vars)).toBe('#ffffff')
    })

    it('returns undefined for non-ref strings', () => {
      expect(resolveVariableRef('#ff0000', vars)).toBeUndefined()
    })
  })

  describe('resolveColorRef', () => {
    it('returns the color when not a ref', () => {
      expect(resolveColorRef('#ff0000', vars)).toBe('#ff0000')
    })

    it('resolves a color ref', () => {
      expect(resolveColorRef('$primary', vars)).toBe('#3b82f6')
    })

    it('returns undefined for undefined input', () => {
      expect(resolveColorRef(undefined, vars)).toBeUndefined()
    })
  })

  describe('resolveNumericRef', () => {
    it('returns the number when not a ref', () => {
      expect(resolveNumericRef(42, vars)).toBe(42)
    })

    it('resolves a numeric ref', () => {
      expect(resolveNumericRef('$spacing', vars)).toBe(16)
    })

    it('returns undefined for non-numeric results', () => {
      expect(resolveNumericRef('$primary', vars)).toBeUndefined()
    })
  })

  describe('resolveNodeForCanvas', () => {
    it('returns same node when no variables', () => {
      const node: PenNode = { id: '1', type: 'rectangle', x: 0, y: 0 }
      expect(resolveNodeForCanvas(node, {})).toBe(node)
    })

    it('resolves $variable opacity', () => {
      const node: PenNode = { id: '1', type: 'rectangle', x: 0, y: 0, opacity: '$spacing' }
      const resolved = resolveNodeForCanvas(node, vars)
      expect(resolved.opacity).toBe(16)
      expect(resolved).not.toBe(node)
    })

    it('resolves fill colors', () => {
      const node: PenNode = {
        id: '1', type: 'rectangle', x: 0, y: 0,
        fill: [{ type: 'solid', color: '$primary' }],
      }
      const resolved = resolveNodeForCanvas(node, vars)
      const fill = (resolved as { fill: Array<{ color: string }> }).fill
      expect(fill[0].color).toBe('#3b82f6')
    })
  })
})
