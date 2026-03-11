import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { Canvas } from 'fabric'
import type { AnimationIndex } from '@/animation/animation-index'
import type { VideoClipData, AnimationClip } from '@/types/animation'

vi.mock('@/animation/video-registry', () => ({
  getVideoElement: vi.fn(),
  getAllVideoElements: vi.fn(() => new Map()),
  seekVideoToTime: vi.fn(),
  registerVideoElement: vi.fn(),
  unregisterVideoElement: vi.fn(),
}))

vi.mock('@/animation/canvas-bridge', () => ({
  findFabricObject: vi.fn(),
}))

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: { getState: () => ({ getNodeById: () => null }) },
}))

import { getVideoElement } from '@/animation/video-registry'
import { findFabricObject } from '@/animation/canvas-bridge'
import {
  syncVideoFramesV2,
  seekVideoClipsV2,
  pauseAllVideosV2,
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

function makeMockVideo(overrides: Partial<HTMLVideoElement> = {}) {
  return {
    currentTime: 0,
    paused: true,
    playbackRate: 1,
    play: vi.fn().mockResolvedValue(undefined),
    pause: vi.fn(),
    ...overrides,
  } as unknown as HTMLVideoElement
}

function makeIndex(clips: [string, AnimationClip[]][]): AnimationIndex {
  return {
    clipsByNode: new Map(clips),
    animatedNodes: new Set(clips.map(([id]) => id)),
    version: 1,
  }
}

const mockCanvas = {} as Canvas
const mockFabricObj = { dirty: false, penNodeId: 'node-1' }

beforeEach(() => {
  vi.clearAllMocks()
  mockFabricObj.dirty = false
  vi.mocked(findFabricObject).mockReturnValue(mockFabricObj as any)
})

describe('syncVideoFramesV2', () => {
  it('plays video and marks dirty when in range', () => {
    const video = makeMockVideo({ paused: true, currentTime: 0 })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesV2(mockCanvas, 2500, index)

    expect(video.play).toHaveBeenCalled()
    expect(mockFabricObj.dirty).toBe(true)
  })

  it('pauses video when before clip start', () => {
    const video = makeMockVideo({ paused: false })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip({ startTime: 1000 })
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesV2(mockCanvas, 500, index)

    expect(video.pause).toHaveBeenCalled()
    expect(video.play).not.toHaveBeenCalled()
  })

  it('pauses video when after clip end', () => {
    const video = makeMockVideo({ paused: false })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip({ startTime: 0, duration: 2000 })
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesV2(mockCanvas, 3000, index)

    expect(video.pause).toHaveBeenCalled()
  })

  it('does not seek when drift is under 50ms', () => {
    // Clip: 0-5000ms, source 0-5000ms. At t=2500, expected source = 2.5s
    const video = makeMockVideo({ paused: false, currentTime: 2.48 })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesV2(mockCanvas, 2500, index)

    // Drift = |2.48 - 2.5| * 1000 = 20ms < 50ms — should not seek
    expect(video.currentTime).toBe(2.48)
  })

  it('seeks when drift exceeds 50ms', () => {
    // At t=2500, expected source = 2.5s. Current = 2.0s → drift = 500ms
    const video = makeMockVideo({ paused: false, currentTime: 2.0 })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesV2(mockCanvas, 2500, index)

    expect(video.currentTime).toBe(2.5)
  })

  it('sets playback rate when different from clip', () => {
    const video = makeMockVideo({ paused: true, currentTime: 0, playbackRate: 1 })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip({ playbackRate: 2 })
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesV2(mockCanvas, 1000, index)

    expect(video.playbackRate).toBe(2)
  })

  it('maps source time correctly with offset source range', () => {
    // Clip: startTime=0, duration=4000, sourceStart=2000, sourceEnd=6000
    // At t=2000 (halfway), expected source = 2000 + 0.5*4000 = 4000ms = 4.0s
    const video = makeMockVideo({ paused: true, currentTime: 0 })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip({
      sourceStart: 2000,
      sourceEnd: 6000,
      duration: 4000,
    })
    const index = makeIndex([['node-1', [clip]]])

    syncVideoFramesV2(mockCanvas, 2000, index)

    expect(video.currentTime).toBe(4.0)
  })

  it('skips non-video clips', () => {
    vi.mocked(getVideoElement).mockReturnValue(undefined)

    const animClip = {
      id: 'ac-1',
      kind: 'animation' as const,
      startTime: 0,
      duration: 1000,
      keyframes: [],
    }
    const index = makeIndex([['node-1', [animClip as AnimationClip]]])

    syncVideoFramesV2(mockCanvas, 500, index)

    expect(getVideoElement).not.toHaveBeenCalled()
  })
})

describe('seekVideoClipsV2', () => {
  it('pauses and seeks to correct source time', () => {
    const video = makeMockVideo({ paused: false, currentTime: 0 })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip()
    const index = makeIndex([['node-1', [clip]]])

    seekVideoClipsV2(mockCanvas, 3000, index)

    expect(video.pause).toHaveBeenCalled()
    expect(video.currentTime).toBe(3.0)
    expect(mockFabricObj.dirty).toBe(true)
  })

  it('does not seek when outside clip range', () => {
    const video = makeMockVideo({ paused: false, currentTime: 1.0 })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const clip = makeVideoClip({ startTime: 5000 })
    const index = makeIndex([['node-1', [clip]]])

    seekVideoClipsV2(mockCanvas, 1000, index)

    expect(video.pause).toHaveBeenCalled()
    // currentTime should not have been changed (still 1.0 from mock)
    expect(video.currentTime).toBe(1.0)
    expect(mockFabricObj.dirty).toBe(false)
  })
})

describe('pauseAllVideosV2', () => {
  it('pauses all playing videos in the index', () => {
    const video1 = makeMockVideo({ paused: false })
    const video2 = makeMockVideo({ paused: false })

    vi.mocked(getVideoElement)
      .mockReturnValueOnce(video1)
      .mockReturnValueOnce(video2)

    const index = makeIndex([
      ['node-1', [makeVideoClip({ id: 'vc-1' })]],
      ['node-2', [makeVideoClip({ id: 'vc-2' })]],
    ])

    pauseAllVideosV2(index)

    expect(video1.pause).toHaveBeenCalled()
    expect(video2.pause).toHaveBeenCalled()
  })

  it('skips already paused videos', () => {
    const video = makeMockVideo({ paused: true })
    vi.mocked(getVideoElement).mockReturnValue(video)

    const index = makeIndex([['node-1', [makeVideoClip()]]])

    pauseAllVideosV2(index)

    expect(video.pause).not.toHaveBeenCalled()
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

    pauseAllVideosV2(index)

    expect(getVideoElement).not.toHaveBeenCalled()
  })
})
