import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { createPlaybackController } from './playback-controller'
import type { PlaybackController } from './playback-controller'
import type { CompositionSettings } from '@/types/animation'
import type { AnimationIndex } from './animation-index'

const composition: CompositionSettings = { duration: 5000, fps: 30 }

function emptyIndex(): AnimationIndex {
  return { clipsByNode: new Map(), animatedNodes: new Set(), version: 1 }
}

describe('PlaybackController', () => {
  let controller: PlaybackController
  let onFrame: ReturnType<typeof vi.fn>
  let onStop: ReturnType<typeof vi.fn>
  let rafCallbacks: ((time: number) => void)[]
  let rafId: number
  let nowValue: number

  beforeEach(() => {
    onFrame = vi.fn()
    onStop = vi.fn()
    rafCallbacks = []
    rafId = 0
    nowValue = 0

    vi.spyOn(performance, 'now').mockImplementation(() => nowValue)

    // Polyfill rAF/cAF for non-browser test environments
    globalThis.requestAnimationFrame = vi.fn((cb: FrameRequestCallback) => {
      rafCallbacks.push(cb as (time: number) => void)
      return ++rafId
    })
    globalThis.cancelAnimationFrame = vi.fn(() => {
      rafCallbacks = []
    })
  })

  afterEach(() => {
    controller?.dispose()
    vi.restoreAllMocks()
  })

  function flushRAF(advanceMs: number): void {
    nowValue += advanceMs
    const cbs = [...rafCallbacks]
    rafCallbacks = []
    for (const cb of cbs) cb(nowValue)
  }

  function createController(loopEnabled = false): PlaybackController {
    controller = createPlaybackController({
      composition,
      getIndex: emptyIndex,
      onFrame,
      onStop,
      loopEnabled,
    })
    return controller
  }

  it('starts in paused state', () => {
    createController()
    expect(controller.isPlaying()).toBe(false)
    expect(controller.currentTime).toBe(0)
  })

  it('play transitions to playing', () => {
    createController()
    controller.play()
    expect(controller.isPlaying()).toBe(true)
  })

  it('pause transitions back to paused', () => {
    createController()
    controller.play()
    flushRAF(100)
    controller.pause()
    expect(controller.isPlaying()).toBe(false)
  })

  it('stop resets time to 0 and calls onStop', () => {
    createController()
    controller.play()
    flushRAF(500)
    controller.stop()
    expect(controller.isPlaying()).toBe(false)
    expect(controller.currentTime).toBe(0)
    expect(onStop).toHaveBeenCalledOnce()
  })

  it('stop does not call onStop if already paused', () => {
    createController()
    controller.play()
    flushRAF(100)
    controller.pause()
    onStop.mockClear()
    controller.stop()
    expect(onStop).not.toHaveBeenCalled()
  })

  it('seekTo sets time and calls onFrame', () => {
    createController()
    controller.seekTo(2500)
    expect(controller.currentTime).toBe(2500)
    expect(onFrame).toHaveBeenCalledWith(2500)
  })

  it('seekTo clamps to composition duration', () => {
    createController()
    controller.seekTo(10000)
    expect(controller.currentTime).toBe(5000)
  })

  it('seekTo clamps to 0', () => {
    createController()
    controller.seekTo(-100)
    expect(controller.currentTime).toBe(0)
  })

  it('onFrame called with correct time during playback', () => {
    createController()
    controller.play()
    flushRAF(1000)
    expect(onFrame).toHaveBeenCalledWith(1000)
  })

  it('speed 2x doubles elapsed time', () => {
    createController()
    controller.setSpeed(2)
    controller.play()
    flushRAF(500)
    expect(onFrame).toHaveBeenCalledWith(1000)
  })

  it('speed 0.5x halves elapsed time', () => {
    createController()
    controller.setSpeed(0.5)
    controller.play()
    flushRAF(1000)
    expect(onFrame).toHaveBeenCalledWith(500)
  })

  it('speed clamps to 0.25 min', () => {
    createController()
    controller.setSpeed(0.01)
    controller.play()
    flushRAF(1000)
    expect(onFrame).toHaveBeenCalledWith(250)
  })

  it('speed clamps to 4x max', () => {
    createController()
    controller.setSpeed(10)
    controller.play()
    flushRAF(500)
    expect(onFrame).toHaveBeenCalledWith(2000)
  })

  it('speed change during playback does not jump position', () => {
    createController()
    controller.play()
    flushRAF(1000) // at 1000ms
    controller.setSpeed(2)
    flushRAF(500) // 500ms * 2x = 1000ms more
    expect(onFrame).toHaveBeenLastCalledWith(2000)
  })

  it('subscribe/getSnapshot work for useSyncExternalStore', () => {
    createController()
    const listener = vi.fn()
    const unsub = controller.subscribe(listener)

    controller.play()
    expect(listener).toHaveBeenCalled()

    listener.mockClear()
    flushRAF(100)
    expect(listener).toHaveBeenCalled()
    expect(controller.getSnapshot()).toBe(100)

    unsub()
    listener.mockClear()
    flushRAF(100)
    expect(listener).not.toHaveBeenCalled()
  })

  it('stops at composition end without loop', () => {
    createController(false)
    controller.play()
    flushRAF(5000)
    expect(controller.isPlaying()).toBe(false)
    expect(controller.currentTime).toBe(5000)
    expect(onStop).toHaveBeenCalledOnce()
  })

  it('loops at composition end with loop enabled', () => {
    createController(true)
    controller.play()
    flushRAF(5000)
    // Should still be playing after loop
    expect(controller.isPlaying()).toBe(true)
    expect(onStop).not.toHaveBeenCalled()

    // Next frame should be near 0
    flushRAF(100)
    expect(onFrame).toHaveBeenLastCalledWith(100)
  })

  it('play from end wraps to start', () => {
    createController(false)
    controller.seekTo(5000)
    controller.play()
    expect(controller.isPlaying()).toBe(true)
    flushRAF(500)
    expect(onFrame).toHaveBeenLastCalledWith(500)
  })

  it('dispose cleans up rAF and listeners', () => {
    createController()
    const listener = vi.fn()
    controller.subscribe(listener)
    controller.play()
    controller.dispose()

    expect(cancelAnimationFrame).toHaveBeenCalled()
    listener.mockClear()
    // After dispose, play should be no-op
    controller.play()
    expect(listener).not.toHaveBeenCalled()
  })

  it('multiple play calls are idempotent', () => {
    createController()
    controller.play()
    controller.play()
    // Only one rAF should be queued
    expect(requestAnimationFrame).toHaveBeenCalledTimes(1)
  })
})
