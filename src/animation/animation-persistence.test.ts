import { describe, it, expect, vi } from 'vitest'
import type { PenNode } from '@/types/pen'
import type { AnimationClipData, VideoClipData } from '@/types/animation'

// Mock effect-registry before importing the module under test
vi.mock('@/animation/effect-registry', () => ({
  getEffect: (id: string) => {
    const validEffects = new Set(['fadeIn', 'slideUp'])
    return validEffects.has(id) ? { id, name: id } : undefined
  },
}))

import {
  reconcileAnimationWithDocument,
  cleanOrphanedClips,
} from './animation-persistence'

function makeAnimationClip(
  id: string,
  effectId?: string,
): AnimationClipData {
  return {
    id,
    kind: 'animation',
    startTime: 0,
    duration: 1000,
    effectId,
    keyframes: [],
  }
}

function makeVideoClip(id: string): VideoClipData {
  return {
    id,
    kind: 'video',
    startTime: 0,
    duration: 2000,
    sourceStart: 0,
    sourceEnd: 2000,
    playbackRate: 1,
  }
}

function makeNode(
  id: string,
  clips?: PenNode['clips'],
  children?: PenNode[],
): PenNode {
  return {
    id,
    type: 'rectangle',
    name: id,
    x: 0,
    y: 0,
    width: 100,
    height: 100,
    clips,
    children,
  } as unknown as PenNode
}

describe('reconcileAnimationWithDocument', () => {
  it('removes clips with invalid effectIds', () => {
    const nodes = [
      makeNode('a', [
        makeAnimationClip('c1', 'fadeIn'),
        makeAnimationClip('c2', 'nonExistentEffect'),
      ]),
    ]

    reconcileAnimationWithDocument(nodes)

    expect(nodes[0].clips).toHaveLength(1)
    expect(nodes[0].clips![0].id).toBe('c1')
  })

  it('keeps clips without effectId', () => {
    const nodes = [
      makeNode('a', [makeAnimationClip('c1', undefined)]),
    ]

    reconcileAnimationWithDocument(nodes)

    expect(nodes[0].clips).toHaveLength(1)
  })

  it('keeps video clips regardless of effectId', () => {
    const nodes = [
      makeNode('a', [makeVideoClip('v1')]),
    ]

    reconcileAnimationWithDocument(nodes)

    expect(nodes[0].clips).toHaveLength(1)
    expect(nodes[0].clips![0].kind).toBe('video')
  })

  it('sets clips to undefined when all are invalid', () => {
    const nodes = [
      makeNode('a', [
        makeAnimationClip('c1', 'bogus'),
        makeAnimationClip('c2', 'alsoInvalid'),
      ]),
    ]

    reconcileAnimationWithDocument(nodes)

    expect(nodes[0].clips).toBeUndefined()
  })

  it('handles nested children', () => {
    const child = makeNode('child', [
      makeAnimationClip('c1', 'nonExistent'),
    ])
    const nodes = [makeNode('parent', undefined, [child])]

    reconcileAnimationWithDocument(nodes)

    const nestedChild = (nodes[0] as any).children[0]
    expect(nestedChild.clips).toBeUndefined()
  })

  it('leaves nodes without clips unchanged', () => {
    const nodes = [makeNode('a')]

    reconcileAnimationWithDocument(nodes)

    expect(nodes[0].clips).toBeUndefined()
  })
})

describe('cleanOrphanedClips', () => {
  it('returns same node when all effectIds are valid', () => {
    const node = makeNode('a', [
      makeAnimationClip('c1', 'fadeIn'),
    ])
    const validIds = new Set(['fadeIn', 'slideUp'])

    const result = cleanOrphanedClips(node, validIds)

    expect(result).toBe(node) // same reference — no change
  })

  it('returns cleaned copy with invalid effectIds removed', () => {
    const node = makeNode('a', [
      makeAnimationClip('c1', 'fadeIn'),
      makeAnimationClip('c2', 'invalidEffect'),
    ])
    const validIds = new Set(['fadeIn'])

    const result = cleanOrphanedClips(node, validIds)

    expect(result).not.toBe(node)
    expect(result.clips).toHaveLength(1)
    expect(result.clips![0].id).toBe('c1')
  })

  it('sets clips to undefined when all are invalid', () => {
    const node = makeNode('a', [
      makeAnimationClip('c1', 'bogus'),
    ])
    const validIds = new Set(['fadeIn'])

    const result = cleanOrphanedClips(node, validIds)

    expect(result.clips).toBeUndefined()
  })

  it('returns same node when no clips exist', () => {
    const node = makeNode('a')
    const validIds = new Set(['fadeIn'])

    const result = cleanOrphanedClips(node, validIds)

    expect(result).toBe(node)
  })

  it('keeps video clips', () => {
    const node = makeNode('a', [makeVideoClip('v1')])
    const validIds = new Set<string>()

    const result = cleanOrphanedClips(node, validIds)

    expect(result).toBe(node) // video clips are always valid
  })

  it('keeps animation clips without effectId', () => {
    const node = makeNode('a', [makeAnimationClip('c1', undefined)])
    const validIds = new Set<string>()

    const result = cleanOrphanedClips(node, validIds)

    expect(result).toBe(node)
  })
})
