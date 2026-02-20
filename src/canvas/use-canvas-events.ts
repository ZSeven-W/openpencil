import { useEffect } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, generateId } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import type { PenDocument, PenNode } from '@/types/pen'
import type { ToolType } from '@/types/canvas'
import {
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
} from './canvas-constants'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'
import { nodeRenderInfo, rebuildNodeRenderInfo } from './use-canvas-sync'
import { calculateAndSnap, clearGuides } from './guide-utils'
import {
  beginParentDrag,
  endParentDrag,
  getActiveDragSession,
  moveDescendants,
  scaleDescendants,
  rotateDescendants,
  finalizeParentTransform,
  finalizeParentRotation,
} from './parent-child-transform'
import {
  isPenToolActive,
  penToolPointerDown,
  penToolPointerMove,
  penToolPointerUp,
  penToolDoubleClick,
  cancelPenTool,
} from './pen-tool'
import {
  beginLayoutDrag,
  updateLayoutDrag,
  endLayoutDrag,
  cancelLayoutDrag,
  isLayoutDragActive,
} from './layout-reorder'
import { isEnterableContainer, resolveTargetAtDepth } from './selection-context'
import { checkDragReparent } from './drag-reparent'
import {
  checkDragIntoTarget,
  commitDragInto,
  cancelDragInto,
  isDragIntoActive,
} from './drag-into-layout'

function createNodeForTool(
  tool: ToolType,
  x: number,
  y: number,
  width: number,
  height: number,
): PenNode | null {
  const id = generateId()

  switch (tool) {
    case 'rectangle':
      return {
        id,
        type: 'rectangle',
        name: 'Rectangle',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        fill: [{ type: 'solid', color: DEFAULT_FILL }],
        stroke: {
          thickness: DEFAULT_STROKE_WIDTH,
          fill: [{ type: 'solid', color: DEFAULT_STROKE }],
        },
      }
    case 'frame':
      return {
        id,
        type: 'frame',
        name: 'Frame',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        fill: [{ type: 'solid', color: '#ffffff' }],
        children: [],
      }
    case 'ellipse':
      return {
        id,
        type: 'ellipse',
        name: 'Ellipse',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        fill: [{ type: 'solid', color: DEFAULT_FILL }],
        stroke: {
          thickness: DEFAULT_STROKE_WIDTH,
          fill: [{ type: 'solid', color: DEFAULT_STROKE }],
        },
      }
    case 'line':
      return {
        id,
        type: 'line',
        name: 'Line',
        x,
        y,
        x2: x + width,
        y2: y + height,
        stroke: {
          thickness: DEFAULT_STROKE_WIDTH,
          fill: [{ type: 'solid', color: DEFAULT_STROKE }],
        },
      }
    case 'text':
      return {
        id,
        type: 'text',
        name: 'Text',
        x,
        y,
        content: 'Type here',
        fontSize: 16,
        fontFamily: 'Inter, sans-serif',
        fill: [{ type: 'solid', color: '#000000' }],
      }
    default:
      return null
  }
}

function isDrawingTool(tool: ToolType): boolean {
  return tool !== 'select' && tool !== 'hand'
}

/**
 * Convert a pointer event to scene coordinates using Fabric's own method
 * which correctly handles DPR / retina scaling and viewport transform.
 */
function toScene(
  canvas: fabric.Canvas,
  e: PointerEvent,
): { x: number; y: number } {
  canvas.calcOffset()
  const point = canvas.getScenePoint(e)
  return { x: point.x, y: point.y }
}

export function useCanvasEvents() {
  useEffect(() => {
    let tempObj: fabric.FabricObject | null = null
    let startPoint: { x: number; y: number } | null = null
    let drawing = false

    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      const upperEl = canvas.upperCanvasEl
      if (!upperEl) return

      // --- Tool change: toggle selection ---
      let prevTool = useCanvasStore.getState().activeTool
      const unsubTool = useCanvasStore.subscribe((state) => {
        if (state.activeTool === prevTool) return
        // Cancel pen tool if switching away mid-drawing
        if (prevTool === 'path' && isPenToolActive() && state.fabricCanvas) {
          cancelPenTool(state.fabricCanvas)
        }
        prevTool = state.activeTool
        if (!state.fabricCanvas) return
        if (isDrawingTool(state.activeTool)) {
          state.fabricCanvas.selection = false
          state.fabricCanvas.skipTargetFind = true
          state.fabricCanvas.discardActiveObject()
          state.fabricCanvas.requestRenderAll()
        } else if (state.activeTool === 'select') {
          state.fabricCanvas.selection = true
          state.fabricCanvas.skipTargetFind = false
        }
      })

      // --- Drawing via native pointer events on the upper canvas ---

      const onPointerDown = (e: PointerEvent) => {
        const tool = useCanvasStore.getState().activeTool
        if (!isDrawingTool(tool)) return
        const { isPanning } = useCanvasStore.getState().interaction
        if (isPanning) return

        const pointer = toScene(canvas, e)

        // Pen tool: delegate to state machine
        if (tool === 'path') {
          penToolPointerDown(canvas, pointer)
          return
        }

        startPoint = { x: pointer.x, y: pointer.y }
        drawing = true

        // Text: create immediately
        if (tool === 'text') {
          const node = createNodeForTool(tool, pointer.x, pointer.y, 0, 0)
          if (node) {
            useDocumentStore.getState().addNode(null, node)
          }
          drawing = false
          startPoint = null
          useCanvasStore.getState().setActiveTool('select')
          return
        }

        const baseProps = {
          left: pointer.x,
          top: pointer.y,
          originX: 'left' as const,
          originY: 'top' as const,
          selectable: false,
          evented: false,
          objectCaching: false,
        }

        switch (tool) {
          case 'rectangle':
          case 'frame':
            tempObj = new fabric.Rect({
              ...baseProps,
              width: 0,
              height: 0,
              fill: 'rgba(59, 130, 246, 0.1)',
              strokeWidth: 0,
            })
            break
          case 'ellipse':
            tempObj = new fabric.Ellipse({
              ...baseProps,
              rx: 0,
              ry: 0,
              fill: 'rgba(59, 130, 246, 0.1)',
              strokeWidth: 0,
            })
            break
          case 'line':
            tempObj = new fabric.Line(
              [pointer.x, pointer.y, pointer.x, pointer.y],
              {
                ...baseProps,
                fill: '',
                stroke: '#3b82f6',
                strokeWidth: 1,
                strokeUniform: true,
              },
            )
            break
        }

        if (tempObj) {
          canvas.add(tempObj)
          canvas.renderAll()
        }
      }

      const onPointerMove = (e: PointerEvent) => {
        // Pen tool has its own move handling
        if (isPenToolActive()) {
          const pointer = toScene(canvas, e)
          penToolPointerMove(canvas, pointer)
          return
        }

        if (!drawing || !tempObj || !startPoint) return

        const tool = useCanvasStore.getState().activeTool
        const pointer = toScene(canvas, e)
        const dx = pointer.x - startPoint.x
        const dy = pointer.y - startPoint.y

        switch (tool) {
          case 'rectangle':
          case 'frame': {
            tempObj.set({
              left: dx < 0 ? pointer.x : startPoint.x,
              top: dy < 0 ? pointer.y : startPoint.y,
              width: Math.abs(dx),
              height: Math.abs(dy),
            })
            break
          }
          case 'ellipse': {
            tempObj.set({
              left: dx < 0 ? pointer.x : startPoint.x,
              top: dy < 0 ? pointer.y : startPoint.y,
              rx: Math.abs(dx) / 2,
              ry: Math.abs(dy) / 2,
            })
            break
          }
          case 'line': {
            tempObj.set({ x2: pointer.x, y2: pointer.y })
            break
          }
        }

        tempObj.setCoords()
        canvas.renderAll()
      }

      const onPointerUp = (_e: PointerEvent) => {
        // Pen tool: end handle drag
        if (isPenToolActive()) {
          penToolPointerUp(canvas)
          return
        }

        if (!drawing || !tempObj || !startPoint) {
          drawing = false
          startPoint = null
          return
        }

        const tool = useCanvasStore.getState().activeTool
        const finalX = tempObj.left ?? 0
        const finalY = tempObj.top ?? 0

        let width = 0
        let height = 0

        if (tool === 'line') {
          width = ((tempObj as fabric.Line).x2 ?? 0) - startPoint.x
          height = ((tempObj as fabric.Line).y2 ?? 0) - startPoint.y
        } else {
          width = tempObj.width ?? 0
          height = tempObj.height ?? 0
        }

        canvas.remove(tempObj)
        tempObj = null
        drawing = false

        if (
          Math.abs(width) > 2 ||
          Math.abs(height) > 2 ||
          tool === 'line'
        ) {
          const node = createNodeForTool(tool, finalX, finalY, width, height)
          if (node) {
            useDocumentStore.getState().addNode(null, node)
          }
        }

        startPoint = null
        useCanvasStore.getState().setActiveTool('select')
      }

      const onDoubleClick = (e: MouseEvent) => {
        if (isPenToolActive()) {
          e.preventDefault()
          e.stopPropagation()
          penToolDoubleClick(canvas)
          return
        }

        const tool = useCanvasStore.getState().activeTool
        if (tool !== 'select') return

        const { activeId } = useCanvasStore.getState().selection
        if (!activeId) return

        if (isEnterableContainer(activeId)) {
          canvas.discardActiveObject()
          useCanvasStore.getState().enterFrame(activeId)

          // Find and select the child under the cursor (Figma-style)
          canvas.calcOffset()
          const pointer = canvas.getScenePoint(e as unknown as PointerEvent)
          const objects = canvas.getObjects() as FabricObjectWithPenId[]

          // Iterate topmost-first to find the child under the cursor
          for (let i = objects.length - 1; i >= 0; i--) {
            const obj = objects[i]
            if (!obj.penNodeId) continue
            if (!obj.containsPoint(pointer)) continue

            // Resolve to a selectable node at the new (entered) depth
            const resolved = resolveTargetAtDepth(obj.penNodeId)
            if (!resolved) continue

            // Find the Fabric object for the resolved target
            const resolvedObj = objects.find((o) => o.penNodeId === resolved)
            if (resolvedObj) {
              canvas.setActiveObject(resolvedObj)
              useCanvasStore.getState().setSelection([resolved], resolved)
            }
            break
          }

          canvas.requestRenderAll()
        }
      }

      // All listeners on upperEl because Fabric.js captures the pointer
      // to this element, so pointermove/pointerup won't reach document.
      upperEl.addEventListener('pointerdown', onPointerDown)
      upperEl.addEventListener('pointermove', onPointerMove)
      upperEl.addEventListener('pointerup', onPointerUp)
      upperEl.addEventListener('dblclick', onDoubleClick)

      // --- Drag session setup (layout reorder + parent-child propagation) ---
      // We capture the document snapshot here (before any modification) so that
      // `object:modified` can use it as the undo base state.  History batching
      // lives in `object:modified` — NOT here — so that click-to-select without
      // modification never creates a no-op undo entry.
      let preModificationDoc: PenDocument | null = null

      canvas.on('mouse:down', (opt) => {
        clipPathsCleared = false
        preModificationDoc = null
        const tool = useCanvasStore.getState().activeTool
        if (tool !== 'select') return
        const target = opt.target as FabricObjectWithPenId | null
        if (!target?.penNodeId) return

        // Snapshot the document BEFORE any drag/resize/rotate begins.
        // structuredClone ensures we have a deep copy unaffected by later mutations.
        preModificationDoc = structuredClone(useDocumentStore.getState().document)

        // Try to start layout reorder drag first
        beginLayoutDrag(target.penNodeId)

        // Start parent-child drag session (still needed for child propagation)
        beginParentDrag(target.penNodeId, canvas)
      })

      canvas.on('mouse:up', () => {
        cancelLayoutDrag()
        // NOTE: do NOT cancelDragInto() here — object:modified handles the
        // commit and cleanup.  In Fabric.js v7 mouse:up can fire before
        // object:modified, which would clear the session prematurely.
        endParentDrag()
      })

      // --- Object modifications (drag, resize, rotate) via Fabric events ---

      /** Sync a single Fabric object's transform back to the document store. */
      const syncObjToStore = (obj: FabricObjectWithPenId) => {
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
       */
      const syncSelectionToStore = (
        target: fabric.FabricObject,
      ) => {
        if (!('getObjects' in target)) return
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

          const updates: Partial<PenNode> = {
            x: absLeft - offsetX,
            y: absTop - offsetY,
            rotation: child.angle ?? 0,
          }
          if (child.width !== undefined) {
            ;(updates as Record<string, unknown>).width =
              child.width * scaleX
          }
          if (child.height !== undefined) {
            ;(updates as Record<string, unknown>).height =
              child.height * scaleY
          }

          useDocumentStore.getState().updateNode(obj.penNodeId, updates)
        }
        setFabricSyncLock(false)
      }

      // Real-time sync during drag / resize / rotate (locked to prevent circular sync)
      let clipPathsCleared = false

      canvas.on('object:moving', (opt) => {
        // Clear clip paths on first move so content isn't clipped by stale
        // ancestor frame bounds during drag.  Restored by post-drag re-sync.
        if (!clipPathsCleared) {
          clipPathsCleared = true
          const movingObj = opt.target as FabricObjectWithPenId
          if (movingObj.clipPath) movingObj.clipPath = undefined
          // Also clear descendants' clip paths
          const session = getActiveDragSession()
          if (session) {
            for (const [, descObj] of session.descendantObjects) {
              if (descObj.clipPath) descObj.clipPath = undefined
            }
          }
        }

        if (isLayoutDragActive()) {
          // Layout reorder mode: update insertion indicator, still propagate children
          updateLayoutDrag(opt.target as FabricObjectWithPenId, canvas)
          if (getActiveDragSession()) {
            moveDescendants(opt.target as FabricObjectWithPenId, canvas)
          }
          return
        }

        // Check drag-into for non-layout-child nodes
        checkDragIntoTarget(opt.target as FabricObjectWithPenId, canvas)

        // Calculate guides + snap BEFORE syncing so the store gets the snapped position
        calculateAndSnap(opt.target, canvas)

        // Propagate move to descendants (visual only, no store sync needed)
        if (getActiveDragSession()) {
          moveDescendants(opt.target as FabricObjectWithPenId, canvas)
        }
      })
      canvas.on('object:scaling', (opt) => {
        // Propagate scale to descendants (visual only)
        if (getActiveDragSession()) {
          scaleDescendants(opt.target as FabricObjectWithPenId, canvas)
        }
      })
      canvas.on('object:rotating', (opt) => {
        // Propagate rotation to descendants (visual only)
        if (getActiveDragSession()) {
          rotateDescendants(opt.target as FabricObjectWithPenId, canvas)
        }
      })

      // Final sync: reset scale to 1 and bake into width/height.
      // History batching lives here (not in mouse:down/mouse:up) so that
      // click-to-select without modification never creates a no-op undo
      // entry.  We use the pre-modification snapshot captured in mouse:down
      // as the batch base to guarantee a correct undo point.
      canvas.on('object:modified', (opt) => {
        clearGuides()
        const target = opt.target

        // Use the snapshot from mouse:down if available; otherwise fall back
        // to the current document (e.g. programmatic modifications).
        const baseDoc = preModificationDoc ?? useDocumentStore.getState().document
        preModificationDoc = null

        // Open a history batch for this modification when no outer batch
        // (e.g. AI generation) is active.
        const needsBatch = useHistoryStore.getState().batchDepth === 0
        if (needsBatch) {
          useHistoryStore.getState().startBatch(baseDoc)
        }

        try {
        // Single object -- bake scale and sync
        const asPen = target as FabricObjectWithPenId
        if (asPen.penNodeId) {
          // Layout reorder: skip normal sync, reorder instead
          // BUT first check if the node was dragged outside its root frame
          if (isLayoutDragActive()) {
            if (checkDragReparent(asPen)) {
              // Dragged outside parent — cancel layout reorder and detach
              cancelLayoutDrag()
              rebuildNodeRenderInfo()
              const doc = useDocumentStore.getState().document
              useDocumentStore.setState({
                document: { ...doc, children: [...doc.children] },
              })
              return
            }
            endLayoutDrag(asPen, canvas)
            rebuildNodeRenderInfo()
            return
          }

          // Drag-into layout container: reparent into target container
          if (isDragIntoActive()) {
            commitDragInto(asPen, canvas)
            rebuildNodeRenderInfo()
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
          if (checkDragReparent(asPen)) {
            // Force re-sync since tree structure changed
            rebuildNodeRenderInfo()
            const doc = useDocumentStore.getState().document
            useDocumentStore.setState({
              document: { ...doc, children: [...doc.children] },
            })
          }
        } else if ('getObjects' in target) {
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
          syncSelectionToStore(target)
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
        rebuildNodeRenderInfo()
        const currentDoc = useDocumentStore.getState().document
        useDocumentStore.setState({
          document: { ...currentDoc, children: [...currentDoc.children] },
        })
      })

      // --- Text editing: sync edited content back to document store ---
      canvas.on('text:editing:exited', (opt) => {
        const obj = opt.target as FabricObjectWithPenId
        if (!obj?.penNodeId) return

        const text =
          'text' in obj ? (obj as fabric.IText | fabric.Textbox).text : undefined
        if (text === undefined) return

        setFabricSyncLock(true)
        useDocumentStore.getState().updateNode(obj.penNodeId, {
          content: text,
        } as Partial<PenNode>)
        setFabricSyncLock(false)
      })

      return () => {
        unsubTool()
        upperEl.removeEventListener('pointerdown', onPointerDown)
        upperEl.removeEventListener('pointermove', onPointerMove)
        upperEl.removeEventListener('pointerup', onPointerUp)
        upperEl.removeEventListener('dblclick', onDoubleClick)
      }
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
