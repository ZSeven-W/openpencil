/**
 * React hook bridge for the v2 PlaybackController.
 *
 * - Global singleton controller, lazy-created on first play
 * - Two hooks: usePlaybackTime (re-renders ~30fps) and usePlaybackPlaying (re-renders on state transitions)
 * - useSyncExternalStore for tear-free concurrent rendering
 */

import { useSyncExternalStore } from 'react'
import {
  createPlaybackController,
  type PlaybackController,
} from '@/animation/playback-controller'
import { buildAnimationIndex, type AnimationIndex } from '@/animation/animation-index'
import { interpolateClip } from '@/animation/interpolation'
import {
  applyAnimatedFrame,
  captureNodeState,
  buildFabricObjectMap,
  clearFabricObjectMap,
  restoreNodeStates,
  recalcCoordsForAnimatedObjects,
  findFabricObject,
} from '@/animation/canvas-bridge'
import { syncVideoFramesV2, pauseAllVideosV2 } from '@/animation/video-sync'
import { setPlaybackControllerRef as setPauseMiddlewareRef } from '@/stores/animation-pause-middleware'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useTimelineStore } from '@/stores/timeline-store'
import type { Canvas, FabricObject } from 'fabric'
import type { AnimatableValue } from '@/types/animation'
import { isVideoClip } from '@/types/animation'
import type { PenNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Singleton state
// ---------------------------------------------------------------------------

let controllerRef: PlaybackController | null = null

/** Mutable refs updated before each play — closures read these */
let activeIndex: AnimationIndex = { clipsByNode: new Map(), animatedNodes: new Set(), version: 0 }
let activeSavedStates = new Map<string, Record<string, AnimatableValue>>()
let activeCanvas: Canvas | null = null

export function getPlaybackController(): PlaybackController | null {
  return controllerRef
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getPageChildren(): PenNode[] {
  const doc = useDocumentStore.getState().document
  const activePageId = useCanvasStore.getState().activePageId
  if (activePageId && doc.pages) {
    const page = doc.pages.find((p) => p.id === activePageId)
    if (page) return page.children
  }
  return doc.children
}

// ---------------------------------------------------------------------------
// Controller factory — created once, reused across play/pause cycles
// ---------------------------------------------------------------------------

function ensureController(): PlaybackController {
  if (controllerRef) return controllerRef

  const canvas = useCanvasStore.getState().fabricCanvas
  if (!canvas) {
    throw new Error('Cannot create PlaybackController: no Fabric canvas mounted')
  }

  const ctrl = createPlaybackController({
    composition: {
      duration: useTimelineStore.getState().getCompositionDuration(),
      fps: useTimelineStore.getState().getCompositionFps(),
    },
    getIndex: () => activeIndex,
    onFrame(timeMs: number) {
      if (!activeCanvas) return

      // Interpolate all animation clips and apply to canvas
      for (const [nodeId, clips] of activeIndex.clipsByNode) {
        for (const clip of clips) {
          if (isVideoClip(clip)) continue
          const values = interpolateClip(clip, timeMs)
          if (!values) continue
          const obj = findFabricObject(activeCanvas, nodeId)
          if (obj) applyAnimatedFrame(obj as FabricObject, values)
        }
      }

      // Sync video clips
      syncVideoFramesV2(activeCanvas, timeMs, activeIndex)

      // Single render call per frame — renderAll (not requestRenderAll) to avoid 1-frame lag
      activeCanvas.renderAll()
    },
    onStop() {
      if (!activeCanvas) return

      // Stop time polling before any store updates
      stopTimePolling()

      // Restore original node states
      restoreNodeStates(activeCanvas, activeSavedStates)
      recalcCoordsForAnimatedObjects()
      pauseAllVideosV2(activeIndex)
      clearFabricObjectMap()
      activeSavedStates.clear()

      // Update timeline store
      useTimelineStore.getState().setPlaybackMode('idle')
      useTimelineStore.getState().setCurrentTime(0)
    },
    loopEnabled: useTimelineStore.getState().loopEnabled,
  })

  // Bridge state transitions (play/pause/stop) to React.
  // We do NOT forward every frame tick — time is polled by the RAF loop below.
  let lastPlaying = false
  ctrl.subscribe(() => {
    const nowPlaying = ctrl.isPlaying()
    if (nowPlaying !== lastPlaying) {
      lastPlaying = nowPlaying
      notifyGlobalListeners()
    }
  })

  setPauseMiddlewareRef(ctrl)
  controllerRef = ctrl
  return ctrl
}

// ---------------------------------------------------------------------------
// Imperative API — called by UI components
// ---------------------------------------------------------------------------

export function playV2(): void {
  const canvas = useCanvasStore.getState().fabricCanvas
  if (!canvas) return

  const ctrl = ensureController()
  if (ctrl.isPlaying()) return

  // Update mutable refs before play
  activeCanvas = canvas
  activeIndex = buildAnimationIndex(getPageChildren())

  // Build Fabric object map for O(1) lookups during playback
  buildFabricObjectMap(canvas)

  // Capture current node states for restore on stop
  activeSavedStates.clear()
  for (const nodeId of activeIndex.animatedNodes) {
    const obj = findFabricObject(canvas, nodeId)
    if (obj) activeSavedStates.set(nodeId, captureNodeState(obj as FabricObject))
  }

  useTimelineStore.getState().setPlaybackMode('playing')
  ctrl.play()
  startTimePolling()
}

export function pauseV2(): void {
  if (!controllerRef) return
  controllerRef.pause()
  stopTimePolling()
  useTimelineStore.getState().setPlaybackMode('idle')
}

export function stopV2(): void {
  if (!controllerRef) return
  controllerRef.stop()
  stopTimePolling()
  // onStop callback handles timeline store updates
}

export function seekToV2(timeMs: number): void {
  if (!controllerRef) return
  controllerRef.seekTo(timeMs)
  useTimelineStore.getState().setCurrentTime(timeMs)
}

export function isPlayingV2(): boolean {
  return controllerRef?.isPlaying() ?? false
}

export function disposeController(): void {
  if (controllerRef) {
    stopTimePolling()
    controllerRef.dispose()
    setPauseMiddlewareRef(null)
    controllerRef = null
    activeCanvas = null
    activeSavedStates.clear()
  }
}

// ---------------------------------------------------------------------------
// useSyncExternalStore hooks
// ---------------------------------------------------------------------------

// Global listener set — survives controller create/dispose cycles.
// Only notified on state transitions (play/pause/stop), NOT every frame.
const globalListeners = new Set<() => void>()

/** Called on play/pause/stop transitions only. */
export function notifyGlobalListeners(): void {
  for (const cb of globalListeners) cb()
}

function subscribe(cb: () => void): () => void {
  globalListeners.add(cb)
  return () => { globalListeners.delete(cb) }
}

function getPlayingSnapshot(): boolean {
  return controllerRef?.isPlaying() ?? false
}

/** Whether playback is active. Re-renders only on play/pause/stop transitions. */
export function usePlaybackPlaying(): boolean {
  return useSyncExternalStore(subscribe, getPlayingSnapshot, () => false)
}

// ---------------------------------------------------------------------------
// Time polling — separate RAF loop at ~30fps for time display
// ---------------------------------------------------------------------------

const timeListeners = new Set<() => void>()
let timePollingRafId: number | null = null
let lastPolledTime = 0

function pollTime(): void {
  const t = controllerRef?.currentTime ?? 0
  if (t !== lastPolledTime) {
    lastPolledTime = t
    for (const cb of timeListeners) cb()
  }
  if (controllerRef?.isPlaying()) {
    timePollingRafId = requestAnimationFrame(pollTime)
  } else {
    timePollingRafId = null
  }
}

/** Start polling when playback begins, stop when it ends. */
function startTimePolling(): void {
  if (timePollingRafId !== null) return
  timePollingRafId = requestAnimationFrame(pollTime)
}

function stopTimePolling(): void {
  if (timePollingRafId !== null) {
    cancelAnimationFrame(timePollingRafId)
    timePollingRafId = null
  }
  // One final notify so subscribers see the stopped time
  lastPolledTime = controllerRef?.currentTime ?? 0
  for (const cb of timeListeners) cb()
}

function subscribeTime(cb: () => void): () => void {
  timeListeners.add(cb)
  return () => { timeListeners.delete(cb) }
}

function getTimeSnapshot(): number {
  return lastPolledTime
}

/** Playback time in ms. Re-renders via polling RAF loop while playing. */
export function usePlaybackTime(): number {
  return useSyncExternalStore(subscribeTime, getTimeSnapshot, () => 0)
}
