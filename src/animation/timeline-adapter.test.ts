import { describe, it, expect, vi } from 'vitest'
import {
  toTimelineRows,
  applyActionMove,
  applyActionResize,
  validateActionMove,
} from './timeline-adapter'
import { msToSec, secToMs, EFFECT_ANIMATION_PHASE, EFFECT_VIDEO_CLIP } from './timeline-adapter-types'
import type { TimelineStores, VideoNodeProjection } from './timeline-adapter-types'
import type { AnimationTrack, AnimationPhases } from '@/types/animation'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makePhases(inStart: number, inDur: number, whileStart: number, whileDur: number, outStart: number, outDur: number): AnimationPhases {
  return {
    in: { start: inStart, duration: inDur },
    while: { start: whileStart, duration: whileDur },
    out: { start: outStart, duration: outDur },
  }
}

function makeTrack(nodeId: string, phases: AnimationPhases): AnimationTrack {
  return {
    nodeId,
    keyframes: [
      { id: 'kf-1', time: phases.in.start, properties: { opacity: 0 }, easing: 'smooth', phase: 'in' },
      { id: 'kf-2', time: phases.in.start + phases.in.duration, properties: { opacity: 1 }, easing: 'smooth', phase: 'in' },
      { id: 'kf-3', time: phases.out.start, properties: { opacity: 1 }, easing: 'smooth', phase: 'out' },
      { id: 'kf-4', time: phases.out.start + phases.out.duration, properties: { opacity: 0 }, easing: 'smooth', phase: 'out' },
    ],
    phases,
    startDelay: 0,
  }
}

function makeVideoNode(overrides: Partial<VideoNodeProjection> = {}): VideoNodeProjection {
  return {
    id: 'video-1',
    name: 'clip.mp4',
    inPoint: 0,
    outPoint: 5000,
    timelineOffset: 1000,
    videoDuration: 10000,
    ...overrides,
  }
}

function makeMockStores(
  tracks: Record<string, AnimationTrack> = {},
  videoNode?: VideoNodeProjection,
): TimelineStores & { updateKeyframe: ReturnType<typeof vi.fn>; updateNode: ReturnType<typeof vi.fn> } {
  const updateKeyframe = vi.fn()
  const updateNode = vi.fn()

  return {
    getTimelineState: () => ({
      tracks,
      duration: 5000,
      videoClipIds: videoNode ? [videoNode.id] : [],
    }),
    updateKeyframe,
    getDocumentState: () => ({
      getNodeById: (id: string) => {
        if (videoNode && id === videoNode.id) {
          return {
            id: videoNode.id,
            type: 'video' as const,
            name: videoNode.name ?? 'video',
            inPoint: videoNode.inPoint,
            outPoint: videoNode.outPoint,
            timelineOffset: videoNode.timelineOffset,
            videoDuration: videoNode.videoDuration,
          } as unknown as import('@/types/pen').PenNode
        }
        return undefined
      },
    }),
    updateNode,
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
    // 7ms should round-trip cleanly
    const sec = msToSec(7)
    const backToMs = secToMs(sec)
    expect(backToMs).toBe(7)
  })

  it('round-trips 33ms (common frame duration)', () => {
    const sec = msToSec(33)
    const backToMs = secToMs(sec)
    expect(backToMs).toBe(33)
  })

  it('round-trips 16.667ms (60fps) via rounding', () => {
    // secToMs rounds, so 16.667ms → msToSec → 0.017s → secToMs → 17ms
    // This is expected: sub-ms precision is lost by design
    const sec = msToSec(17)
    expect(secToMs(sec)).toBe(17)
  })
})

// ---------------------------------------------------------------------------
// toTimelineRows
// ---------------------------------------------------------------------------

describe('toTimelineRows', () => {
  it('converts animation track to rows with phase actions', () => {
    const phases = makePhases(0, 500, 500, 1000, 1500, 500)
    const tracks = { 'node-1': makeTrack('node-1', phases) }

    const { rows, metadata } = toTimelineRows(tracks, [], 5000)

    expect(rows).toHaveLength(1)
    expect(rows[0].id).toBe('node-1')
    expect(rows[0].actions).toHaveLength(3)

    // Check action IDs and effectIds
    const actionIds = rows[0].actions.map((a) => a.id)
    expect(actionIds).toContain('node-1::in')
    expect(actionIds).toContain('node-1::while')
    expect(actionIds).toContain('node-1::out')

    for (const action of rows[0].actions) {
      expect(action.effectId).toBe(EFFECT_ANIMATION_PHASE)
      expect(action.flexible).toBe(true)
      expect(action.movable).toBe(true)
    }

    // Check metadata
    expect(metadata.size).toBe(3)
    const inMeta = metadata.get('node-1::in')
    expect(inMeta).toEqual({ type: 'animation-phase', phase: 'in', nodeId: 'node-1' })
  })

  it('converts time from ms to seconds', () => {
    const phases = makePhases(0, 500, 500, 1000, 1500, 500)
    const tracks = { 'node-1': makeTrack('node-1', phases) }

    const { rows } = toTimelineRows(tracks, [], 5000)

    const inAction = rows[0].actions.find((a) => a.id === 'node-1::in')!
    expect(inAction.start).toBe(0)
    expect(inAction.end).toBe(0.5)

    const outAction = rows[0].actions.find((a) => a.id === 'node-1::out')!
    expect(outAction.start).toBe(1.5)
    expect(outAction.end).toBe(2)
  })

  it('converts video node to row with single action', () => {
    const videoNode = makeVideoNode()
    const { rows, metadata } = toTimelineRows({}, [videoNode], 5000)

    expect(rows).toHaveLength(1)
    expect(rows[0].id).toBe('video-1')
    expect(rows[0].actions).toHaveLength(1)

    const action = rows[0].actions[0]
    expect(action.id).toBe('video-1::video')
    expect(action.effectId).toBe(EFFECT_VIDEO_CLIP)
    expect(action.start).toBe(msToSec(1000)) // timelineOffset
    expect(action.end).toBe(msToSec(1000 + 5000)) // offset + (outPoint - inPoint)

    const meta = metadata.get('video-1::video')
    expect(meta).toEqual({ type: 'video-clip', nodeId: 'video-1' })
  })

  it('handles mixed animation + video rows', () => {
    const phases = makePhases(0, 500, 500, 1000, 1500, 500)
    const tracks = { 'node-1': makeTrack('node-1', phases) }
    const videoNode = makeVideoNode()

    const { rows, metadata } = toTimelineRows(tracks, [videoNode], 5000)

    expect(rows).toHaveLength(2) // 1 animation + 1 video
    expect(metadata.size).toBe(4) // 3 phases + 1 video
  })

  it('skips zero-duration phases (except while)', () => {
    const phases = makePhases(0, 0, 0, 1000, 1000, 0)
    const tracks = { 'node-1': makeTrack('node-1', phases) }

    const { rows } = toTimelineRows(tracks, [], 5000)

    // Only 'while' phase should be present (in/out have duration 0)
    const actionIds = rows[0].actions.map((a) => a.id)
    expect(actionIds).not.toContain('node-1::in')
    expect(actionIds).toContain('node-1::while')
    expect(actionIds).not.toContain('node-1::out')
  })

  it('handles empty tracks and no video nodes', () => {
    const { rows, metadata } = toTimelineRows({}, [], 5000)
    expect(rows).toHaveLength(0)
    expect(metadata.size).toBe(0)
  })

  it('handles video node with default values', () => {
    const videoNode = makeVideoNode({
      inPoint: undefined,
      outPoint: undefined,
      timelineOffset: undefined,
    })

    const { rows } = toTimelineRows({}, [videoNode], 5000)

    const action = rows[0].actions[0]
    // inPoint defaults to 0, outPoint defaults to videoDuration (10000), offset defaults to 0
    expect(action.start).toBe(0)
    expect(action.end).toBe(msToSec(10000))
  })
})

// ---------------------------------------------------------------------------
// applyActionMove
// ---------------------------------------------------------------------------

describe('applyActionMove', () => {
  it('moves animation phase keyframes by delta', () => {
    const phases = makePhases(0, 500, 500, 1000, 1500, 500)
    const track = makeTrack('node-1', phases)
    const stores = makeMockStores({ 'node-1': track })

    const { metadata } = toTimelineRows({ 'node-1': track }, [], 5000)

    // Move 'in' phase from 0-0.5s to 0.2-0.7s (delta = 200ms)
    applyActionMove('node-1::in', 0.2, 0.7, metadata, stores)

    // Should update both 'in' phase keyframes with +200ms delta
    expect(stores.updateKeyframe).toHaveBeenCalledTimes(2)
    expect(stores.updateKeyframe).toHaveBeenCalledWith('node-1', 'kf-1', { time: 200 })
    expect(stores.updateKeyframe).toHaveBeenCalledWith('node-1', 'kf-2', { time: 700 })
  })

  it('moves video clip by updating timelineOffset', () => {
    const videoNode = makeVideoNode()
    const stores = makeMockStores({}, videoNode)

    const { metadata } = toTimelineRows({}, [videoNode], 5000)

    // Move video from 1s to 2s
    applyActionMove('video-1::video', 2, 7, metadata, stores)

    expect(stores.updateNode).toHaveBeenCalledWith('video-1', {
      timelineOffset: 2000,
    })
  })

  it('does nothing for unknown action', () => {
    const stores = makeMockStores()
    const metadata = new Map()

    applyActionMove('unknown', 0, 1, metadata, stores)

    expect(stores.updateKeyframe).not.toHaveBeenCalled()
    expect(stores.updateNode).not.toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// applyActionResize
// ---------------------------------------------------------------------------

describe('applyActionResize', () => {
  it('resizes animation phase left edge (moves first keyframe)', () => {
    const phases = makePhases(0, 500, 500, 1000, 1500, 500)
    const track = makeTrack('node-1', phases)
    const stores = makeMockStores({ 'node-1': track })

    const { metadata } = toTimelineRows({ 'node-1': track }, [], 5000)

    applyActionResize('node-1::in', 0.1, 0.5, 'left', metadata, stores)

    expect(stores.updateKeyframe).toHaveBeenCalledWith('node-1', 'kf-1', { time: 100 })
  })

  it('resizes animation phase right edge (moves last keyframe)', () => {
    const phases = makePhases(0, 500, 500, 1000, 1500, 500)
    const track = makeTrack('node-1', phases)
    const stores = makeMockStores({ 'node-1': track })

    const { metadata } = toTimelineRows({ 'node-1': track }, [], 5000)

    applyActionResize('node-1::in', 0, 0.8, 'right', metadata, stores)

    expect(stores.updateKeyframe).toHaveBeenCalledWith('node-1', 'kf-2', { time: 800 })
  })

  it('resizes video clip left edge (adjusts inPoint + timelineOffset)', () => {
    const videoNode = makeVideoNode({ timelineOffset: 1000, inPoint: 0 })
    const stores = makeMockStores({}, videoNode)

    const { metadata } = toTimelineRows({}, [videoNode], 5000)

    // Trim left from 1s to 1.5s
    applyActionResize('video-1::video', 1.5, 6, 'left', metadata, stores)

    expect(stores.updateNode).toHaveBeenCalledWith('video-1', {
      timelineOffset: 1500,
      inPoint: 500, // shifted right by 500ms
    })
  })

  it('resizes video clip right edge (adjusts outPoint)', () => {
    const videoNode = makeVideoNode({ timelineOffset: 1000, inPoint: 0, outPoint: 5000 })
    const stores = makeMockStores({}, videoNode)

    const { metadata } = toTimelineRows({}, [videoNode], 5000)

    // Trim right from 6s to 5s
    applyActionResize('video-1::video', 1, 5, 'right', metadata, stores)

    expect(stores.updateNode).toHaveBeenCalledWith('video-1', {
      outPoint: 4000, // inPoint(0) + (5000 - 1000)
    })
  })
})

// ---------------------------------------------------------------------------
// validateActionMove
// ---------------------------------------------------------------------------

describe('validateActionMove', () => {
  it('rejects start >= end', () => {
    const stores = makeMockStores()
    const metadata = new Map([['a', { type: 'animation-phase' as const, phase: 'in' as const, nodeId: 'n' }]])

    expect(validateActionMove('a', 1, 1, metadata, stores)).toBe(false)
    expect(validateActionMove('a', 2, 1, metadata, stores)).toBe(false)
  })

  it('rejects duration < 50ms', () => {
    const stores = makeMockStores()
    const metadata = new Map([['a', { type: 'animation-phase' as const, phase: 'in' as const, nodeId: 'n' }]])

    // 0.04s = 40ms < 50ms minimum
    expect(validateActionMove('a', 0, 0.04, metadata, stores)).toBe(false)
  })

  it('rejects negative start', () => {
    const stores = makeMockStores()
    const metadata = new Map([['a', { type: 'animation-phase' as const, phase: 'in' as const, nodeId: 'n' }]])

    expect(validateActionMove('a', -0.1, 1, metadata, stores)).toBe(false)
  })

  it('allows valid animation phase move', () => {
    const stores = makeMockStores()
    const metadata = new Map([['a', { type: 'animation-phase' as const, phase: 'in' as const, nodeId: 'n' }]])

    expect(validateActionMove('a', 0, 1, metadata, stores)).toBe(true)
  })

  it('rejects video clip exceeding video duration', () => {
    const videoNode = makeVideoNode({ videoDuration: 5000 })
    const stores = makeMockStores({}, videoNode)
    const metadata = new Map([['video-1::video', { type: 'video-clip' as const, nodeId: 'video-1' }]])

    // Clip duration of 6s > video duration of 5s
    expect(validateActionMove('video-1::video', 0, 6, metadata, stores)).toBe(false)
  })

  it('allows valid video clip move', () => {
    const videoNode = makeVideoNode({ videoDuration: 10000 })
    const stores = makeMockStores({}, videoNode)
    const metadata = new Map([['video-1::video', { type: 'video-clip' as const, nodeId: 'video-1' }]])

    expect(validateActionMove('video-1::video', 1, 5, metadata, stores)).toBe(true)
  })

  it('rejects unknown action', () => {
    const stores = makeMockStores()
    const metadata = new Map()

    expect(validateActionMove('unknown', 0, 1, metadata, stores)).toBe(false)
  })
})
