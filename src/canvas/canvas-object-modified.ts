import * as fabric from 'fabric'
import type { PenDocument, PenNode } from '@/types/pen'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'
import { nodeRenderInfo, rebuildNodeRenderInfo } from './use-canvas-sync'
import { clearGuides } from './guide-utils'
import {
  getActiveDragSession,
  finalizeParentTransform,
  finalizeParentRotation,
} from './parent-child-transform'
import {
  endLayoutDrag,
  cancelLayoutDrag,
  isLayoutDragActive,
} from './layout-reorder'
import {
  checkDragReparent,
  checkDragReparentByBounds,
  checkReparentIntoFrame,
} from './drag-reparent'
import {
  commitDragInto,
  cancelDragInto,
  isDragIntoActive,
  commitDragIntoMulti,
} from './drag-into-layout'

/** Absolute bounds for each child computed during syncSelectionToStore. */
export interface ChildAbsBounds {
  nodeId: string
  x: number
  y: number
  w: number
  h: number
}

/** Sync a single Fabric object's transform back to the document store. */
export function syncObjToStore(obj: FabricObjectWithPenId): void {
  if (!obj?.penNodeId) return

  const info = nodeRenderInfo.get(obj.penNodeId)
  const scaleX = obj.scaleX ?? 1
  const scaleY = obj.scaleY ?? 1

  // Convert Fabric absolute position -> document-tree relative position
  const offsetX = info?.parentOffsetX ?? 0
  const offsetY = info?.parentOffsetY ?? 0
  const updates: Partial<PenNode> = {
    x: (obj.left ?? 0) - offsetX,
    y: (obj.top ?? 0) - offsetY,
    rotation: obj.angle ?? 0,
  }

  if (obj.width !== undefined) {
    ;(updates as Record<string, unknown>).width = obj.width * scaleX
  }
  if (obj.height !== undefined) {
    ;(updates as Record<string, unknown>).height = obj.height * scaleY
  }

  setFabricSyncLock(true)
  useDocumentStore.getState().updateNode(obj.penNodeId, updates)
  setFabricSyncLock(false)
}

/**
 * Resolve the absolute position of each child inside an ActiveSelection
 * and sync every child to the document store.
 * Returns an array of absolute bounds (for subsequent reparent checks).
 */
export function syncSelectionToStore(
  target: fabric.FabricObject,
): ChildAbsBounds[] {
  const boundsOut: ChildAbsBounds[] = []
  if (!('getObjects' in target)) return boundsOut
  const group = target as fabric.ActiveSelection
  const groupMatrix = group.calcTransformMatrix()

  setFabricSyncLock(true)
  for (const child of group.getObjects()) {
    const obj = child as FabricObjectWithPenId
    if (!obj.penNodeId) continue

    // Transform the child's local origin into absolute scene coords
    const childMatrix = child.calcOwnMatrix()
    const combined = fabric.util.multiplyTransformMatrices(
      groupMatrix,
      childMatrix,
    )
    // The origin point in local space is (0,0) when originX/Y = 'left'/'top'.
    // For originX:'left', originY:'top' the top-left is at (-width/2, -height/2)
    // relative to the object's own center.
    const halfW =
      ((child.width ?? 0) * (child.scaleX ?? 1)) / 2
    const halfH =
      ((child.height ?? 0) * (child.scaleY ?? 1)) / 2
    const absCenter = fabric.util.transformPoint(
      new fabric.Point(0, 0),
      combined,
    )
    const absLeft = absCenter.x - halfW
    const absTop = absCenter.y - halfH

    const info = nodeRenderInfo.get(obj.penNodeId)
    const offsetX = info?.parentOffsetX ?? 0
    const offsetY = info?.parentOffsetY ?? 0
    const scaleX = child.scaleX ?? 1
    const scaleY = child.scaleY ?? 1

    const absW = (child.width ?? 0) * scaleX
    const absH = (child.height ?? 0) * scaleY

    const updates: Partial<PenNode> = {
      x: absLeft - offsetX,
      y: absTop - offsetY,
      rotation: child.angle ?? 0,
    }
    if (child.width !== undefined) {
      ;(updates as Record<string, unknown>).width = absW
    }
    if (child.height !== undefined) {
      ;(updates as Record<string, unknown>).height = absH
    }

    useDocumentStore.getState().updateNode(obj.penNodeId, updates)
    boundsOut.push({
      nodeId: obj.penNodeId,
      x: absLeft,
      y: absTop,
      w: absW,
      h: absH,
    })
  }
  setFabricSyncLock(false)
  return boundsOut
}

// Re-entry guard for handleObjectModified (module-level state)
let inModifiedHandler = false

/**
 * Handle the object:modified Fabric event.
 * Extracted from use-canvas-events.ts to keep it under 800 lines.
 *
 * @param opt             - The Fabric event options containing the modified target
 * @param canvas          - The Fabric canvas instance
 * @param getPreModDoc    - Getter for the pre-modification document snapshot
 * @param clearPreModDoc  - Callback to clear the pre-modification snapshot after use
 * @param closeTransformBatch - Callback to close the history batch
 */
export function handleObjectModified(
  opt: { target: fabric.FabricObject },
  canvas: fabric.Canvas,
  getPreModDoc: () => PenDocument | null,
  clearPreModDoc: () => void,
  closeTransformBatch: () => void,
): void {
  // Guard against re-entry: discardActiveObject() fires
  // _finalizeCurrentTransform -> object:modified recursively.
  if (inModifiedHandler) return
  inModifiedHandler = true
  try {
    clearGuides()
    const target = opt.target

    // Use the snapshot from mouse:down if available; otherwise fall back
    // to the current document (e.g. programmatic modifications).
    const baseDoc = getPreModDoc() ?? useDocumentStore.getState().document
    clearPreModDoc()

    // Open a history batch for this modification when no outer batch
    // (e.g. AI generation) is active.
    const needsBatch = useHistoryStore.getState().batchDepth === 0
    if (needsBatch) {
      useHistoryStore.getState().startBatch(baseDoc)
    }

    let isSelectionModification = false
    let selectionReparented = false

    try {
      // Single object -- bake scale and sync
      const asPen = target as FabricObjectWithPenId
      if (asPen.penNodeId) {
        // Layout reorder: skip normal sync, reorder instead
        // BUT first check if the node was dragged outside its root frame
        if (isLayoutDragActive()) {
          if (checkDragReparent(asPen)) {
            // Dragged outside parent -- cancel layout reorder and detach
            cancelLayoutDrag()
            rebuildNodeRenderInfo()
            const doc = useDocumentStore.getState().document
            useDocumentStore.setState({
              document: { ...doc, children: [...doc.children] },
            })
            closeTransformBatch()
            return
          }
          endLayoutDrag(asPen, canvas)
          rebuildNodeRenderInfo()
          closeTransformBatch()
          return
        }

        // Drag-into layout container: reparent into target container
        if (isDragIntoActive()) {
          commitDragInto(asPen, canvas)
          rebuildNodeRenderInfo()
          closeTransformBatch()
          return
        }

        const scaleX = target.scaleX ?? 1
        const scaleY = target.scaleY ?? 1
        // Path/Polygon dimensions are derived from their data, so we can't
        // bake scale into width/height. Keep scaleX/scaleY on the Fabric
        // object and let syncObjToStore compute the stored dimensions.
        const isPathLike =
          target.type === 'path' ||
          target.type === 'polygon'
        if (!isPathLike) {
          if (target.width !== undefined) {
            target.set({ width: target.width * scaleX, scaleX: 1 })
          }
          if (target.height !== undefined) {
            target.set({ height: target.height * scaleY, scaleY: 1 })
          }
        }
        target.setCoords()
        syncObjToStore(asPen)

        // Finalize children: bake their scale + update store positions
        if (getActiveDragSession()) {
          if (scaleX !== 1 || scaleY !== 1) {
            finalizeParentTransform(asPen, canvas, scaleX, scaleY)
          }
          finalizeParentRotation(asPen)
        }

        // Check if the node was dragged out of / into a root frame
        const didReparentOut = checkDragReparent(asPen)
        // Fallback: check if a root-level node should be reparented INTO a frame
        const didReparentIn = !didReparentOut && asPen.penNodeId
          ? (() => {
              setFabricSyncLock(true)
              const result = checkReparentIntoFrame(asPen.penNodeId!, {
                x: asPen.left ?? 0,
                y: asPen.top ?? 0,
                w: (asPen.width ?? 0) * (asPen.scaleX ?? 1),
                h: (asPen.height ?? 0) * (asPen.scaleY ?? 1),
              })
              setFabricSyncLock(false)
              return result
            })()
          : false
        if (didReparentOut || didReparentIn) {
          // Force re-sync since tree structure changed
          rebuildNodeRenderInfo()
          const doc = useDocumentStore.getState().document
          useDocumentStore.setState({
            document: { ...doc, children: [...doc.children] },
          })
        }
      } else if ('getObjects' in target) {
        isSelectionModification = true
        // ActiveSelection -- bake scale per child, then sync all
        const group = target as fabric.ActiveSelection
        for (const child of group.getObjects()) {
          const sx = child.scaleX ?? 1
          const sy = child.scaleY ?? 1
          const childIsPathLike =
            child.type === 'path' ||
            child.type === 'polygon'
          if (!childIsPathLike) {
            if (child.width !== undefined) {
              child.set({ width: child.width * sx, scaleX: 1 })
            }
            if (child.height !== undefined) {
              child.set({ height: child.height * sy, scaleY: 1 })
            }
          }
          child.setCoords()
        }
        // Drag-into container: reparent all selected objects
        if (isDragIntoActive()) {
          canvas.discardActiveObject()
          commitDragIntoMulti(canvas)
          rebuildNodeRenderInfo()
          closeTransformBatch()
          return
        }

        const childBounds = syncSelectionToStore(target)

        // Check reparenting for each child based on final positions:
        // 1. Objects with a parent that are dragged completely outside -> detach
        // 2. Root-level objects whose center is inside a frame -> reparent into it
        let anyReparented = false
        const selNodeIdSet = new Set(childBounds.map((b) => b.nodeId))
        setFabricSyncLock(true)
        for (const b of childBounds) {
          const bounds = { x: b.x, y: b.y, w: b.w, h: b.h }
          if (checkDragReparentByBounds(b.nodeId, bounds)) {
            anyReparented = true
          } else if (
            checkReparentIntoFrame(b.nodeId, bounds, selNodeIdSet)
          ) {
            anyReparented = true
          }
        }
        setFabricSyncLock(false)

        if (anyReparented) {
          // Tree structure changed -- force full re-sync after the
          // ActiveSelection is disbanded so clip paths and positions
          // are recomputed correctly.
          isSelectionModification = false
          selectionReparented = true
        }
      }

      // Safety cleanup: clear any leftover drag-into session that wasn't
      // committed (e.g. cursor left the container on the final move frame).
      cancelDragInto()
    } finally {
      if (needsBatch) {
        useHistoryStore.getState().endBatch()
      }
    }

    // Force re-sync so clip paths (which use absolute coordinates) are
    // recomputed from the new node positions.  Without this, children of
    // a dragged frame stay clipped to the old parent frame bounds.
    // Skip the forced setState for ActiveSelection modifications -- the
    // positions were already written by syncSelectionToStore(), and
    // re-syncing absolute coords onto group-relative objects would undo
    // the move.
    rebuildNodeRenderInfo()
    if (selectionReparented) {
      // Disband the ActiveSelection so objects become individual canvas
      // items.  Then defer the full re-sync to the next frame -- Fabric
      // needs a render cycle to fully restore the objects' transforms
      // and properties from the disbanded group.  Without this delay,
      // the subscriber re-syncs while objects are in a transitional
      // state and visual properties (fill, clip paths) get lost.
      canvas.discardActiveObject()
      canvas.requestRenderAll()
      requestAnimationFrame(() => {
        rebuildNodeRenderInfo()
        const doc = useDocumentStore.getState().document
        useDocumentStore.setState({
          document: { ...doc, children: [...doc.children] },
        })
      })
    } else if (!isSelectionModification) {
      const currentDoc = useDocumentStore.getState().document
      useDocumentStore.setState({
        document: { ...currentDoc, children: [...currentDoc.children] },
      })
    }

    closeTransformBatch()
  } finally {
    inModifiedHandler = false
  }
}
