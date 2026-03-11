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
          if (clip.kind === 'video') continue
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
}

export function pauseV2(): void {
  if (!controllerRef) return
  controllerRef.pause()
  useTimelineStore.getState().setPlaybackMode('idle')
}

export function stopV2(): void {
  if (!controllerRef) return
  controllerRef.stop()
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

function subscribeTime(cb: () => void): () => void {
  if (!controllerRef) return () => {}
  return controllerRef.subscribe(cb)
}

function getTimeSnapshot(): number {
  return controllerRef?.currentTime ?? 0
}

/** Playback time in ms. Re-renders at controller's notify rate. */
export function usePlaybackTime(): number {
  return useSyncExternalStore(subscribeTime, getTimeSnapshot, () => 0)
}

function subscribePlaying(cb: () => void): () => void {
  if (!controllerRef) return () => {}
  return controllerRef.subscribe(cb)
}

function getPlayingSnapshot(): boolean {
  return controllerRef?.isPlaying() ?? false
}

/** Whether playback is active. Re-renders only on play/pause/stop transitions. */
export function usePlaybackPlaying(): boolean {
  return useSyncExternalStore(subscribePlaying, getPlayingSnapshot, () => false)
}
