/**
 * Runtime storage for video File objects.
 *
 * VideoNode is a serializable type — File objects can't go on it.
 * This module stores File references keyed by node ID, enabling
 * MediaBunny to re-create decoders (e.g. after page switch).
 */

const fileStore = new Map<string, File>()

export function storeVideoFile(nodeId: string, file: File): void {
  fileStore.set(nodeId, file)
}

export function getVideoFile(nodeId: string): File | undefined {
  return fileStore.get(nodeId)
}

export function removeVideoFile(nodeId: string): void {
  fileStore.delete(nodeId)
}

/** Copy a File reference when duplicating a video node. */
export function copyVideoFile(sourceNodeId: string, targetNodeId: string): void {
  const file = fileStore.get(sourceNodeId)
  if (file) {
    fileStore.set(targetNodeId, file)
  }
}
