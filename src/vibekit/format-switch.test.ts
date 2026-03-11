import { describe, it, expect, vi, beforeEach } from 'vitest'
import { LINKEDIN_CAROUSEL, LINKEDIN_VIDEO } from './format-presets'

const mockUpdateNode = vi.fn()
const mockStartBatch = vi.fn()
const mockEndBatch = vi.fn()
const mockSetActiveFormat = vi.fn()

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      document: {
        version: '1',
        children: [],
        pages: [
          {
            id: 'page-1',
            name: 'Page 1',
            children: [
              {
                id: 'root-frame',
                type: 'frame',
                width: 1080,
                height: 1350,
                children: [
                  { id: 'text-1', type: 'text', x: 100, y: 200, fontSize: 48, content: 'Hello' },
                  { id: 'fill-child', type: 'frame', width: 'fill_container', height: 'fill_container', children: [] },
                ],
              },
            ],
          },
        ],
      },
      updateNode: mockUpdateNode,
    }),
  },
}))

vi.mock('@/stores/history-store', () => ({
  useHistoryStore: {
    getState: () => ({
      startBatch: mockStartBatch,
      endBatch: mockEndBatch,
    }),
  },
}))

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: {
    getState: () => ({
      activeFormat: LINKEDIN_CAROUSEL,
      setActiveFormat: mockSetActiveFormat,
    }),
  },
}))

vi.mock('@/canvas/canvas-sync-utils', () => ({
  forcePageResync: vi.fn(),
}))

import { switchFormat } from './format-switch'

describe('switchFormat', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('wraps changes in a history batch', () => {
    switchFormat(LINKEDIN_VIDEO)
    expect(mockStartBatch).toHaveBeenCalledTimes(1)
    expect(mockEndBatch).toHaveBeenCalledTimes(1)
  })

  it('resizes root frame to new dimensions', () => {
    switchFormat(LINKEDIN_VIDEO)
    const rootUpdate = mockUpdateNode.mock.calls.find(
      (c: unknown[]) => c[0] === 'root-frame',
    )
    expect(rootUpdate).toBeDefined()
    expect(rootUpdate![1]).toMatchObject({ width: 1080, height: 1920 })
  })

  it('scales fixed-size child positions', () => {
    switchFormat(LINKEDIN_VIDEO)
    const textUpdate = mockUpdateNode.mock.calls.find(
      (c: unknown[]) => c[0] === 'text-1',
    )
    expect(textUpdate).toBeDefined()
    // scaleX = 1080/1080 = 1, scaleY = 1920/1350 ≈ 1.422
    expect(textUpdate![1].x).toBe(100) // no change (scaleX = 1)
    expect(textUpdate![1].y).toBe(Math.round(200 * (1920 / 1350)))
  })

  it('does NOT scale fill_container children width/height', () => {
    switchFormat(LINKEDIN_VIDEO)
    const fillUpdate = mockUpdateNode.mock.calls.find(
      (c: unknown[]) => c[0] === 'fill-child',
    )
    // fill_container has string width/height, so walkAndScale should NOT update them
    expect(fillUpdate).toBeUndefined()
  })

  it('updates the active format in canvas store', () => {
    switchFormat(LINKEDIN_VIDEO)
    expect(mockSetActiveFormat).toHaveBeenCalledWith(LINKEDIN_VIDEO)
  })

  it('skips when switching to same dimensions', () => {
    switchFormat(LINKEDIN_CAROUSEL)
    expect(mockStartBatch).not.toHaveBeenCalled()
    expect(mockSetActiveFormat).toHaveBeenCalledWith(LINKEDIN_CAROUSEL)
  })
})
