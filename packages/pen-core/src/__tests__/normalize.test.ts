import { describe, it, expect } from 'vitest'
import type { PenDocument } from '@zseven-w/pen-types'
import { normalizePenDocument } from '../normalize'

describe('normalizePenDocument', () => {
  it('normalizes "color" fill type to "solid"', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{
        id: '1', type: 'rectangle', x: 0, y: 0,
        fill: [{ type: 'color' as 'solid', color: '#ff0000' }],
      }],
    }
    const result = normalizePenDocument(doc)
    const fill = (result.children[0] as { fill: Array<{ type: string }> }).fill
    expect(fill[0].type).toBe('solid')
  })

  it('normalizes string fill shorthand to solid fill array', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{
        id: '1', type: 'rectangle', x: 0, y: 0,
        fill: '#ff0000' as unknown as Array<{ type: 'solid'; color: string }>,
      }],
    }
    const result = normalizePenDocument(doc)
    const fill = (result.children[0] as { fill: Array<{ type: string; color: string }> }).fill
    expect(fill).toHaveLength(1)
    expect(fill[0]).toEqual({ type: 'solid', color: '#ff0000' })
  })

  it('normalizes fill_container sizing', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{
        id: '1', type: 'frame', x: 0, y: 0,
        width: 'fill_container(300)' as unknown as number,
        height: 'fill_container' as unknown as number,
        children: [],
      }],
    }
    const result = normalizePenDocument(doc)
    const node = result.children[0] as { width: unknown; height: unknown }
    expect(node.width).toBe('fill_container')
    expect(node.height).toBe('fill_container')
  })

  it('normalizes fit_content with hint to number', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{
        id: '1', type: 'frame', x: 0, y: 0,
        width: 'fit_content(250)' as unknown as number,
        height: 'fit_content' as unknown as number,
        children: [],
      }],
    }
    const result = normalizePenDocument(doc)
    const node = result.children[0] as { width: unknown; height: unknown }
    expect(node.width).toBe(250)
    expect(node.height).toBe('fit_content')
  })

  it('normalizes pages children too', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      pages: [{
        id: 'p1', name: 'Page 1',
        children: [{
          id: '1', type: 'rectangle', x: 0, y: 0,
          fill: [{ type: 'color' as 'solid', color: '#00ff00' }],
        }],
      }],
      children: [],
    }
    const result = normalizePenDocument(doc)
    const fill = (result.pages![0].children[0] as { fill: Array<{ type: string }> }).fill
    expect(fill[0].type).toBe('solid')
  })

  it('normalizes string elements inside fill array', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{
        id: '1', type: 'path', d: 'M0 0', x: 0, y: 0,
        fill: ['#ff0000'] as unknown as Array<{ type: 'solid'; color: string }>,
      }],
    }
    const result = normalizePenDocument(doc)
    const fill = (result.children[0] as { fill: Array<{ type: string; color: string }> }).fill
    expect(fill).toHaveLength(1)
    expect(fill[0]).toEqual({ type: 'solid', color: '#ff0000' })
  })

  it('preserves $variable references', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{
        id: '1', type: 'rectangle', x: 0, y: 0,
        fill: [{ type: 'solid', color: '$primary' }],
        opacity: '$opacity' as unknown as number,
      }],
    }
    const result = normalizePenDocument(doc)
    const node = result.children[0] as { fill: Array<{ color: string }>; opacity: unknown }
    expect(node.fill[0].color).toBe('$primary')
    expect(node.opacity).toBe('$opacity')
  })
})
