import type { Canvas } from 'fabric'
import { useTimelineStore } from '@/stores/timeline-store'
import { getInterpolatedProperties } from '@/animation/interpolation'
import {
  applyAnimatedProperties,
  setPlaybackActive,
  captureCurrentState,
} from '@/animation/canvas-bridge'
import type { AnimatableProperties } from '@/types/animation'

// --- Playback Engine ---

let animationFrameId: number | null = null
let startTimestamp: number | null = null
let pausedAt = 0

// Saved states for restoring canvas after playback
let savedStates: Record<string, AnimatableProperties> = {}

// Throttled UI update ref (external consumers read this)
export let currentPlayheadTime = 0
let lastUiUpdate = 0
const UI_UPDATE_INTERVAL = 100 // ~10fps for UI updates

function tick(canvas: Canvas, timestamp: number) {
  if (startTimestamp === null) startTimestamp = timestamp

  const store = useTimelineStore.getState()
  const elapsed = pausedAt + (timestamp - startTimestamp)
  const { duration, tracks, loopEnabled } = store

  // Check if we've reached the end
  if (elapsed >= duration) {
    if (loopEnabled) {
      startTimestamp = timestamp
      pausedAt = 0
    } else {
      stop(canvas)
      return
    }
  }

  const currentTime = Math.min(elapsed, duration)

  // Interpolate and apply all tracks
  for (const track of Object.values(tracks)) {
    const properties = getInterpolatedProperties(track, currentTime)
    if (properties) {
      applyAnimatedProperties(canvas, track.nodeId, properties)
    }
  }

  // Single render call per frame
  canvas.requestRenderAll()

  // Throttled UI update — avoid Zustand writes every frame
  if (timestamp - lastUiUpdate > UI_UPDATE_INTERVAL) {
    currentPlayheadTime = currentTime
    useTimelineStore.getState().setCurrentTime(currentTime)
    lastUiUpdate = timestamp
  }

  animationFrameId = requestAnimationFrame((ts) => tick(canvas, ts))
}

// --- Public API ---

export function play(canvas: Canvas): void {
  if (animationFrameId !== null) return // already playing

  const store = useTimelineStore.getState()
  if (Object.keys(store.tracks).length === 0) return // nothing to play

  // Save current canvas states before playback
  savedStates = {}
  for (const nodeId of Object.keys(store.tracks)) {
    const state = captureCurrentState(canvas, nodeId)
    if (state) savedStates[nodeId] = state
  }

  // Enter playback mode
  setPlaybackActive(true)
  store.setPlaybackMode('playing')

  pausedAt = store.currentTime
  startTimestamp = null
  lastUiUpdate = 0
  animationFrameId = requestAnimationFrame((ts) => tick(canvas, ts))
}

export function pause(_canvas: Canvas): void {
  if (animationFrameId === null) return

  cancelAnimationFrame(animationFrameId)
  animationFrameId = null

  const store = useTimelineStore.getState()
  pausedAt = currentPlayheadTime
  store.setPlaybackMode('idle')
  setPlaybackActive(false)
}

export function stop(canvas: Canvas): void {
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }

  // Restore original canvas states
  for (const [nodeId, state] of Object.entries(savedStates)) {
    applyAnimatedProperties(canvas, nodeId, state)
  }
  canvas.requestRenderAll()
  savedStates = {}

  const store = useTimelineStore.getState()
  store.setCurrentTime(0)
  store.setPlaybackMode('idle')
  setPlaybackActive(false)
  pausedAt = 0
  startTimestamp = null
  currentPlayheadTime = 0
}

export function seekTo(canvas: Canvas, timeMs: number): void {
  const store = useTimelineStore.getState()
  const clampedTime = Math.max(0, Math.min(timeMs, store.duration))

  // Apply frame at the target time
  for (const track of Object.values(store.tracks)) {
    const properties = getInterpolatedProperties(track, clampedTime)
    if (properties) {
      applyAnimatedProperties(canvas, track.nodeId, properties)
    }
  }
  canvas.requestRenderAll()

  pausedAt = clampedTime
  currentPlayheadTime = clampedTime
  store.setCurrentTime(clampedTime)
}

export function isPlaying(): boolean {
  return animationFrameId !== null
}
