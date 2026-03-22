import { describe, it, expect } from 'vitest'
import { inferAspectRatio } from '../image-search-pipeline'
import type { PenNode } from '@/types/pen'

function makeImageNode(w: number, h: number): PenNode {
  return { id: 'test', type: 'image', src: '', width: w, height: h } as PenNode
}

describe('inferAspectRatio', () => {
  it('returns wide for landscape images', () => {
    expect(inferAspectRatio(makeImageNode(1200, 600))).toBe('wide')
  })

  it('returns tall for portrait images', () => {
    expect(inferAspectRatio(makeImageNode(400, 800))).toBe('tall')
  })

  it('returns square for roughly equal dimensions', () => {
    expect(inferAspectRatio(makeImageNode(500, 500))).toBe('square')
    expect(inferAspectRatio(makeImageNode(600, 500))).toBe('square')
  })

  it('returns undefined when dimensions missing', () => {
    expect(inferAspectRatio({ id: 'x', type: 'image', src: '' } as PenNode)).toBeUndefined()
  })
})
