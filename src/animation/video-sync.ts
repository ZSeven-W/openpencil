/**
 * Video ↔ timeline synchronization.
 *
 * During playback, video elements are driven frame-by-frame:
 * - We DON'T call video.play() — that would cause audio/desync issues
 * - Instead, we set video.currentTime each frame so Fabric re-renders
 *   the video element as a texture source
 *
 * For real-time playback (not scrubbing), we let the browser play the
 * video natively and just mark dirty on the Fabric object so it
 * re-reads the video texture each animation frame.
 */

import type { Canvas } from 'fabric'
import {
  getAllVideoElements,
  getVideoElement,
  seekVideoToTime,
} from '@/animation/video-registry'
import { findFabricObject } from '@/animation/canvas-bridge'
import { useDocumentStore } from '@/stores/document-store'
import type { AnimationIndex } from '@/animation/animation-index'
import type { VideoClipData } from '@/types/animation'
import { isVideoClip } from '@/types/animation'

/**
 * Sync all video elements to the current composition time.
 * Called every frame during playback and on seek.
 *
 * Uses video clips on nodes (VideoClipData) for timing — no longer reads
 * from VideoNode.timelineOffset/inPoint/outPoint.
 */
export function syncVideoFrames(canvas: Canvas, compositionTimeMs: number): void {
  const videoElements = getAllVideoElements()
  if (videoElements.size === 0) return

  const getNodeById = useDocumentStore.getState().getNodeById

  for (const [nodeId] of videoElements) {
    const node = getNodeById(nodeId)
    if (!node || node.type !== 'video') continue

    // Find the first video clip on this node
    const videoClip = node.clips?.find(isVideoClip)
    if (!videoClip) continue

    const clipStart = videoClip.startTime
    const clipEnd = videoClip.startTime + videoClip.duration

    // Only show video when composition time is within the clip range
    const fabricObj = findFabricObject(canvas, nodeId)

    if (compositionTimeMs < clipStart || compositionTimeMs > clipEnd) {
      if (fabricObj && fabricObj.visible) {
        fabricObj.visible = false
        fabricObj.dirty = true
      }
      continue
    }

    // Inside clip range — show and seek
    if (fabricObj && !fabricObj.visible) {
      fabricObj.visible = true
    }

    // Map composition time to source video time
    const clipLocalTime = compositionTimeMs - clipStart
    const sourceRange = videoClip.sourceEnd - videoClip.sourceStart
    const clipProgress = videoClip.duration > 0 ? clipLocalTime / videoClip.duration : 0
    const videoTimeMs = videoClip.sourceStart + clipProgress * sourceRange
    seekVideoToTime(nodeId, videoTimeMs, 0)

    if (fabricObj) {
      fabricObj.dirty = true
    }
  }
}

/**
 * Pause all video elements (called on playback pause/stop).
 */
export function pauseAllVideos(): void {
  for (const videoEl of getAllVideoElements().values()) {
    videoEl.pause()
  }
}

// ============================================================
// v2: AnimationIndex-driven video sync
// ============================================================

const DRIFT_THRESHOLD_MS = 50

function mapClipToSourceTimeSec(clip: VideoClipData, clipLocalTime: number): number {
  const sourceRange = clip.sourceEnd - clip.sourceStart
  const clipProgress = clipLocalTime / clip.duration
  return (clip.sourceStart + clipProgress * sourceRange) / 1000
}

function syncSingleVideoClip(
  canvas: Canvas,
  nodeId: string,
  clip: VideoClipData,
  currentTimeMs: number,
): void {
  const video = getVideoElement(nodeId)
  if (!video) return

  const clipLocalTime = currentTimeMs - clip.startTime

  // Outside clip bounds — pause video
  if (clipLocalTime < 0 || clipLocalTime > clip.duration) {
    if (!video.paused) video.pause()
    return
  }

  const expectedSec = mapClipToSourceTimeSec(clip, clipLocalTime)

  // Set playback rate
  if (video.playbackRate !== clip.playbackRate) {
    video.playbackRate = clip.playbackRate
  }

  // Drift correction: native play + correct only when drift exceeds threshold
  const driftMs = Math.abs(video.currentTime - expectedSec) * 1000
  if (driftMs > DRIFT_THRESHOLD_MS) {
    video.currentTime = expectedSec
  }

  // Ensure playing
  if (video.paused) {
    video.play().catch((e: unknown) => {
      if (e instanceof DOMException && e.name === 'AbortError') return
      throw e
    })
  }

  // Mark Fabric object dirty to re-read video texture
  const obj = findFabricObject(canvas, nodeId)
  if (obj) {
    obj.dirty = true
  }
}

/**
 * Sync video playback during animation playback.
 * Uses native video.play() with drift correction (not seek-every-frame).
 */
export function syncVideoFramesV2(
  canvas: Canvas,
  currentTimeMs: number,
  index: AnimationIndex,
): void {
  for (const [nodeId, clips] of index.clipsByNode) {
    const videoClips = clips.filter((c): c is VideoClipData => c.kind === 'video')
    for (const clip of videoClips) {
      syncSingleVideoClip(canvas, nodeId, clip, currentTimeMs)
    }
  }
}

/**
 * Seek video to specific time (for scrubbing, not playback).
 * Pauses video and seeks directly.
 */
export function seekVideoClipsV2(
  canvas: Canvas,
  currentTimeMs: number,
  index: AnimationIndex,
): void {
  for (const [nodeId, clips] of index.clipsByNode) {
    const videoClips = clips.filter((c): c is VideoClipData => c.kind === 'video')
    for (const clip of videoClips) {
      const video = getVideoElement(nodeId)
      if (!video) continue

      if (!video.paused) video.pause()

      const clipLocalTime = currentTimeMs - clip.startTime
      if (clipLocalTime < 0 || clipLocalTime > clip.duration) continue

      video.currentTime = mapClipToSourceTimeSec(clip, clipLocalTime)

      const obj = findFabricObject(canvas, nodeId)
      if (obj) obj.dirty = true
    }
  }
}

/**
 * Pause all videos that are in the animation index.
 */
export function pauseAllVideosV2(index: AnimationIndex): void {
  for (const [nodeId, clips] of index.clipsByNode) {
    if (clips.some((c) => c.kind === 'video')) {
      const video = getVideoElement(nodeId)
      if (video && !video.paused) video.pause()
    }
  }
}
