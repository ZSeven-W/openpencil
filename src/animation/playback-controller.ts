import type { CompositionSettings } from '@/types/animation'
import type { AnimationIndex } from './animation-index'
import {
  registerEngine,
  unregisterEngine,
  requestPlayback,
  releasePlayback,
} from '@/animation/engine-coordinator'

export interface PlaybackController {
  play(): void
  pause(): void
  stop(): void
  seekTo(timeMs: number): void
  setSpeed(rate: number): void
  isPlaying(): boolean
  readonly currentTime: number
  subscribe(callback: () => void): () => void
  getSnapshot(): number
  dispose(): void
}

interface PlaybackState {
  playing: boolean
  pausedAt: number
  startTimestamp: number
  speed: number
  rafId: number | null
  listeners: Set<() => void>
  disposed: boolean
}

export function createPlaybackController(opts: {
  composition: CompositionSettings
  getIndex: () => AnimationIndex
  onFrame: (timeMs: number) => void
  onStop: () => void
  loopEnabled?: boolean
}): PlaybackController {
  const { composition, onFrame, onStop } = opts

  const state: PlaybackState = {
    playing: false,
    pausedAt: 0,
    startTimestamp: 0,
    speed: 1,
    rafId: null,
    listeners: new Set(),
    disposed: false,
  }

  // Register this v2 controller with the coordinator so v1 can stop it.
  registerEngine('v2', {
    stop: () => controller.stop(),
    isPlaying: () => state.playing,
  })

  function getCurrentTime(): number {
    if (!state.playing) return state.pausedAt
    const elapsed = (performance.now() - state.startTimestamp) * state.speed
    return Math.min(state.pausedAt + elapsed, composition.duration)
  }

  function notify(): void {
    for (const listener of state.listeners) {
      listener()
    }
  }

  function tick(): void {
    if (state.disposed || !state.playing) return

    const time = getCurrentTime()
    onFrame(time)
    notify()

    if (time >= composition.duration) {
      if (opts.loopEnabled) {
        // Reset to start and continue
        state.pausedAt = 0
        state.startTimestamp = performance.now()
        state.rafId = requestAnimationFrame(tick)
      } else {
        state.playing = false
        state.pausedAt = composition.duration
        state.rafId = null
        onStop()
        notify()
      }
      return
    }

    state.rafId = requestAnimationFrame(tick)
  }

  const controller: PlaybackController = {
    play() {
      if (state.disposed || state.playing) return

      // Ensure only one engine owns the canvas — stops v1 if it's running
      requestPlayback('v2')

      // If at end, wrap to start
      if (state.pausedAt >= composition.duration) {
        state.pausedAt = 0
      }

      state.playing = true
      state.startTimestamp = performance.now()
      state.rafId = requestAnimationFrame(tick)
      notify()
    },

    pause() {
      if (!state.playing) return
      state.pausedAt = getCurrentTime()
      state.playing = false
      if (state.rafId !== null) {
        cancelAnimationFrame(state.rafId)
        state.rafId = null
      }
      releasePlayback('v2')
      notify()
    },

    stop() {
      const wasPlaying = state.playing
      state.playing = false
      state.pausedAt = 0
      if (state.rafId !== null) {
        cancelAnimationFrame(state.rafId)
        state.rafId = null
      }
      releasePlayback('v2')
      if (wasPlaying) {
        onStop()
      }
      notify()
    },

    seekTo(timeMs: number) {
      const clamped = Math.max(0, Math.min(timeMs, composition.duration))
      state.pausedAt = clamped
      if (state.playing) {
        state.startTimestamp = performance.now()
      }
      onFrame(clamped)
      notify()
    },

    setSpeed(rate: number) {
      const clamped = Math.max(0.25, Math.min(4, rate))
      if (state.playing) {
        // Capture current time before speed change
        state.pausedAt = getCurrentTime()
        state.startTimestamp = performance.now()
      }
      state.speed = clamped
      notify()
    },

    isPlaying() {
      return state.playing
    },

    get currentTime() {
      return getCurrentTime()
    },

    subscribe(callback: () => void) {
      state.listeners.add(callback)
      return () => {
        state.listeners.delete(callback)
      }
    },

    getSnapshot() {
      return getCurrentTime()
    },

    dispose() {
      state.disposed = true
      state.playing = false
      if (state.rafId !== null) {
        cancelAnimationFrame(state.rafId)
        state.rafId = null
      }
      releasePlayback('v2')
      unregisterEngine('v2')
      state.listeners.clear()
    },
  }

  return controller
}
