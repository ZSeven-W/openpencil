import type { AnimationIndex } from './animation-index'
import type { TrackBuffer } from './interpolation'
import { createTrackBuffer } from './interpolation'

export interface TrackBufferMap {
  buffers: Map<string, TrackBuffer>
}

export function createTrackBufferMap(index: AnimationIndex): TrackBufferMap {
  const buffers = new Map<string, TrackBuffer>()
  for (const nodeId of index.animatedNodes) {
    buffers.set(nodeId, createTrackBuffer())
  }
  return { buffers }
}

export function getOrCreateBuffer(
  map: TrackBufferMap,
  nodeId: string,
): TrackBuffer {
  let buffer = map.buffers.get(nodeId)
  if (!buffer) {
    buffer = createTrackBuffer()
    map.buffers.set(nodeId, buffer)
  }
  return buffer
}
