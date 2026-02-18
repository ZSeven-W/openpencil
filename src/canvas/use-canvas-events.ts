import { useEffect } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, generateId } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import type { PenNode } from '@/types/pen'
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
        prevTool = state.activeTool
        if (!state.fabricCanvas) return
        if (isDrawingTool(state.activeTool)) {
          state.fabricCanvas.selection = false
          state.fabricCanvas.discardActiveObject()
          state.fabricCanvas.requestRenderAll()
        } else if (state.activeTool === 'select') {
          state.fabricCanvas.selection = true
        }
      })

      // --- Drawing via native pointer events on the upper canvas ---

      const onPointerDown = (e: PointerEvent) => {
        const tool = useCanvasStore.getState().activeTool
        if (!isDrawingTool(tool)) return
        const { isPanning } = useCanvasStore.getState().interaction
        if (isPanning) return

        const pointer = toScene(canvas, e)
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

      // All listeners on upperEl because Fabric.js captures the pointer
      // to this element, so pointermove/pointerup won't reach document.
      upperEl.addEventListener('pointerdown', onPointerDown)
      upperEl.addEventListener('pointermove', onPointerMove)
      upperEl.addEventListener('pointerup', onPointerUp)

      // --- History batching for drag/resize/rotate ---
      canvas.on('mouse:down', (opt) => {
        const tool = useCanvasStore.getState().activeTool
        if (tool !== 'select') return
        const target = opt.target as FabricObjectWithPenId | null
        if (!target?.penNodeId) return
        const currentChildren =
          useDocumentStore.getState().document.children
        useHistoryStore.getState().beginBatch(currentChildren)
      })

      canvas.on('mouse:up', () => {
        const { batchDepth } = useHistoryStore.getState()
        if (batchDepth > 0) {
          useHistoryStore.getState().cancelBatch()
        }
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

      /**
       * Route to the correct sync helper depending on whether the target is a
       * single object or an ActiveSelection.
       */
      const syncTargetToStore = (target: fabric.FabricObject) => {
        const asPen = target as FabricObjectWithPenId
        if (asPen.penNodeId) {
          syncObjToStore(asPen)
        } else if ('getObjects' in target) {
          syncSelectionToStore(target)
        }
      }

      // History batching: group all intermediate drag/resize/rotate updates
      // into a single undo entry instead of one per mouse-move event.
      canvas.on('mouse:down', () => {
        useHistoryStore
          .getState()
          .startBatch(useDocumentStore.getState().document)
      })
      canvas.on('mouse:up', () => {
        useHistoryStore.getState().endBatch()
      })

      // Real-time sync during drag / resize / rotate (locked to prevent circular sync)
      canvas.on('object:moving', (opt) => {
        // Calculate guides + snap BEFORE syncing so the store gets the snapped position
        calculateAndSnap(opt.target, canvas)
        syncTargetToStore(opt.target)
      })
      canvas.on('object:scaling', (opt) => {
        syncTargetToStore(opt.target)
      })
      canvas.on('object:rotating', (opt) => {
        syncTargetToStore(opt.target)
      })

      // Final sync: reset scale to 1 and bake into width/height
      canvas.on('object:modified', (opt) => {
        clearGuides()
        const target = opt.target

        // Single object -- bake scale and sync
        const asPen = target as FabricObjectWithPenId
        if (asPen.penNodeId) {
          const scaleX = target.scaleX ?? 1
          const scaleY = target.scaleY ?? 1
          if (target.width !== undefined) {
            target.set({ width: target.width * scaleX, scaleX: 1 })
          }
          if (target.height !== undefined) {
            target.set({ height: target.height * scaleY, scaleY: 1 })
          }
          target.setCoords()
          syncObjToStore(asPen)
        } else if ('getObjects' in target) {
          // ActiveSelection -- bake scale per child, then sync all
          const group = target as fabric.ActiveSelection
          for (const child of group.getObjects()) {
            const sx = child.scaleX ?? 1
            const sy = child.scaleY ?? 1
            if (child.width !== undefined) {
              child.set({ width: child.width * sx, scaleX: 1 })
            }
            if (child.height !== undefined) {
              child.set({ height: child.height * sy, scaleY: 1 })
            }
            child.setCoords()
          }
          syncSelectionToStore(target)
        }

        // Rebuild nodeRenderInfo after locked sync so subsequent property
        // changes from the panel use fresh parent-offset data.
        rebuildNodeRenderInfo()
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
      }
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
