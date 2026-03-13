/**
 * Video ↔ timeline synchronization (MediaBunny WebCodecs).
 *
 * Single-clock architecture: composition RAF loop is sole timing authority.
 * No video.play(), no drift correction, no competing clocks.
 *
 * - syncVideoFramesMB: called every RAF tick during playback (synchronous)
 * - seekVideoFramesMB: called on scrub/seek (async, debounced)
 * - startVideoPlaybackMB: called at play-start
 * - stopVideoPlaybackMB: called at stop/pause
 */

import type { Canvas } from 'fabric'
import { getVideoDecoder } from '@/animation/video-registry'
import { findFabricObject } from '@/animation/canvas-bridge'
import type { AnimationIndex } from '@/animation/animation-index'
import type { VideoClipData } from '@/types/animation'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mapClipToSourceTimeSec(clip: VideoClipData, clipLocalTimeMs: number): number {
  const sourceRange = clip.sourceEnd - clip.sourceStart
  const clipProgress = clip.duration > 0 ? clipLocalTimeMs / clip.duration : 0
  return (clip.sourceStart + clipProgress * sourceRange) / 1000
}

function getVideoClips(index: AnimationIndex): [string, VideoClipData][] {
  const result: [string, VideoClipData][] = []
  for (const [nodeId, clips] of index.clipsByNode) {
    for (const clip of clips) {
      if (clip.kind === 'video') {
        result.push([nodeId, clip as VideoClipData])
      }
    }
  }
  return result
}

// ---------------------------------------------------------------------------
// Playback sync (called every RAF tick — MUST be synchronous)
// ---------------------------------------------------------------------------

/**
 * Advance video frames during playback. Called from onFrame callback.
 * advanceFrame() is synchronous — pre-fetched frames are swapped in.
 */
export function syncVideoFramesMB(
  canvas: Canvas,
  currentTimeMs: number,
  index: AnimationIndex,
): void {
  for (const [nodeId, clip] of getVideoClips(index)) {
    const handle = getVideoDecoder(nodeId)
    if (!handle) continue

    const clipLocalTime = currentTimeMs - clip.startTime
    const obj = findFabricObject(canvas, nodeId)

    // Outside clip bounds — hide
    if (clipLocalTime < 0 || clipLocalTime > clip.duration) {
      if (obj && obj.visible) {
        obj.visible = false
        obj.dirty = true
      }
      continue
    }

    // Inside clip — show
    if (obj && !obj.visible) {
      obj.visible = true
      obj.dirty = true
    }

    const sourceTimeSec = mapClipToSourceTimeSec(clip, clipLocalTime)
    const advanced = handle.advanceFrame(sourceTimeSec)

    if (advanced && obj) {
      obj.dirty = true
    }
  }
}

// ---------------------------------------------------------------------------
// Scrub / seek (async, debounced)
// ---------------------------------------------------------------------------

let pendingSeek: Promise<void> | null = null
let latestSeekArgs: { canvas: Canvas; timeMs: number; index: AnimationIndex } | null = null

/**
 * Seek video frames to a specific time. For scrubbing — async.
 * Uses "latest wins" debounce: if a seek is in-flight, records the
 * latest requested time and processes it after the current seek resolves.
 */
export async function seekVideoFramesMB(
  canvas: Canvas,
  currentTimeMs: number,
  index: AnimationIndex,
): Promise<void> {
  if (pendingSeek) {
    // Another seek is in-flight — record latest and return
    latestSeekArgs = { canvas, timeMs: currentTimeMs, index }
    return
  }

  const doSeek = async (timeMs: number) => {
    for (const [nodeId, clip] of getVideoClips(index)) {
      const handle = getVideoDecoder(nodeId)
      if (!handle) continue

      const clipLocalTime = timeMs - clip.startTime
      const obj = findFabricObject(canvas, nodeId)

      if (clipLocalTime < 0 || clipLocalTime > clip.duration) {
        if (obj && obj.visible) {
          obj.visible = false
          obj.dirty = true
        }
        continue
      }

      if (obj && !obj.visible) {
        obj.visible = true
      }

      const sourceTimeSec = mapClipToSourceTimeSec(clip, clipLocalTime)
      await handle.drawFrame(sourceTimeSec)

      if (obj) {
        obj.dirty = true
      }
    }
    canvas.requestRenderAll()
  }

  pendingSeek = doSeek(currentTimeMs)
  try {
    await pendingSeek
  } finally {
    pendingSeek = null
  }

  // Process latest if queued
  if (latestSeekArgs) {
    const args = latestSeekArgs
    latestSeekArgs = null
    await seekVideoFramesMB(args.canvas, args.timeMs, args.index)
  }
}

// ---------------------------------------------------------------------------
// Start / stop playback
// ---------------------------------------------------------------------------

/**
 * Start playback for all video decoders in the index.
 * Called once when user clicks Play.
 */
export function startVideoPlaybackMB(
  canvas: Canvas,
  currentTimeMs: number,
  index: AnimationIndex,
): void {
  for (const [nodeId, clip] of getVideoClips(index)) {
    const handle = getVideoDecoder(nodeId)
    if (!handle) continue

    const clipLocalTime = currentTimeMs - clip.startTime
    if (clipLocalTime < 0 || clipLocalTime > clip.duration) continue

    const sourceTimeSec = mapClipToSourceTimeSec(clip, clipLocalTime)
    handle.startPlayback(sourceTimeSec)

    // Ensure visible
    const obj = findFabricObject(canvas, nodeId)
    if (obj && !obj.visible) {
      obj.visible = true
      obj.dirty = true
    }
  }
}

/**
 * Stop playback for all video decoders in the index.
 * Called on pause/stop.
 */
export function stopVideoPlaybackMB(index: AnimationIndex): void {
  for (const [nodeId, clips] of index.clipsByNode) {
    if (clips.some((c) => c.kind === 'video')) {
      const handle = getVideoDecoder(nodeId)
      handle?.stopPlayback()
    }
  }
}

// ---------------------------------------------------------------------------
// Legacy exports — kept for backward compat during migration
// ---------------------------------------------------------------------------

/** @deprecated Use syncVideoFramesMB */
export function syncVideoFrames(): void {}

/** @deprecated Use syncVideoFramesMB */
export function syncVideoFramesV2(
  canvas: Canvas,
  currentTimeMs: number,
  index: AnimationIndex,
): void {
  syncVideoFramesMB(canvas, currentTimeMs, index)
}

/** @deprecated Use seekVideoFramesMB */
export function seekVideoClipsV2(
  canvas: Canvas,
  currentTimeMs: number,
  index: AnimationIndex,
): void {
  seekVideoFramesMB(canvas, currentTimeMs, index)
}

/** @deprecated Use stopVideoPlaybackMB */
export function pauseAllVideos(): void {}

/** @deprecated Use stopVideoPlaybackMB */
export function pauseAllVideosV2(index: AnimationIndex): void {
  stopVideoPlaybackMB(index)
}
