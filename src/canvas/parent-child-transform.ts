import * as fabric from 'fabric'
import { useDocumentStore } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'

// ---------------------------------------------------------------------------
// Drag session state
// ---------------------------------------------------------------------------

interface InitialPosition {
  left: number
  top: number
  width: number
  height: number
  angle: number
}

interface DragSession {
  parentId: string
  /** All recursive descendant node IDs */
  descendantIds: Set<string>
  /** Initial absolute Fabric positions of parent + all descendants */
  initialPositions: Map<string, InitialPosition>
  /** Cached Fabric object references for fast lookup during drag */
  descendantObjects: Map<string, FabricObjectWithPenId>
}

let activeDragSession: DragSession | null = null

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Recursively collect all descendant node IDs from the document tree. */
export function collectDescendantIds(nodeId: string): Set<string> {
  const result = new Set<string>()
  const node = useDocumentStore.getState().getNodeById(nodeId)
  if (!node) return result

  const recurse = (children: PenNode[] | undefined) => {
    if (!children) return
    for (const child of children) {
      result.add(child.id)
      if ('children' in child && child.children) {
        recurse(child.children)
      }
    }
  }

  if ('children' in node && node.children) {
    recurse(node.children)
  }
  return result
}

/** Check if a node is a container with at least one child. */
function hasChildren(nodeId: string): boolean {
  const node = useDocumentStore.getState().getNodeById(nodeId)
  if (!node) return false
  return (
    'children' in node &&
    Array.isArray(node.children) &&
    node.children.length > 0
  )
}

// ---------------------------------------------------------------------------
// Session lifecycle
// ---------------------------------------------------------------------------

/**
 * Called on mouse:down when the target might be a container.
 * Collects descendant IDs and caches initial Fabric positions.
 * Returns null if the target has no children.
 */
export function beginParentDrag(
  parentId: string,
  canvas: fabric.Canvas,
): DragSession | null {
  if (!hasChildren(parentId)) {
    activeDragSession = null
    return null
  }

  const descendantIds = collectDescendantIds(parentId)
  if (descendantIds.size === 0) {
    activeDragSession = null
    return null
  }

  // Filter out descendants that are part of an ActiveSelection
  // (Fabric already moves those; we'd double-move them otherwise)
  const activeObj = canvas.getActiveObject()
  if (activeObj && 'getObjects' in activeObj) {
    const selectionIds = new Set(
      (activeObj as fabric.ActiveSelection)
        .getObjects()
        .map((o) => (o as FabricObjectWithPenId).penNodeId)
        .filter(Boolean) as string[],
    )
    for (const id of selectionIds) {
      descendantIds.delete(id)
    }
  }

  if (descendantIds.size === 0) {
    activeDragSession = null
    return null
  }

  // Cache initial positions and Fabric object references
  const initialPositions = new Map<string, InitialPosition>()
  const descendantObjects = new Map<string, FabricObjectWithPenId>()
  const objects = canvas.getObjects() as FabricObjectWithPenId[]

  for (const obj of objects) {
    if (!obj.penNodeId) continue

    if (obj.penNodeId === parentId || descendantIds.has(obj.penNodeId)) {
      initialPositions.set(obj.penNodeId, {
        left: obj.left ?? 0,
        top: obj.top ?? 0,
        width: (obj.width ?? 0) * (obj.scaleX ?? 1),
        height: (obj.height ?? 0) * (obj.scaleY ?? 1),
        angle: obj.angle ?? 0,
      })
    }

    if (descendantIds.has(obj.penNodeId)) {
      descendantObjects.set(obj.penNodeId, obj)
    }
  }

  activeDragSession = {
    parentId,
    descendantIds,
    initialPositions,
    descendantObjects,
  }
  return activeDragSession
}

/** Clear the active session. */
export function endParentDrag(): void {
  activeDragSession = null
}

/** Returns the active session, if any. */
export function getActiveDragSession(): DragSession | null {
  return activeDragSession
}

// ---------------------------------------------------------------------------
// Move propagation
// ---------------------------------------------------------------------------

/**
 * Move all descendant Fabric objects by the same delta as the parent.
 * Called on each `object:moving` frame for visual feedback.
 * No store sync needed — children's relative positions don't change.
 */
export function moveDescendants(
  parentObj: FabricObjectWithPenId,
  _canvas: fabric.Canvas,
): void {
  if (!activeDragSession) return

  const initParent = activeDragSession.initialPositions.get(
    activeDragSession.parentId,
  )
  if (!initParent) return

  const deltaX = (parentObj.left ?? 0) - initParent.left
  const deltaY = (parentObj.top ?? 0) - initParent.top

  for (const [nodeId, obj] of activeDragSession.descendantObjects) {
    const initPos = activeDragSession.initialPositions.get(nodeId)
    if (!initPos) continue

    obj.set({
      left: initPos.left + deltaX,
      top: initPos.top + deltaY,
    })
    obj.setCoords()
  }
}

// ---------------------------------------------------------------------------
// Scale propagation
// ---------------------------------------------------------------------------

/**
 * Reposition and scale all descendant Fabric objects proportionally
 * relative to the parent's scale. Called on each `object:scaling` frame.
 */
export function scaleDescendants(
  parentObj: FabricObjectWithPenId,
  _canvas: fabric.Canvas,
): void {
  if (!activeDragSession) return

  const initParent = activeDragSession.initialPositions.get(
    activeDragSession.parentId,
  )
  if (!initParent) return

  const parentScaleX = parentObj.scaleX ?? 1
  const parentScaleY = parentObj.scaleY ?? 1
  const parentLeft = parentObj.left ?? 0
  const parentTop = parentObj.top ?? 0

  for (const [nodeId, obj] of activeDragSession.descendantObjects) {
    const initPos = activeDragSession.initialPositions.get(nodeId)
    if (!initPos) continue

    // Child's initial offset from parent
    const relativeX = initPos.left - initParent.left
    const relativeY = initPos.top - initParent.top

    obj.set({
      left: parentLeft + relativeX * parentScaleX,
      top: parentTop + relativeY * parentScaleY,
      scaleX: parentScaleX,
      scaleY: parentScaleY,
    })
    obj.setCoords()
  }
}

// ---------------------------------------------------------------------------
// Rotation propagation
// ---------------------------------------------------------------------------

/**
 * Rotate all descendant Fabric objects around the parent's center.
 * Called on each `object:rotating` frame for visual feedback.
 *
 * In Fabric.js with originX:'left', originY:'top', the center of an object
 * is always at (left + width/2, top + height/2) regardless of angle.
 * Rotation happens visually around this center, and left/top stay unchanged
 * when only the angle changes.
 */
export function rotateDescendants(
  parentObj: FabricObjectWithPenId,
  _canvas: fabric.Canvas,
): void {
  if (!activeDragSession) return

  const initParent = activeDragSession.initialPositions.get(
    activeDragSession.parentId,
  )
  if (!initParent) return

  const deltaAngle = (parentObj.angle ?? 0) - initParent.angle
  const deltaRad = (deltaAngle * Math.PI) / 180
  const cos = Math.cos(deltaRad)
  const sin = Math.sin(deltaRad)

  // Parent center (stays fixed during Fabric rotation)
  const pcx = initParent.left + initParent.width / 2
  const pcy = initParent.top + initParent.height / 2

  for (const [nodeId, obj] of activeDragSession.descendantObjects) {
    const initPos = activeDragSession.initialPositions.get(nodeId)
    if (!initPos) continue

    // Rotate child center around parent center
    const ccx = initPos.left + initPos.width / 2
    const ccy = initPos.top + initPos.height / 2
    const dx = ccx - pcx
    const dy = ccy - pcy
    const newCx = pcx + dx * cos - dy * sin
    const newCy = pcy + dx * sin + dy * cos

    // Convert center back to left/top
    const halfW = (obj.width ?? 0) * (obj.scaleX ?? 1) / 2
    const halfH = (obj.height ?? 0) * (obj.scaleY ?? 1) / 2

    obj.set({
      left: newCx - halfW,
      top: newCy - halfH,
      angle: initPos.angle + deltaAngle,
    })
    obj.setCoords()
  }
}

/**
 * After parent rotation: update descendants' relative positions and angles
 * in the document store so they persist correctly.
 */
export function finalizeParentRotation(
  parentObj: FabricObjectWithPenId,
): void {
  if (!activeDragSession) return

  const initParent = activeDragSession.initialPositions.get(
    activeDragSession.parentId,
  )
  if (!initParent) return

  const deltaAngle = (parentObj.angle ?? 0) - initParent.angle
  if (deltaAngle === 0) return

  setFabricSyncLock(true)
  useDocumentStore
    .getState()
    .rotateDescendantsInStore(activeDragSession.parentId, deltaAngle)
  setFabricSyncLock(false)
}

// ---------------------------------------------------------------------------
// Finalize after object:modified
// ---------------------------------------------------------------------------

/**
 * After parent scale is baked into width/height:
 * 1. Bake scale into all descendant Fabric objects
 * 2. Update descendants' relative positions and sizes in the document store
 */
export function finalizeParentTransform(
  _parentObj: FabricObjectWithPenId,
  _canvas: fabric.Canvas,
  effectiveScaleX: number,
  effectiveScaleY: number,
): void {
  if (!activeDragSession) return
  if (effectiveScaleX === 1 && effectiveScaleY === 1) return

  // Bake scale into descendant Fabric objects
  for (const [, obj] of activeDragSession.descendantObjects) {
    const sx = obj.scaleX ?? 1
    const sy = obj.scaleY ?? 1
    if (obj.width !== undefined) {
      obj.set({ width: obj.width * sx, scaleX: 1 })
    }
    if (obj.height !== undefined) {
      obj.set({ height: obj.height * sy, scaleY: 1 })
    }
    obj.setCoords()
  }

  // Update document store: scale all descendants' relative positions and sizes
  setFabricSyncLock(true)
  useDocumentStore
    .getState()
    .scaleDescendantsInStore(
      activeDragSession.parentId,
      effectiveScaleX,
      effectiveScaleY,
    )
  setFabricSyncLock(false)
}
