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
  seekVideoToTime,
} from '@/animation/video-registry'
import { findFabricObject } from '@/animation/canvas-bridge'
import { useDocumentStore } from '@/stores/document-store'
import type { VideoNode } from '@/types/pen'

/**
 * Sync all video elements to the current composition time.
 * Called every frame during playback and on seek.
 */
export function syncVideoFrames(canvas: Canvas, compositionTimeMs: number): void {
  const videoElements = getAllVideoElements()
  if (videoElements.size === 0) return

  const getNodeById = useDocumentStore.getState().getNodeById

  for (const [nodeId] of videoElements) {
    const node = getNodeById(nodeId)
    if (!node || node.type !== 'video') continue

    const videoNode = node as VideoNode
    const timelineOffset = videoNode.timelineOffset ?? 0
    const inPoint = videoNode.inPoint ?? 0
    const outPoint = videoNode.outPoint ?? (videoNode.videoDuration ?? Infinity)

    // Calculate where we are within the clip
    const clipStart = timelineOffset
    const clipEnd = timelineOffset + (outPoint - inPoint)

    // Only show video when composition time is within the clip range
    const fabricObj = findFabricObject(canvas, nodeId)

    if (compositionTimeMs < clipStart || compositionTimeMs > clipEnd) {
      // Outside clip range — hide the video
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

    // Map composition time to video time: inPoint + (compositionTime - timelineOffset)
    const videoTimeMs = inPoint + (compositionTimeMs - timelineOffset)
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
