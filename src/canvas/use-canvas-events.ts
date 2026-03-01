import { useEffect } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { useAIStore } from '@/stores/ai-store'
import type { PenDocument, PenNode } from '@/types/pen'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'
import { calculateAndSnap } from './guide-utils'
import {
  beginParentDrag,
  endParentDrag,
  getActiveDragSession,
  moveDescendants,
  scaleDescendants,
  rotateDescendants,
  collectDescendantIds,
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
  cancelLayoutDrag,
  isLayoutDragActive,
} from './layout-reorder'
import { isEnterableContainer, resolveTargetAtDepth } from './selection-context'
import {
  checkDragIntoTarget,
  checkDragIntoTargetMulti,
} from './drag-into-layout'
import { createNodeForTool, isDrawingTool, toScene } from './canvas-node-creator'
import { handleObjectModified } from './canvas-object-modified'

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
      const applyInteractivityState = () => {
        const tool = useCanvasStore.getState().activeTool
        const isStreaming = useAIStore.getState().isStreaming

        if (isStreaming) {
          canvas.selection = false
          canvas.skipTargetFind = true
          canvas.discardActiveObject()
          useCanvasStore.getState().clearSelection()
          canvas.requestRenderAll()
          return
        }

        if (isDrawingTool(tool)) {
          canvas.selection = false
          canvas.skipTargetFind = true
          canvas.discardActiveObject()
          canvas.requestRenderAll()
        } else if (tool === 'select') {
          canvas.selection = true
          canvas.skipTargetFind = false
        }
      }

      let prevTool = useCanvasStore.getState().activeTool
      const unsubTool = useCanvasStore.subscribe((state) => {
        if (state.activeTool === prevTool) return
        // Cancel pen tool if switching away mid-drawing
        if (prevTool === 'path' && isPenToolActive() && state.fabricCanvas) {
          cancelPenTool(state.fabricCanvas)
        }
        prevTool = state.activeTool
        if (!state.fabricCanvas) return
        applyInteractivityState()
      })
      const unsubStreaming = useAIStore.subscribe((state) => {
        void state.isStreaming
        applyInteractivityState()
      })
      applyInteractivityState()

      // --- Drawing via native pointer events on the upper canvas ---

      const onPointerDown = (e: PointerEvent) => {
        if (useAIStore.getState().isStreaming) return

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
        if (useAIStore.getState().isStreaming) return

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
        if (useAIStore.getState().isStreaming) {
          if (tempObj) canvas.remove(tempObj)
          tempObj = null
          drawing = false
          startPoint = null
          return
        }

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
        if (useAIStore.getState().isStreaming) return

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
      // lives in `object:modified` -- NOT here -- so that click-to-select without
      // modification never creates a no-op undo entry.
      let preModificationDoc: PenDocument | null = null

      // --- ActiveSelection descendant tracking ---
      // When dragging an ActiveSelection, children of selected objects are
      // separate Fabric objects (due to tree flattening) and are NOT part of
      // the selection group. We track them here so they follow the group drag.
      interface SelectionDescInfo {
        initGroupLeft: number
        initGroupTop: number
        descendants: Map<string, { obj: FabricObjectWithPenId; initLeft: number; initTop: number }>
      }
      let selectionDragInfo: SelectionDescInfo | null = null

      // --- History batching for drag/resize/rotate ---
      let transformBatchActive = false
      let pendingBatchCloseRaf: number | null = null
      const closeTransformBatch = () => {
        if (!transformBatchActive) return
        useHistoryStore
          .getState()
          .endBatch(useDocumentStore.getState().document)
        transformBatchActive = false
      }

      canvas.on('mouse:down', (opt) => {
        if (useAIStore.getState().isStreaming) return

        if (pendingBatchCloseRaf !== null) {
          cancelAnimationFrame(pendingBatchCloseRaf)
          pendingBatchCloseRaf = null
        }

        clipPathsCleared = false
        preModificationDoc = null
        const tool = useCanvasStore.getState().activeTool
        if (tool !== 'select') return
        const e = opt.e as MouseEvent | undefined

        // Keep multi-selection active when clicking one of its selected objects
        // so users can drag the whole set without needing Shift on drag start.
        if (!e?.shiftKey) {
          const { selectedIds } = useCanvasStore.getState().selection
          const clicked = opt.target as FabricObjectWithPenId | null
          const clickedResolved = clicked?.penNodeId
            ? resolveTargetAtDepth(clicked.penNodeId)
            : null
          const activeObj = canvas.getActiveObject()
          const isActiveSelection = !!activeObj?.isType?.('activeSelection')
          if (
            !isActiveSelection &&
            clickedResolved &&
            selectedIds.length > 1 &&
            selectedIds.includes(clickedResolved)
          ) {
            const objects = canvas.getObjects() as FabricObjectWithPenId[]
            const selectedSet = new Set(selectedIds)
            const selectedObjs = objects.filter(
              (o) => o.penNodeId && selectedSet.has(o.penNodeId),
            )
            if (selectedObjs.length > 1) {
              const sel = new fabric.ActiveSelection(selectedObjs, { canvas })
              canvas.setActiveObject(sel)
              canvas.requestRenderAll()
            }
          }
        }

        const activeTarget = canvas.getActiveObject() ?? opt.target
        if (!activeTarget) return


        // Snapshot the document BEFORE any drag/resize/rotate begins.
        // structuredClone ensures we have a deep copy unaffected by later mutations.
        preModificationDoc = structuredClone(useDocumentStore.getState().document)
        useHistoryStore
          .getState()
          .startBatch(useDocumentStore.getState().document)
        transformBatchActive = true

        // ActiveSelection move/scale/rotate: batch + final sync in object:modified.
        // Layout/parent-child single-node logic does not apply here.
        // However, we must track descendants of selected objects so they
        // visually follow the group during drag.
        if ('getObjects' in activeTarget) {
          // Fix: if Fabric's _currentTransform targets a single object
          // inside the selection (happens when handleSelection creates
          // the ActiveSelection during selection:updated, after Fabric
          // already set up the transform for the clicked single object),
          // redirect the transform to the ActiveSelection so the whole
          // group moves/scales/rotates together.
          const ct = (canvas as unknown as { _currentTransform?: {
            target: fabric.FabricObject
            offsetX: number
            offsetY: number
            original?: Record<string, unknown>
          } })._currentTransform
          if (ct && ct.target !== activeTarget) {
            const pointerEvt = opt.e as PointerEvent | undefined
            if (pointerEvt) {
              canvas.calcOffset()
              const pointer = canvas.getScenePoint(pointerEvt)
              ct.target = activeTarget
              ct.offsetX = pointer.x - (activeTarget.left ?? 0)
              ct.offsetY = pointer.y - (activeTarget.top ?? 0)
              if (ct.original) {
                ct.original = {
                  ...ct.original,
                  left: activeTarget.left,
                  top: activeTarget.top,
                  scaleX: activeTarget.scaleX,
                  scaleY: activeTarget.scaleY,
                }
              }
            }
          }

          cancelLayoutDrag()

          const group = activeTarget as fabric.ActiveSelection
          const selObjs = group.getObjects() as FabricObjectWithPenId[]
          const selIds = new Set(
            selObjs.map((o) => o.penNodeId).filter(Boolean) as string[],
          )

          // Collect descendants of all selected objects that are NOT in the selection
          const allCanvasObjs = canvas.getObjects() as FabricObjectWithPenId[]
          const canvasObjMap = new Map(
            allCanvasObjs.filter((o) => o.penNodeId).map((o) => [o.penNodeId!, o]),
          )
          const descendants = new Map<
            string,
            { obj: FabricObjectWithPenId; initLeft: number; initTop: number }
          >()

          for (const selObj of selObjs) {
            if (!selObj.penNodeId) continue
            for (const descId of collectDescendantIds(selObj.penNodeId)) {
              if (selIds.has(descId) || descendants.has(descId)) continue
              const descObj = canvasObjMap.get(descId)
              if (descObj) {
                descendants.set(descId, {
                  obj: descObj,
                  initLeft: descObj.left ?? 0,
                  initTop: descObj.top ?? 0,
                })
              }
            }
          }

          selectionDragInfo =
            descendants.size > 0
              ? {
                  initGroupLeft: group.left ?? 0,
                  initGroupTop: group.top ?? 0,
                  descendants,
                }
              : null

          return
        }

        const target = activeTarget as FabricObjectWithPenId
        if (!target.penNodeId) return

        // Only start layout reorder for actual move drags.
        // Scale/rotate handles on layout children should follow normal transform sync.
        const transform = (opt as unknown as {
          transform?: { action?: string; corner?: string | null }
        }).transform
        const action = transform?.action
        const corner = transform?.corner
        const isHandleTransform = typeof corner === 'string' && corner.length > 0
        const isMoveAction =
          !isHandleTransform &&
          (action === undefined || action === 'drag' || action === 'move')
        if (isMoveAction) {
          beginLayoutDrag(target.penNodeId)
        } else {
          cancelLayoutDrag()
        }

        // Start parent-child drag session (still needed for child propagation)
        beginParentDrag(target.penNodeId, canvas)
      })

      canvas.on('mouse:up', () => {
        cancelLayoutDrag()
        // NOTE: do NOT cancelDragInto() here -- object:modified handles the
        // commit and cleanup.  In Fabric.js v7 mouse:up can fire before
        // object:modified, which would clear the session prematurely.
        endParentDrag()
        selectionDragInfo = null

        // Defer batch close one frame so object:modified can run first.
        if (transformBatchActive) {
          if (pendingBatchCloseRaf !== null) {
            cancelAnimationFrame(pendingBatchCloseRaf)
          }
          pendingBatchCloseRaf = requestAnimationFrame(() => {
            pendingBatchCloseRaf = null
            closeTransformBatch()
          })
        }
      })

      // --- Object modifications (drag, resize, rotate) via Fabric events ---

      // Real-time sync during drag / resize / rotate (locked to prevent circular sync)
      let clipPathsCleared = false

      canvas.on('object:moving', (opt) => {
        // Clear frame-level clip paths (absolutePositioned: true) on first
        // move so content isn't clipped by stale ancestor frame bounds
        // during drag.  Restored by post-drag re-sync.
        // Preserve object-level clips (absolutePositioned: false) like
        // image corner radius — these are self-contained and stay valid.
        if (!clipPathsCleared) {
          clipPathsCleared = true
          const movingObj = opt.target as FabricObjectWithPenId
          if (movingObj.clipPath?.absolutePositioned) movingObj.clipPath = undefined
          // Clear clip paths on objects INSIDE the ActiveSelection
          if ('getObjects' in opt.target) {
            for (const child of (opt.target as fabric.ActiveSelection).getObjects()) {
              if (child.clipPath?.absolutePositioned) child.clipPath = undefined
            }
          }
          // Also clear descendants' clip paths
          const session = getActiveDragSession()
          if (session) {
            for (const [, descObj] of session.descendantObjects) {
              if (descObj.clipPath?.absolutePositioned) descObj.clipPath = undefined
            }
          }
          // Clear clip paths on ActiveSelection descendants too
          if (selectionDragInfo) {
            for (const [, { obj }] of selectionDragInfo.descendants) {
              if (obj.clipPath?.absolutePositioned) obj.clipPath = undefined
            }
          }
        }

        // ActiveSelection drag: snap + move descendants + drag-into detection
        if ('getObjects' in opt.target) {
          const group = opt.target as fabric.ActiveSelection

          // Smart guides + snapping for the whole selection bounding box
          calculateAndSnap(opt.target, canvas)

          // Move descendants based on the (possibly snapped) group position
          if (selectionDragInfo) {
            const deltaX = (group.left ?? 0) - selectionDragInfo.initGroupLeft
            const deltaY = (group.top ?? 0) - selectionDragInfo.initGroupTop
            for (const [, { obj, initLeft, initTop }] of selectionDragInfo.descendants) {
              obj.set({ left: initLeft + deltaX, top: initTop + deltaY })
              obj.setCoords()
            }
          }

          // Drag-into layout container detection (using selection center)
          const selObjs = group.getObjects() as FabricObjectWithPenId[]
          const selNodeIds = selObjs
            .map((o) => o.penNodeId)
            .filter(Boolean) as string[]
          // ActiveSelection uses center origin -- left/top IS the center
          const cx = group.originX === 'center'
            ? (group.left ?? 0)
            : (group.left ?? 0) + ((group.width ?? 0) * (group.scaleX ?? 1)) / 2
          const cy = group.originY === 'center'
            ? (group.top ?? 0)
            : (group.top ?? 0) + ((group.height ?? 0) * (group.scaleY ?? 1)) / 2
          checkDragIntoTargetMulti(cx, cy, selNodeIds, canvas)

          return
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
        if (pendingBatchCloseRaf !== null) {
          cancelAnimationFrame(pendingBatchCloseRaf)
          pendingBatchCloseRaf = null
        }
        handleObjectModified(
          opt,
          canvas,
          () => preModificationDoc,
          () => { preModificationDoc = null },
          closeTransformBatch,
        )
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
        if (pendingBatchCloseRaf !== null) {
          cancelAnimationFrame(pendingBatchCloseRaf)
        }
        closeTransformBatch()
        unsubTool()
        unsubStreaming()
        upperEl.removeEventListener('pointerdown', onPointerDown)
        upperEl.removeEventListener('pointermove', onPointerMove)
        upperEl.removeEventListener('pointerup', onPointerUp)
        upperEl.removeEventListener('dblclick', onDoubleClick)
      }
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
