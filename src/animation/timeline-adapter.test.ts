import { describe, it, expect, vi } from 'vitest'
import {
  applyActionMove,
  applyActionResize,
  validateActionMove,
  buildTimelineRowsFromNodes,
  clipToTimelineAction,
} from './timeline-adapter'
import { msToSec, secToMs, EFFECT_ANIMATION_CLIP } from './timeline-adapter-types'
import type { TimelineStores } from './timeline-adapter-types'
import type { PenNode, VideoNode } from '@/types/pen'
import type { AnimationClipData } from '@/types/animation'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeVideoNode(overrides: Partial<VideoNode> = {}): VideoNode {
  return {
    id: 'video-1',
    type: 'video',
    name: 'clip.mp4',
    x: 0,
    y: 0,
    width: 100,
    height: 100,
    src: '',
    mimeType: 'video/mp4',
    videoDuration: 10000,
    inPoint: 0,
    outPoint: 5000,
    timelineOffset: 1000,
    ...overrides,
  } as VideoNode
}

function makeNodeWithClips(
  nodeId: string,
  clips: AnimationClipData[],
): PenNode {
  return {
    id: nodeId,
    type: 'rectangle',
    name: 'rect',
    x: 0,
    y: 0,
    width: 100,
    height: 100,
    clips,
  } as unknown as PenNode
}

function makeMockStores(
  videoNode?: VideoNode,
): TimelineStores & { updateNode: ReturnType<typeof vi.fn> } {
  const updateNode = vi.fn()
  const nodes = new Map<string, PenNode>()
  if (videoNode) nodes.set(videoNode.id, videoNode as unknown as PenNode)

  return {
    getDocumentState: () => ({
      getNodeById: (id: string) => nodes.get(id),
    }),
    updateNode,
    getDuration: () => 5000,
  }
}

// ---------------------------------------------------------------------------
// Time conversion
// ---------------------------------------------------------------------------

describe('time conversion', () => {
  it('converts ms to seconds', () => {
    expect(msToSec(1000)).toBe(1)
    expect(msToSec(500)).toBe(0.5)
    expect(msToSec(0)).toBe(0)
  })

  it('converts seconds to ms', () => {
    expect(secToMs(1)).toBe(1000)
    expect(secToMs(0.5)).toBe(500)
    expect(secToMs(0)).toBe(0)
  })

  it('handles IEEE 754 drift (7ms = 0.007s)', () => {
    const sec = msToSec(7)
    const backToMs = secToMs(sec)
    expect(backToMs).toBe(7)
  })

  it('round-trips 33ms (common frame duration)', () => {
    const sec = msToSec(33)
    const backToMs = secToMs(sec)
    expect(backToMs).toBe(33)
  })
})

// ---------------------------------------------------------------------------
// clipToTimelineAction
// ---------------------------------------------------------------------------

describe('clipToTimelineAction', () => {
  it('converts animation clip to timeline action', () => {
    const clip: AnimationClipData = {
      id: 'c1',
      kind: 'animation',
      startTime: 500,
      duration: 1000,
      keyframes: [],
    }

    const action = clipToTimelineAction(clip)

    expect(action.id).toBe('c1')
    expect(action.start).toBe(msToSec(500))
    expect(action.end).toBe(msToSec(1500))
    expect(action.effectId).toBe(EFFECT_ANIMATION_CLIP)
    expect(action.flexible).toBe(true)
    expect(action.movable).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// buildTimelineRowsFromNodes
// ---------------------------------------------------------------------------

describe('buildTimelineRowsFromNodes', () => {
  it('creates rows for nodes with clips', () => {
    const clips: AnimationClipData[] = [
      { id: 'c1', kind: 'animation', startTime: 0, duration: 500, keyframes: [] },
      { id: 'c2', kind: 'animation', startTime: 500, duration: 500, keyframes: [] },
    ]
    const node = makeNodeWithClips('node-1', clips)

    const { rows, metadata } = buildTimelineRowsFromNodes([node])

    expect(rows).toHaveLength(1)
    expect(rows[0].id).toBe('v2::node-1')
    expect(rows[0].actions).toHaveLength(2)

    expect(metadata.size).toBe(2)
    expect(metadata.get('c1')).toEqual({
      type: 'animation-clip',
      nodeId: 'node-1',
      clipId: 'c1',
    })
  })

  it('returns empty for nodes without clips', () => {
    const node = { id: 'n1', type: 'rectangle', name: 'r', x: 0, y: 0, width: 100, height: 100 } as unknown as PenNode
    const { rows, metadata } = buildTimelineRowsFromNodes([node])
    expect(rows).toHaveLength(0)
    expect(metadata.size).toBe(0)
  })

  it('walks children recursively', () => {
    const child = makeNodeWithClips('child-1', [
      { id: 'c1', kind: 'animation', startTime: 0, duration: 300, keyframes: [] },
    ])
    const parent = {
      id: 'frame-1',
      type: 'frame',
      name: 'frame',
      x: 0,
      y: 0,
      width: 200,
      height: 200,
      children: [child],
    } as unknown as PenNode

    const { rows } = buildTimelineRowsFromNodes([parent])
    expect(rows).toHaveLength(1)
    expect(rows[0].id).toBe('v2::child-1')
  })
})

// ---------------------------------------------------------------------------
// applyActionMove (v2 clip-based)
// ---------------------------------------------------------------------------

describe('applyActionMove', () => {
  it('moves video clip by updating timelineOffset', () => {
    const videoNode = makeVideoNode()
    const stores = makeMockStores(videoNode)

    const metadata = new Map([
      ['video-1::video', { type: 'video-clip' as const, nodeId: 'video-1' }],
    ])

    applyActionMove('video-1::video', 2, 7, metadata, stores)

    expect(stores.updateNode).toHaveBeenCalledWith('video-1', {
      timelineOffset: 2000,
    })
  })

  it('moves animation clip by updating startTime', () => {
    const clip: AnimationClipData = {
      id: 'c1',
      kind: 'animation',
      startTime: 0,
      duration: 500,
      keyframes: [],
    }
    const node = makeNodeWithClips('node-1', [clip])
    const updateNode = vi.fn()
    const stores: TimelineStores = {
      getDocumentState: () => ({
        getNodeById: (id: string) => (id === 'node-1' ? node : undefined),
      }),
      updateNode,
      getDuration: () => 5000,
    }

    const metadata = new Map([
      ['c1', { type: 'animation-clip' as const, nodeId: 'node-1', clipId: 'c1' }],
    ])

    applyActionMove('c1', 0.5, 1, metadata, stores)

    expect(updateNode).toHaveBeenCalledWith('node-1', {
      clips: [{ ...clip, startTime: 500 }],
    })
  })

  it('does nothing for unknown action', () => {
    const stores = makeMockStores()
    const metadata = new Map()

    applyActionMove('unknown', 0, 1, metadata, stores)
    expect(stores.updateNode).not.toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// applyActionResize
// ---------------------------------------------------------------------------

describe('applyActionResize', () => {
  it('resizes video clip left edge (adjusts inPoint + timelineOffset)', () => {
    const videoNode = makeVideoNode({ timelineOffset: 1000, inPoint: 0 })
    const stores = makeMockStores(videoNode)

    const metadata = new Map([
      ['video-1::video', { type: 'video-clip' as const, nodeId: 'video-1' }],
    ])

    applyActionResize('video-1::video', 1.5, 6, 'left', metadata, stores)

    expect(stores.updateNode).toHaveBeenCalledWith('video-1', {
      timelineOffset: 1500,
      inPoint: 500,
    })
  })

  it('resizes video clip right edge (adjusts outPoint)', () => {
    const videoNode = makeVideoNode({ timelineOffset: 1000, inPoint: 0, outPoint: 5000 })
    const stores = makeMockStores(videoNode)

    const metadata = new Map([
      ['video-1::video', { type: 'video-clip' as const, nodeId: 'video-1' }],
    ])

    applyActionResize('video-1::video', 1, 5, 'right', metadata, stores)

    expect(stores.updateNode).toHaveBeenCalledWith('video-1', {
      outPoint: 4000,
    })
  })
})

// ---------------------------------------------------------------------------
// validateActionMove
// ---------------------------------------------------------------------------

describe('validateActionMove', () => {
  it('rejects start >= end', () => {
    const stores = makeMockStores()
    const metadata = new Map([
      ['a', { type: 'animation-clip' as const, nodeId: 'n', clipId: 'c' }],
    ])

    expect(validateActionMove('a', 1, 1, metadata, stores)).toBe(false)
    expect(validateActionMove('a', 2, 1, metadata, stores)).toBe(false)
  })

  it('rejects duration < 50ms', () => {
    const stores = makeMockStores()
    const metadata = new Map([
      ['a', { type: 'animation-clip' as const, nodeId: 'n', clipId: 'c' }],
    ])

    expect(validateActionMove('a', 0, 0.04, metadata, stores)).toBe(false)
  })

  it('rejects negative start', () => {
    const stores = makeMockStores()
    const metadata = new Map([
      ['a', { type: 'animation-clip' as const, nodeId: 'n', clipId: 'c' }],
    ])

    expect(validateActionMove('a', -0.1, 1, metadata, stores)).toBe(false)
  })

  it('allows valid animation clip move', () => {
    const stores = makeMockStores()
    const metadata = new Map([
      ['a', { type: 'animation-clip' as const, nodeId: 'n', clipId: 'c' }],
    ])

    expect(validateActionMove('a', 0, 1, metadata, stores)).toBe(true)
  })

  it('rejects video clip exceeding video duration', () => {
    const videoNode = makeVideoNode({ videoDuration: 5000 })
    const stores = makeMockStores(videoNode)
    const metadata = new Map([
      ['video-1::video', { type: 'video-clip' as const, nodeId: 'video-1' }],
    ])

    expect(validateActionMove('video-1::video', 0, 6, metadata, stores)).toBe(false)
  })

  it('allows valid video clip move', () => {
    const videoNode = makeVideoNode({ videoDuration: 10000 })
    const stores = makeMockStores(videoNode)
    const metadata = new Map([
      ['video-1::video', { type: 'video-clip' as const, nodeId: 'video-1' }],
    ])

    expect(validateActionMove('video-1::video', 1, 5, metadata, stores)).toBe(true)
  })

  it('rejects unknown action', () => {
    const stores = makeMockStores()
    const metadata = new Map()

    expect(validateActionMove('unknown', 0, 1, metadata, stores)).toBe(false)
  })
})
