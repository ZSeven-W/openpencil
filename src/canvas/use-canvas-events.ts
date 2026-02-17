import { useEffect } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, generateId } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'
import type { ToolType } from '@/types/canvas'
import {
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
} from './canvas-constants'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'

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

      // --- Object modifications (drag, resize, rotate) via Fabric events ---

      const syncObjToStore = (obj: FabricObjectWithPenId) => {
        if (!obj?.penNodeId) return

        const scaleX = obj.scaleX ?? 1
        const scaleY = obj.scaleY ?? 1
        const updates: Partial<PenNode> = {
          x: obj.left,
          y: obj.top,
          rotation: obj.angle,
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

      // Real-time sync during drag / resize / rotate
      canvas.on('object:moving', (opt) => {
        syncObjToStore(opt.target as FabricObjectWithPenId)
      })
      canvas.on('object:scaling', (opt) => {
        syncObjToStore(opt.target as FabricObjectWithPenId)
      })
      canvas.on('object:rotating', (opt) => {
        syncObjToStore(opt.target as FabricObjectWithPenId)
      })

      // Final sync: reset scale to 1 and bake into width/height
      canvas.on('object:modified', (opt) => {
        const obj = opt.target as FabricObjectWithPenId
        if (!obj?.penNodeId) return

        const scaleX = obj.scaleX ?? 1
        const scaleY = obj.scaleY ?? 1
        if (obj.width !== undefined) {
          obj.set({ width: obj.width * scaleX, scaleX: 1 })
        }
        if (obj.height !== undefined) {
          obj.set({ height: obj.height * scaleY, scaleY: 1 })
        }
        obj.setCoords()

        syncObjToStore(obj)
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
