/**
 * Registry of HTMLVideoElement instances keyed by PenNode ID.
 *
 * Video elements are created when a video node is added to the canvas
 * and cleaned up when the node is removed. The playback loop reads
 * from this registry to sync each video's currentTime with the
 * composition timeline.
 */

const videoElements = new Map<string, HTMLVideoElement>()

export function registerVideoElement(nodeId: string, el: HTMLVideoElement): void {
  videoElements.set(nodeId, el)
}

export function unregisterVideoElement(nodeId: string): void {
  const el = videoElements.get(nodeId)
  if (el) {
    el.pause()
    el.removeAttribute('src')
    el.load() // release memory
  }
  videoElements.delete(nodeId)
}

export function getVideoElement(nodeId: string): HTMLVideoElement | undefined {
  return videoElements.get(nodeId)
}

export function getAllVideoElements(): Map<string, HTMLVideoElement> {
  return videoElements
}

/**
 * Seek a video to a specific composition time.
 * Accounts for the node's startTime offset.
 */
export function seekVideoToTime(
  nodeId: string,
  compositionTimeMs: number,
  startTimeMs: number = 0,
): void {
  const el = videoElements.get(nodeId)
  if (!el) return
  const videoTime = Math.max(0, (compositionTimeMs - startTimeMs) / 1000)
  // Clamp to video duration
  if (el.duration && Number.isFinite(el.duration)) {
    el.currentTime = Math.min(videoTime, el.duration)
  } else {
    el.currentTime = videoTime
  }
}
