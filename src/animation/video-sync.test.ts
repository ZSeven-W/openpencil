import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { Canvas } from 'fabric'
import type { AnimationIndex } from '@/animation/animation-index'
import type { VideoClipData, AnimationClip } from '@/types/animation'

// Mock the decoder registry
const mockDecoder = {
  advanceFrame: vi.fn().mockReturnValue(false),
  drawFrame: vi.fn().mockResolvedValue(undefined),
  startPlayback: vi.fn(),
  stopPlayback: vi.fn(),
  dispose: vi.fn(),
  isPlaying: false,
  duration: 5,
  width: 1920,
  height: 1080,
  hasAudio: false,
  canvas: {} as HTMLCanvasElement,
  resizeCanvas: vi.fn(),
}

vi.mock('@/animation/video-registry', () => ({
  getVideoDecoder: vi.fn(() => mockDecoder),
  registerVideoDecoder: vi.fn(),
  unregisterVideoDecoder: vi.fn(),
}))

vi.mock('@/animation/canvas-bridge', () => ({
  findFabricObject: vi.fn(),
}))

import { getVideoDecoder } from '@/animation/video-registry'
import { findFabricObject } from '@/animation/canvas-bridge'
import {
  syncVideoFramesMB,
  seekVideoFramesMB,
  startVideoPlaybackMB,
  stopVideoPlaybackMB,
} from '@/animation/video-sync'

function makeVideoClip(overrides: Partial<VideoClipData> = {}): VideoClipData {
  return {
    id: 'vc-1',
    kind: 'video',
    startTime: 0,
    duration: 5000,
    sourceStart: 0,
    sourceEnd: 5000,
    playbackRate: 1,
    ...overrides,
  }
}

function makeIndex(clips: [string, AnimationClip[]][]): AnimationIndex {
  return {
    clipsByNode: new Map(clips),
    animatedNodes: new Set(clips.map(([id]) => id)),
    version: 1,
  }
}

const mockCanvas = { requestRenderAll: vi.fn() } as unknown as Canvas
const mockFabricObj = { dirty: false, visible: true, penNodeId: 'node-1' }

beforeEach(() => {
  vi.clearAllMocks()
  mockFabricObj.dirty = false
  mockFabricObj.visible = true
  mockDecoder.advanceFrame.mockReturnValue(false)
  vi.mocked(findFabricObject).mockReturnValue(mockFabricObj as any)
  vi.mocked(getVideoDecoder).mockReturnValue(mockDecoder as any)
})

describe('syncVideoFramesMB', () => {
  it('calls advanceFrame with correct source time when in range', () => {
    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesMB(mockCanvas, 2500, index)

    // t=2500ms, clip 0-5000ms → clipLocal=2500, progress=0.5, source = 0 + 0.5*5000 = 2500ms = 2.5s
    expect(mockDecoder.advanceFrame).toHaveBeenCalledWith(2.5)
  })

  it('marks dirty when frame advances', () => {
    mockDecoder.advanceFrame.mockReturnValue(true)
    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesMB(mockCanvas, 2500, index)

    expect(mockFabricObj.dirty).toBe(true)
  })

  it('does not mark dirty when frame does not advance', () => {
    mockDecoder.advanceFrame.mockReturnValue(false)
    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesMB(mockCanvas, 2500, index)

    expect(mockFabricObj.dirty).toBe(false)
  })

  it('hides fabric object when before clip start', () => {
    const clip = makeVideoClip({ startTime: 1000 })
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesMB(mockCanvas, 500, index)

    expect(mockFabricObj.visible).toBe(false)
    expect(mockDecoder.advanceFrame).not.toHaveBeenCalled()
  })

  it('hides fabric object when after clip end', () => {
    const clip = makeVideoClip({ startTime: 0, duration: 2000 })
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesMB(mockCanvas, 3000, index)

    expect(mockFabricObj.visible).toBe(false)
  })

  it('maps source time correctly with offset source range', () => {
    // Clip: startTime=0, duration=4000, sourceStart=2000, sourceEnd=6000
    // At t=2000 (halfway), expected source = (2000 + 0.5*4000) / 1000 = 4.0s
    const clip = makeVideoClip({
      sourceStart: 2000,
      sourceEnd: 6000,
      duration: 4000,
    })
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesMB(mockCanvas, 2000, index)

    expect(mockDecoder.advanceFrame).toHaveBeenCalledWith(4.0)
  })

  it('skips non-video clips', () => {
    const animClip = {
      id: 'ac-1',
      kind: 'animation' as const,
      startTime: 0,
      duration: 1000,
      keyframes: [],
    }
    const index = makeIndex([['node-1', [animClip as AnimationClip]]])

    syncVideoFramesMB(mockCanvas, 500, index)

    expect(mockDecoder.advanceFrame).not.toHaveBeenCalled()
  })

  it('skips when no decoder found', () => {
    vi.mocked(getVideoDecoder).mockReturnValue(undefined)
    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesMB(mockCanvas, 2500, index)

    expect(mockDecoder.advanceFrame).not.toHaveBeenCalled()
  })
})

describe('seekVideoFramesMB', () => {
  it('calls drawFrame with correct source time', async () => {
    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    await seekVideoFramesMB(mockCanvas, 3000, index)

    expect(mockDecoder.drawFrame).toHaveBeenCalledWith(3.0)
    expect(mockFabricObj.dirty).toBe(true)
  })

  it('does not seek when outside clip range', async () => {
    const clip = makeVideoClip({ startTime: 5000 })
    const index = makeIndex([['node-1', [clip]]])

    await seekVideoFramesMB(mockCanvas, 1000, index)

    expect(mockDecoder.drawFrame).not.toHaveBeenCalled()
  })
})

describe('startVideoPlaybackMB', () => {
  it('starts playback at correct source time', () => {
    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    startVideoPlaybackMB(mockCanvas, 2500, index)

    expect(mockDecoder.startPlayback).toHaveBeenCalledWith(2.5)
  })

  it('does not start when outside clip range', () => {
    const clip = makeVideoClip({ startTime: 5000 })
    const index = makeIndex([['node-1', [clip]]])

    startVideoPlaybackMB(mockCanvas, 1000, index)

    expect(mockDecoder.startPlayback).not.toHaveBeenCalled()
  })
})

describe('stopVideoPlaybackMB', () => {
  it('stops all video decoders in index', () => {
    const index = makeIndex([
      ['node-1', [makeVideoClip({ id: 'vc-1' })]],
      ['node-2', [makeVideoClip({ id: 'vc-2' })]],
    ])

    stopVideoPlaybackMB(index)

    expect(mockDecoder.stopPlayback).toHaveBeenCalledTimes(2)
  })

  it('skips nodes with only animation clips', () => {
    const animClip = {
      id: 'ac-1',
      kind: 'animation' as const,
      startTime: 0,
      duration: 1000,
      keyframes: [],
    }
    const index = makeIndex([['node-1', [animClip as AnimationClip]]])

    stopVideoPlaybackMB(index)

    expect(mockDecoder.stopPlayback).not.toHaveBeenCalled()
  })
})
