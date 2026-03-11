import { describe, it, expect } from 'vitest'
import { buildAnimationIndex, getClipsForNode } from './animation-index'
import type { PenNode } from '@/types/pen'
import type { AnimationClipData } from '@/types/animation'

function makeClip(id: string): AnimationClipData {
  return {
    id,
    kind: 'animation',
    startTime: 0,
    duration: 1000,
    keyframes: [
      { id: 'kf1', offset: 0, properties: { opacity: 0 }, easing: 'linear' },
      { id: 'kf2', offset: 1, properties: { opacity: 1 }, easing: 'linear' },
    ],
  }
}

describe('buildAnimationIndex', () => {
  it('returns empty index for nodes without clips', () => {
    const nodes: PenNode[] = [
      { id: 'rect1', type: 'rectangle', width: 100, height: 100 },
    ]
    const index = buildAnimationIndex(nodes)
    expect(index.animatedNodes.size).toBe(0)
    expect(index.clipsByNode.size).toBe(0)
  })

  it('collects clips from flat nodes', () => {
    const clip = makeClip('clip-1')
    const nodes: PenNode[] = [
      { id: 'rect1', type: 'rectangle', width: 100, height: 100, clips: [clip] },
      { id: 'rect2', type: 'rectangle', width: 100, height: 100 },
    ]
    const index = buildAnimationIndex(nodes)
    expect(index.animatedNodes.size).toBe(1)
    expect(index.animatedNodes.has('rect1')).toBe(true)
    expect(index.clipsByNode.get('rect1')).toEqual([clip])
  })

  it('collects clips from nested frames/groups', () => {
    const clipA = makeClip('clip-a')
    const clipB = makeClip('clip-b')
    const nodes: PenNode[] = [
      {
        id: 'frame1',
        type: 'frame',
        width: 500,
        height: 500,
        clips: [clipA],
        children: [
          {
            id: 'group1',
            type: 'group',
            children: [
              { id: 'rect-inner', type: 'rectangle', width: 50, height: 50, clips: [clipB] },
            ],
          },
        ],
      },
    ]
    const index = buildAnimationIndex(nodes)
    expect(index.animatedNodes.size).toBe(2)
    expect(index.animatedNodes.has('frame1')).toBe(true)
    expect(index.animatedNodes.has('rect-inner')).toBe(true)
  })

  it('increments version on each build', () => {
    const nodes: PenNode[] = []
    const idx1 = buildAnimationIndex(nodes)
    const idx2 = buildAnimationIndex(nodes)
    expect(idx2.version).toBeGreaterThan(idx1.version)
  })
})

describe('getClipsForNode', () => {
  it('returns clips for animated node', () => {
    const clip = makeClip('c1')
    const nodes: PenNode[] = [
      { id: 'n1', type: 'rectangle', width: 100, height: 100, clips: [clip] },
    ]
    const index = buildAnimationIndex(nodes)
    expect(getClipsForNode(index, 'n1')).toEqual([clip])
  })

  it('returns empty array for non-animated node', () => {
    const index = buildAnimationIndex([])
    expect(getClipsForNode(index, 'missing')).toEqual([])
  })
})
