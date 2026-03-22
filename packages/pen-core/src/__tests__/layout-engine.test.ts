import { describe, it, expect } from 'vitest'
import type { PenNode } from '@zseven-w/pen-types'
import {
  resolvePadding,
  isNodeVisible,
  inferLayout,
  getNodeWidth,
  getNodeHeight,
  computeLayoutPositions,
} from '../layout/engine'

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode => ({
  id: 'f1', type: 'frame', x: 0, y: 0, ...props,
} as PenNode)

const rect = (id: string, w = 50, h = 30): PenNode => ({
  id, type: 'rectangle', x: 0, y: 0, width: w, height: h,
})

describe('layout-engine', () => {
  describe('resolvePadding', () => {
    it('returns zero for undefined', () => {
      expect(resolvePadding(undefined)).toEqual({ top: 0, right: 0, bottom: 0, left: 0 })
    })

    it('resolves uniform padding', () => {
      expect(resolvePadding(10)).toEqual({ top: 10, right: 10, bottom: 10, left: 10 })
    })

    it('resolves [vertical, horizontal]', () => {
      expect(resolvePadding([10, 20])).toEqual({ top: 10, right: 20, bottom: 10, left: 20 })
    })

    it('resolves [top, right, bottom, left]', () => {
      expect(resolvePadding([1, 2, 3, 4])).toEqual({ top: 1, right: 2, bottom: 3, left: 4 })
    })

    it('returns zero for string (variable ref)', () => {
      expect(resolvePadding('$spacing')).toEqual({ top: 0, right: 0, bottom: 0, left: 0 })
    })
  })

  describe('isNodeVisible', () => {
    it('returns true by default', () => {
      expect(isNodeVisible(rect('a'))).toBe(true)
    })

    it('returns false when visible is false', () => {
      expect(isNodeVisible({ ...rect('a'), visible: false })).toBe(false)
    })
  })

  describe('inferLayout', () => {
    it('returns undefined for non-frame nodes', () => {
      expect(inferLayout(rect('a'))).toBeUndefined()
    })

    it('infers horizontal when gap is set', () => {
      expect(inferLayout(frame({ gap: 10, children: [] }))).toBe('horizontal')
    })

    it('infers horizontal when padding is set', () => {
      expect(inferLayout(frame({ padding: 10, children: [] }))).toBe('horizontal')
    })

    it('returns undefined when no layout hints', () => {
      expect(inferLayout(frame({ children: [rect('a')] }))).toBeUndefined()
    })
  })

  describe('getNodeWidth / getNodeHeight', () => {
    it('returns explicit width', () => {
      expect(getNodeWidth(rect('a', 200))).toBe(200)
    })

    it('returns explicit height', () => {
      expect(getNodeHeight(rect('a', 50, 100))).toBe(100)
    })

    it('estimates text width', () => {
      const text: PenNode = { id: 't', type: 'text', content: 'Hello World', fontSize: 16 }
      expect(getNodeWidth(text)).toBeGreaterThan(0)
    })
  })

  describe('computeLayoutPositions', () => {
    it('positions children horizontally', () => {
      const parent = frame({
        width: 300, height: 100,
        layout: 'horizontal', gap: 10,
        children: [rect('a', 50, 30), rect('b', 50, 30)],
      })
      const result = computeLayoutPositions(parent, (parent as PenNode & { children: PenNode[] }).children)
      expect(result[0].x).toBe(0)
      expect(result[0].y).toBe(0)
      expect(result[1].x).toBe(60) // 50 + 10 gap
    })

    it('positions children vertically', () => {
      const parent = frame({
        width: 100, height: 300,
        layout: 'vertical', gap: 10,
        children: [rect('a', 50, 30), rect('b', 50, 30)],
      })
      const result = computeLayoutPositions(parent, (parent as PenNode & { children: PenNode[] }).children)
      expect(result[0].x).toBe(0)
      expect(result[0].y).toBe(0)
      expect(result[1].y).toBe(40) // 30 + 10 gap
    })

    it('applies padding', () => {
      const parent = frame({
        width: 300, height: 100,
        layout: 'horizontal', padding: 20,
        children: [rect('a', 50, 30)],
      })
      const result = computeLayoutPositions(parent, (parent as PenNode & { children: PenNode[] }).children)
      expect(result[0].x).toBe(20)
      expect(result[0].y).toBe(20)
    })

    it('centers children on cross axis', () => {
      const parent = frame({
        width: 300, height: 100,
        layout: 'horizontal', alignItems: 'center',
        children: [rect('a', 50, 30)],
      })
      const result = computeLayoutPositions(parent, (parent as PenNode & { children: PenNode[] }).children)
      expect(result[0].y).toBe(35) // (100 - 30) / 2
    })

    it('filters invisible children', () => {
      const parent = frame({
        width: 300, height: 100,
        layout: 'horizontal',
        children: [rect('a', 50, 30), { ...rect('b', 50, 30), visible: false }],
      })
      const result = computeLayoutPositions(parent, (parent as PenNode & { children: PenNode[] }).children)
      expect(result).toHaveLength(1)
    })

    it('returns visible children as-is when layout is none', () => {
      const parent = frame({
        width: 300, height: 100,
        layout: 'none',
        children: [rect('a', 50, 30)],
      })
      const result = computeLayoutPositions(parent, (parent as PenNode & { children: PenNode[] }).children)
      expect(result).toHaveLength(1)
      expect(result[0].id).toBe('a')
    })
  })
})
