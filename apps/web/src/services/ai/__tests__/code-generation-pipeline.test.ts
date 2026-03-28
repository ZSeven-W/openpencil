// apps/web/src/services/ai/__tests__/code-generation-pipeline.test.ts

import { describe, it, expect } from 'vitest'
import { hydratePlan, computeExecutionOrder, parseChunkResponse } from '../code-generation-pipeline'
import type { CodePlanFromAI, PlannedChunk } from '@zseven-w/pen-codegen'
import type { PenNode } from '@zseven-w/pen-types'

describe('hydratePlan', () => {
  it('maps nodeIds to actual PenNode objects', () => {
    const nodes: PenNode[] = [
      { id: 'n1', type: 'frame', name: 'Header', x: 0, y: 0, width: 100, height: 50 } as PenNode,
      { id: 'n2', type: 'text', name: 'Title', x: 0, y: 0, width: 80, height: 20 } as PenNode,
    ]
    const plan: CodePlanFromAI = {
      chunks: [
        { id: 'c1', name: 'header', nodeIds: ['n1', 'n2'], role: 'navbar', suggestedComponentName: 'Header', dependencies: [] },
      ],
      sharedStyles: [],
      rootLayout: { direction: 'vertical', gap: 0, responsive: true },
    }

    const result = hydratePlan(plan, nodes)
    expect(result.chunks[0].nodes).toHaveLength(2)
    expect(result.chunks[0].nodes[0].id).toBe('n1')
  })

  it('strips chunks with no valid nodeIds', () => {
    const nodes: PenNode[] = [
      { id: 'n1', type: 'frame', name: 'Header', x: 0, y: 0, width: 100, height: 50 } as PenNode,
    ]
    const plan: CodePlanFromAI = {
      chunks: [
        { id: 'c1', name: 'header', nodeIds: ['n1'], role: 'navbar', suggestedComponentName: 'Header', dependencies: [] },
        { id: 'c2', name: 'ghost', nodeIds: ['nonexistent'], role: 'card', suggestedComponentName: 'Ghost', dependencies: [] },
      ],
      sharedStyles: [],
      rootLayout: { direction: 'vertical', gap: 0, responsive: true },
    }

    const result = hydratePlan(plan, nodes)
    expect(result.chunks).toHaveLength(1)
    expect(result.chunks[0].id).toBe('c1')
  })
})

describe('computeExecutionOrder', () => {
  it('assigns order 0 to chunks with no dependencies', () => {
    const chunks: PlannedChunk[] = [
      { id: 'c1', name: 'a', nodeIds: [], role: '', suggestedComponentName: 'A', dependencies: [] },
      { id: 'c2', name: 'b', nodeIds: [], role: '', suggestedComponentName: 'B', dependencies: [] },
    ]
    const orders = computeExecutionOrder(chunks)
    expect(orders.get('c1')).toBe(0)
    expect(orders.get('c2')).toBe(0)
  })

  it('assigns higher order to dependent chunks', () => {
    const chunks: PlannedChunk[] = [
      { id: 'c1', name: 'a', nodeIds: [], role: '', suggestedComponentName: 'A', dependencies: [] },
      { id: 'c2', name: 'b', nodeIds: [], role: '', suggestedComponentName: 'B', dependencies: ['c1'] },
      { id: 'c3', name: 'c', nodeIds: [], role: '', suggestedComponentName: 'C', dependencies: ['c2'] },
    ]
    const orders = computeExecutionOrder(chunks)
    expect(orders.get('c1')).toBe(0)
    expect(orders.get('c2')).toBe(1)
    expect(orders.get('c3')).toBe(2)
  })
})

describe('parseChunkResponse', () => {
  it('splits code and contract from ---CONTRACT--- separator', () => {
    const response = `export function NavBar() { return <nav /> }
---CONTRACT---
{"chunkId":"c1","componentName":"NavBar","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}`

    const result = parseChunkResponse(response, 'c1')
    expect(result.code).toContain('NavBar')
    expect(result.contract.componentName).toBe('NavBar')
    expect(result.contract.chunkId).toBe('c1')
  })

  it('infers component name when no separator found', () => {
    const response = 'export function NavBar() { return <nav /> }'
    const result = parseChunkResponse(response, 'c1')
    expect(result.code).toContain('NavBar')
    expect(result.contract.componentName).toBe('NavBar')
  })

  it('extracts contract from markdown json block', () => {
    const response = `\`\`\`tsx
export function HeroSection() { return <div /> }
\`\`\`

\`\`\`json
{"componentName":"HeroSection","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}
\`\`\``
    const result = parseChunkResponse(response, 'c2')
    expect(result.code).toContain('HeroSection')
    expect(result.contract.componentName).toBe('HeroSection')
    expect(result.contract.chunkId).toBe('c2')
  })
})
