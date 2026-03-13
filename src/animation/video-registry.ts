/**
 * Registry of VideoDecoderHandle instances keyed by PenNode ID.
 *
 * Decoders are created during video import and registered here.
 * The playback loop reads from AnimationIndex (not this registry)
 * to drive sync — this registry is for lifecycle management.
 */

import type { VideoDecoderHandle } from '@/animation/video-decoder'
import { removeVideoFile } from '@/animation/video-file-store'

const MAX_CONCURRENT_DECODERS = 8

const decoders = new Map<string, VideoDecoderHandle>()

/**
 * Register a decoder. Returns false if limit reached (handle is disposed).
 */
export function registerVideoDecoder(nodeId: string, handle: VideoDecoderHandle): boolean {
  if (decoders.size >= MAX_CONCURRENT_DECODERS) {
    console.warn(`[video-registry] Max decoders (${MAX_CONCURRENT_DECODERS}) reached, disposing new handle`)
    handle.dispose()
    return false
  }
  decoders.set(nodeId, handle)
  return true
}

/**
 * Unregister and dispose a decoder + its File reference.
 */
export function unregisterVideoDecoder(nodeId: string): void {
  const handle = decoders.get(nodeId)
  if (handle) {
    handle.dispose()
  }
  decoders.delete(nodeId)
  removeVideoFile(nodeId)
}

export function getVideoDecoder(nodeId: string): VideoDecoderHandle | undefined {
  return decoders.get(nodeId)
}

// ---------------------------------------------------------------------------
// Backward-compat exports — removed in this migration
// ---------------------------------------------------------------------------
// These are kept temporarily so existing imports don't break during
// the multi-file migration. Consumers will be updated to use the new API.

/** @deprecated Use registerVideoDecoder */
export function registerVideoElement(_nodeId: string, _el: HTMLVideoElement): void {
  console.warn('[video-registry] registerVideoElement is deprecated — use registerVideoDecoder')
}

/** @deprecated Use unregisterVideoDecoder */
export function unregisterVideoElement(nodeId: string): void {
  unregisterVideoDecoder(nodeId)
}

/** @deprecated Use getVideoDecoder */
export function getVideoElement(_nodeId: string): HTMLVideoElement | undefined {
  // Return undefined — callers need to be migrated
  return undefined
}

/** @deprecated Removed — sync functions iterate via AnimationIndex */
export function getAllVideoElements(): Map<string, HTMLVideoElement> {
  return new Map()
}

/** @deprecated Use handle.drawFrame(t) */
export function seekVideoToTime(
  _nodeId: string,
  _compositionTimeMs: number,
  _startTimeMs?: number,
): void {
  console.warn('[video-registry] seekVideoToTime is deprecated')
}
