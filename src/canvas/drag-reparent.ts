import { useDocumentStore } from '@/stores/document-store'
import { setFabricSyncLock } from './canvas-sync-lock'
import { rootFrameBounds } from './use-canvas-sync'
import type { FabricObjectWithPenId } from './canvas-object-factory'

interface Bounds {
  x: number
  y: number
  w: number
  h: number
}

function isCompletelyOutside(obj: Bounds, parent: Bounds): boolean {
  return (
    obj.x + obj.w <= parent.x ||
    obj.x >= parent.x + parent.w ||
    obj.y + obj.h <= parent.y ||
    obj.y >= parent.y + parent.h
  )
}

function overlapArea(a: Bounds, b: Bounds): number {
  const overlapX = Math.max(
    0,
    Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x),
  )
  const overlapY = Math.max(
    0,
    Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y),
  )
  return overlapX * overlapY
}

/**
 * Check if a Fabric object was dragged outside its parent root frame.
 * If so, reparent it to the overlapping root frame or to the root level.
 *
 * Only triggers for direct children of root frames (MVP scope).
 * Returns true if reparenting occurred.
 */
export function checkDragReparent(obj: FabricObjectWithPenId): boolean {
  const nodeId = obj.penNodeId
  if (!nodeId) return false

  const store = useDocumentStore.getState()
  const parent = store.getParentOf(nodeId)
  if (!parent) return false // Already root-level

  // Only handle direct children of root frames
  const parentBounds = rootFrameBounds.get(parent.id)
  if (!parentBounds) return false // Parent is not a root frame

  // Compute object's absolute bounds from Fabric
  const objBounds: Bounds = {
    x: obj.left ?? 0,
    y: obj.top ?? 0,
    w: (obj.width ?? 0) * (obj.scaleX ?? 1),
    h: (obj.height ?? 0) * (obj.scaleY ?? 1),
  }

  // Only trigger if completely outside parent
  if (!isCompletelyOutside(objBounds, parentBounds)) return false

  // Find the root frame with the most overlap
  let bestFrameId: string | null = null
  let bestOverlap = 0

  for (const [frameId, frameBounds] of rootFrameBounds) {
    if (frameId === parent.id) continue // Skip current parent
    const area = overlapArea(objBounds, frameBounds)
    if (area > bestOverlap) {
      bestOverlap = area
      bestFrameId = frameId
    }
  }

  setFabricSyncLock(true)
  try {
    if (bestFrameId) {
      // Reparent into the overlapping frame — convert absolute to relative position
      const targetBounds = rootFrameBounds.get(bestFrameId)!
      store.updateNode(nodeId, {
        x: objBounds.x - targetBounds.x,
        y: objBounds.y - targetBounds.y,
      })
      const targetChildren = store.getNodeById(bestFrameId)
      const childCount =
        targetChildren && 'children' in targetChildren && targetChildren.children
          ? targetChildren.children.length
          : 0
      store.moveNode(nodeId, bestFrameId, childCount)
    } else {
      // No overlapping frame — make it a root-level node
      store.updateNode(nodeId, {
        x: objBounds.x,
        y: objBounds.y,
      })
      const rootCount = store.document.children.length
      store.moveNode(nodeId, null, rootCount)
    }
  } finally {
    setFabricSyncLock(false)
  }

  return true
}
