import { describe, it, expect } from 'vitest'
import type { PenNode } from '@zseven-w/pen-types'
import { resolveNodeForCanvas } from '../variables/resolve'

describe('resolveNodeForCanvas — recursive', () => {
  const variables = {
    spacing: { name: 'spacing', type: 'number' as const, value: 16 },
    'bg-color': { name: 'bg-color', type: 'color' as const, value: '#ff0000' },
  }

  it('resolves $variable gap on nested child frame', () => {
    const doc: PenNode = {
      id: 'root', type: 'frame', x: 0, y: 0, width: 400, height: 600,
      layout: 'vertical',
      children: [
        {
          id: 'child', type: 'frame', x: 0, y: 0, width: 300, height: 200,
          layout: 'vertical', gap: '$spacing' as unknown as number,
          children: [
            { id: 'text1', type: 'text', x: 0, y: 0, content: 'Hello' },
            { id: 'text2', type: 'text', x: 0, y: 0, content: 'World' },
          ],
        } as PenNode,
      ],
    } as PenNode

    const result = resolveNodeForCanvas(doc, variables)
    const childFrame = (result as any).children[0]
    expect(childFrame.gap).toBe(16)
  })

  it('resolves $variable fill on deeply nested text node', () => {
    const doc: PenNode = {
      id: 'root', type: 'frame', x: 0, y: 0, width: 400, height: 600,
      children: [
        {
          id: 'child', type: 'frame', x: 0, y: 0, width: 300, height: 200,
          children: [
            {
              id: 'text1', type: 'text', x: 0, y: 0, content: 'Hello',
              fill: [{ type: 'solid', color: '$bg-color' }],
            } as PenNode,
          ],
        } as PenNode,
      ],
    } as PenNode

    const result = resolveNodeForCanvas(doc, variables)
    const textNode = (result as any).children[0].children[0]
    expect(textNode.fill[0].color).toBe('#ff0000')
  })

  it('resolves $variable padding on nested frame', () => {
    const doc: PenNode = {
      id: 'root', type: 'frame', x: 0, y: 0, width: 400, height: 600,
      children: [
        {
          id: 'child', type: 'frame', x: 0, y: 0, width: 300, height: 200,
          padding: '$spacing' as unknown as number,
          children: [],
        } as PenNode,
      ],
    } as PenNode

    const result = resolveNodeForCanvas(doc, variables)
    const childFrame = (result as any).children[0]
    expect(childFrame.padding).toBe(16)
  })

  it('returns same reference when no variables exist', () => {
    const doc: PenNode = {
      id: 'root', type: 'frame', x: 0, y: 0, width: 400, height: 600,
      children: [
        { id: 'text1', type: 'text', x: 0, y: 0, content: 'Hello' },
      ],
    } as PenNode

    const result = resolveNodeForCanvas(doc, {})
    expect(result).toBe(doc)
  })

  it('resolves $variable opacity on nested text node', () => {
    const variables3 = {
      'text-opacity': { name: 'text-opacity', type: 'number' as const, value: 0.5 },
    }
    const doc: PenNode = {
      id: 'root', type: 'frame', x: 0, y: 0, width: 400, height: 600,
      children: [
        {
          id: 'child', type: 'frame', x: 0, y: 0, width: 300, height: 200,
          children: [
            {
              id: 'text1', type: 'text', x: 0, y: 0, content: 'Hello',
              opacity: '$text-opacity' as unknown as number,
            } as PenNode,
          ],
        } as PenNode,
      ],
    } as PenNode

    const result = resolveNodeForCanvas(doc, variables3)
    const textNode = (result as any).children[0].children[0]
    expect(textNode.opacity).toBe(0.5)
  })

  it('preserves reference when children have no variables', () => {
    const variables2 = { color1: { name: 'color1', type: 'color' as const, value: '#000' } }
    const doc: PenNode = {
      id: 'root', type: 'frame', x: 0, y: 0, width: 400, height: 600,
      children: [
        { id: 'text1', type: 'text', x: 0, y: 0, content: 'Hello' },
      ],
    } as PenNode

    const result = resolveNodeForCanvas(doc, variables2)
    expect(result).toBe(doc)
  })
})
