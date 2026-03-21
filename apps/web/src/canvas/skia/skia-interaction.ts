import { screenToScene } from './skia-engine'
import type { SkiaEngine } from './skia-engine'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { createNodeForTool, isDrawingTool } from '../canvas-node-creator'
import { inferLayout } from '../canvas-layout-engine'
import { SkiaPenTool } from './skia-pen-tool'
import type { ToolType } from '@/types/canvas'
import type { PenNode, ContainerProps, TextNode, EllipseNode } from '@/types/pen'
import {
  type HandleDir, type ArcHandleType,
  DRAG_THRESHOLD, handleCursors,
  hitTestHandle, hitTestRotation, hitTestArcHandle,
} from './skia-hit-handlers'

export interface TextEditState {
  nodeId: string
  x: number; y: number; w: number; h: number
  content: string
  fontSize: number
  fontFamily: string
  fontWeight: string
  textAlign: string
  color: string
  lineHeight: number
}


export function toolToCursor(tool: ToolType): string {
  switch (tool) {
    case 'hand': return 'grab'
    case 'text': return 'text'
    case 'select': return 'default'
    default: return 'crosshair'
  }
}

/**
 * Encapsulates all canvas mouse/keyboard interaction state and handlers.
 * Extracted from SkiaCanvas to keep the component focused on lifecycle and rendering.
 */
export class SkiaInteractionManager {
  private engineRef: { current: SkiaEngine | null }
  private canvasEl: HTMLCanvasElement
  private onEditText: (state: TextEditState | null) => void

  // Shared state
  private isPanning = false
  private spacePressed = false
  private lastX = 0
  private lastY = 0

  // Select tool state
  private isDragging = false
  private dragMoved = false
  private isMarquee = false
  private dragNodeIds: string[] = []
  private dragStartSceneX = 0
  private dragStartSceneY = 0
  private dragOrigPositions: { id: string; x: number; y: number }[] = []
  private dragPrevDx = 0
  private dragPrevDy = 0
  private dragAllIds: Set<string> | null = null

  // Resize handle state
  private isResizing = false
  private resizeHandle: HandleDir | null = null
  private resizeNodeId: string | null = null
  private resizeOrigX = 0
  private resizeOrigY = 0
  private resizeOrigW = 0
  private resizeOrigH = 0
  private resizeStartSceneX = 0
  private resizeStartSceneY = 0

  // Rotation state
  private isRotating = false
  private rotateNodeId: string | null = null
  private rotateOrigAngle = 0
  private rotateCenterX = 0
  private rotateCenterY = 0
  private rotateStartAngle = 0

  // Arc handle state
  private isDraggingArc = false
  private arcHandleType: ArcHandleType | null = null
  private arcNodeId: string | null = null

  // Drawing tool state
  private isDrawing = false
  private drawTool: ToolType = 'select'
  private drawStartX = 0
  private drawStartY = 0

  // Pen tool
  private penTool: SkiaPenTool

  constructor(
    engineRef: { current: SkiaEngine | null },
    canvasEl: HTMLCanvasElement,
    onEditText: (state: TextEditState | null) => void,
  ) {
    this.engineRef = engineRef
    this.canvasEl = canvasEl
    this.onEditText = onEditText
    this.penTool = new SkiaPenTool(() => this.engineRef.current)
  }

  private getEngine() { return this.engineRef.current }
  private getTool() { return useCanvasStore.getState().activeTool }

  private getScene(e: MouseEvent) {
    const engine = this.getEngine()
    if (!engine) return null
    const rect = engine.getCanvasRect()
    if (!rect) return null
    return screenToScene(e.clientX, e.clientY, rect, {
      zoom: engine.zoom, panX: engine.panX, panY: engine.panY,
    })
  }

  // ---------------------------------------------------------------------------
  // Mouse down
  // ---------------------------------------------------------------------------

  private onMouseDown = (e: MouseEvent) => {
    const engine = this.getEngine()
    if (!engine) return

    if (e.button === 2) return

    // Pan: space+click, hand tool, or middle mouse
    if (this.spacePressed || this.getTool() === 'hand' || e.button === 1) {
      this.isPanning = true
      this.lastX = e.clientX
      this.lastY = e.clientY
      this.canvasEl.style.cursor = 'grabbing'
      return
    }

    const tool = this.getTool()
    const scene = this.getScene(e)
    if (!scene) return

    // Text tool: click to create immediately
    if (tool === 'text') {
      const node = createNodeForTool('text', scene.x, scene.y, 0, 0)
      if (node) {
        useDocumentStore.getState().addNode(null, node)
        useCanvasStore.getState().setSelection([node.id], node.id)
      }
      useCanvasStore.getState().setActiveTool('select')
      return
    }

    // Pen tool
    if (tool === 'path') {
      this.penTool.onMouseDown(scene, engine.zoom || 1)
      return
    }

    // Drawing tools: start rubber-band
    if (isDrawingTool(tool)) {
      this.isDrawing = true
      this.drawTool = tool
      this.drawStartX = scene.x
      this.drawStartY = scene.y
      engine.previewShape = {
        type: tool as 'rectangle' | 'ellipse' | 'frame' | 'line' | 'polygon',
        x: scene.x, y: scene.y, w: 0, h: 0,
      }
      engine.markDirty()
      return
    }

    // Select tool
    if (tool === 'select') {
      this.handleSelectMouseDown(e, scene, engine)
    }
  }

  private handleSelectMouseDown(
    e: MouseEvent,
    scene: { x: number; y: number },
    engine: SkiaEngine,
  ) {
    // Check arc handles first
    const arcHit = hitTestArcHandle(engine,scene.x, scene.y)
    if (arcHit) {
      this.isDraggingArc = true
      this.arcHandleType = arcHit.type
      this.arcNodeId = arcHit.nodeId
      this.canvasEl.style.cursor = 'pointer'
      return
    }

    // Check resize handle
    const handleHit = hitTestHandle(engine,scene.x, scene.y)
    if (handleHit) {
      this.isResizing = true
      this.resizeHandle = handleHit.dir
      this.resizeNodeId = handleHit.nodeId
      this.resizeStartSceneX = scene.x
      this.resizeStartSceneY = scene.y
      const docNode = useDocumentStore.getState().getNodeById(handleHit.nodeId)
      this.resizeOrigX = docNode?.x ?? 0
      this.resizeOrigY = docNode?.y ?? 0
      const resizeRN = engine.spatialIndex.get(handleHit.nodeId)
      const docNodeAny = docNode as (PenNode & ContainerProps) | undefined
      this.resizeOrigW = resizeRN?.absW ?? (typeof docNodeAny?.width === 'number' ? docNodeAny.width : 100)
      this.resizeOrigH = resizeRN?.absH ?? (typeof docNodeAny?.height === 'number' ? docNodeAny.height : 100)
      this.canvasEl.style.cursor = handleCursors[handleHit.dir]
      return
    }

    // Check rotation zone
    const rotHit = hitTestRotation(engine,scene.x, scene.y)
    if (rotHit) {
      this.isRotating = true
      this.rotateNodeId = rotHit.nodeId
      const docNode = useDocumentStore.getState().getNodeById(rotHit.nodeId)
      this.rotateOrigAngle = docNode?.rotation ?? 0
      const rn = engine.spatialIndex.get(rotHit.nodeId)!
      this.rotateCenterX = rn.absX + rn.absW / 2
      this.rotateCenterY = rn.absY + rn.absH / 2
      this.rotateStartAngle = Math.atan2(scene.y - this.rotateCenterY, scene.x - this.rotateCenterX) * 180 / Math.PI
      this.canvasEl.style.cursor = 'grabbing'
      return
    }

    const hits = engine.spatialIndex.hitTest(scene.x, scene.y)

    if (hits.length > 0) {
      const topHit = hits[0]
      let nodeId = topHit.node.id
      const currentSelection = useCanvasStore.getState().selection.selectedIds
      const docStore = useDocumentStore.getState()

      const isChildOfSelected = currentSelection.some(
        (selId) => selId !== nodeId && docStore.isDescendantOf(nodeId, selId),
      )
      if (isChildOfSelected) {
        // Don't change selection
      } else if (!currentSelection.includes(nodeId)) {
        const parent = docStore.getParentOf(nodeId)
        if (parent && (parent.type === 'frame' || parent.type === 'group')) {
          const grandparent = docStore.getParentOf(parent.id)
          if (!grandparent || grandparent.type === 'frame') {
            nodeId = parent.id
          }
        }

        if (e.shiftKey) {
          if (currentSelection.includes(nodeId)) {
            const next = currentSelection.filter((id) => id !== nodeId)
            useCanvasStore.getState().setSelection(next, next[0] ?? null)
          } else {
            useCanvasStore.getState().setSelection([...currentSelection, nodeId], nodeId)
          }
        } else {
          useCanvasStore.getState().setSelection([nodeId], nodeId)
        }
      }

      // Start drag
      const selectedIds = useCanvasStore.getState().selection.selectedIds
      this.isDragging = true
      this.dragMoved = false
      this.dragNodeIds = selectedIds
      this.dragStartSceneX = scene.x
      this.dragStartSceneY = scene.y
      this.dragOrigPositions = selectedIds.map((id) => {
        const n = useDocumentStore.getState().getNodeById(id)
        return { id, x: n?.x ?? 0, y: n?.y ?? 0 }
      })
    } else {
      // Empty space → start marquee or clear selection
      if (!e.shiftKey) {
        useCanvasStore.getState().clearSelection()
      }
      this.isMarquee = true
      this.lastX = scene.x
      this.lastY = scene.y
      engine.marquee = { x1: scene.x, y1: scene.y, x2: scene.x, y2: scene.y }
    }
  }

  // ---------------------------------------------------------------------------
  // Mouse move
  // ---------------------------------------------------------------------------

  private onMouseMove = (e: MouseEvent) => {
    const engine = this.getEngine()
    if (!engine) return

    if (this.isPanning) {
      const dx = e.clientX - this.lastX
      const dy = e.clientY - this.lastY
      this.lastX = e.clientX
      this.lastY = e.clientY
      engine.pan(dx, dy)
      return
    }

    const scene = this.getScene(e)
    if (!scene) return

    if (this.penTool.onMouseMove(scene)) return

    if (this.isResizing && this.resizeHandle && this.resizeNodeId) {
      this.handleResizeMove(scene, engine)
      return
    }

    if (this.isRotating && this.rotateNodeId) {
      this.handleRotateMove(scene, e.shiftKey)
      return
    }

    if (this.isDraggingArc && this.arcNodeId && this.arcHandleType) {
      this.handleArcMove(scene, engine)
      return
    }

    if (this.isDrawing && engine.previewShape) {
      this.handleDrawingMove(scene, engine)
      return
    }

    if (this.isDragging && this.dragNodeIds.length > 0) {
      this.handleDragMove(scene, engine)
      return
    }

    if (this.isMarquee && engine.marquee) {
      this.handleMarqueeMove(scene, engine)
      return
    }

    // Hover + handle cursor (select tool only)
    if (this.getTool() === 'select' && !this.spacePressed) {
      this.handleHoverCursor(scene, engine)
    }
  }

  private handleResizeMove(scene: { x: number; y: number }, engine: SkiaEngine) {
    const dx = scene.x - this.resizeStartSceneX
    const dy = scene.y - this.resizeStartSceneY
    let newX = this.resizeOrigX
    let newY = this.resizeOrigY
    let newW = this.resizeOrigW
    let newH = this.resizeOrigH

    const dir = this.resizeHandle!
    if (dir.includes('w')) { newX = this.resizeOrigX + dx; newW = this.resizeOrigW - dx }
    if (dir.includes('e')) { newW = this.resizeOrigW + dx }
    if (dir.includes('n')) { newY = this.resizeOrigY + dy; newH = this.resizeOrigH - dy }
    if (dir.includes('s')) { newH = this.resizeOrigH + dy }

    const MIN = 2
    if (newW < MIN) { if (dir.includes('w')) newX = this.resizeOrigX + this.resizeOrigW - MIN; newW = MIN }
    if (newH < MIN) { if (dir.includes('n')) newY = this.resizeOrigY + this.resizeOrigH - MIN; newH = MIN }

    const resizedNode = useDocumentStore.getState().getNodeById(this.resizeNodeId!)
    const updates: Record<string, unknown> = { x: newX, y: newY, width: newW, height: newH }
    if (resizedNode?.type === 'text' && !(resizedNode as TextNode).textGrowth) {
      updates.textGrowth = 'fixed-width'
    }
    useDocumentStore.getState().updateNode(this.resizeNodeId!, updates as Partial<PenNode>)

    if (
      resizedNode
      && 'children' in resizedNode
      && resizedNode.children?.length
    ) {
      const resizeRN2 = engine.spatialIndex.get(this.resizeNodeId!)
      const resizedContainer = resizedNode as PenNode & ContainerProps
      const oldW = resizeRN2?.absW ?? (typeof resizedContainer.width === 'number' ? resizedContainer.width : 0)
      const oldH = resizeRN2?.absH ?? (typeof resizedContainer.height === 'number' ? resizedContainer.height : 0)
      if (oldW > 0 && oldH > 0) {
        const scaleX = newW / oldW
        const scaleY = newH / oldH
        useDocumentStore.getState().scaleDescendantsInStore(this.resizeNodeId!, scaleX, scaleY)
      }
    }
  }

  private handleRotateMove(scene: { x: number; y: number }, shiftKey: boolean) {
    const currentAngle = Math.atan2(scene.y - this.rotateCenterY, scene.x - this.rotateCenterX) * 180 / Math.PI
    let newAngle = this.rotateOrigAngle + (currentAngle - this.rotateStartAngle)
    if (shiftKey) {
      newAngle = Math.round(newAngle / 15) * 15
    }
    useDocumentStore.getState().updateNode(this.rotateNodeId!, { rotation: newAngle } as Partial<PenNode>)
  }

  private handleArcMove(scene: { x: number; y: number }, engine: SkiaEngine) {
    const rn = engine.spatialIndex.get(this.arcNodeId!)
    if (!rn) return

    const cx = rn.absX + rn.absW / 2
    const cy = rn.absY + rn.absH / 2
    const angle = Math.atan2(scene.y - cy, scene.x - cx) * 180 / Math.PI
    const normalizedAngle = ((angle % 360) + 360) % 360
    const eNode = rn.node as EllipseNode

    if (this.arcHandleType === 'start') {
      const oldStart = eNode.startAngle ?? 0
      const oldEnd = oldStart + (eNode.sweepAngle ?? 360)
      const newSweep = ((oldEnd - normalizedAngle) % 360 + 360) % 360
      useDocumentStore.getState().updateNode(this.arcNodeId!, {
        startAngle: normalizedAngle,
        sweepAngle: newSweep || 360,
      } as Partial<PenNode>)
    } else if (this.arcHandleType === 'end') {
      const startA = eNode.startAngle ?? 0
      const newSweep = ((normalizedAngle - startA) % 360 + 360) % 360
      useDocumentStore.getState().updateNode(this.arcNodeId!, {
        sweepAngle: newSweep || 360,
      } as Partial<PenNode>)
    } else if (this.arcHandleType === 'inner') {
      const rx = rn.absW / 2
      const ry = rn.absH / 2
      const dist = Math.hypot((scene.x - cx) / rx, (scene.y - cy) / ry)
      const newInner = Math.max(0, Math.min(0.99, dist))
      useDocumentStore.getState().updateNode(this.arcNodeId!, {
        innerRadius: newInner,
      } as Partial<PenNode>)
    }
  }

  private handleDrawingMove(scene: { x: number; y: number }, engine: SkiaEngine) {
    const dx = scene.x - this.drawStartX
    const dy = scene.y - this.drawStartY

    if (this.drawTool === 'line') {
      engine.previewShape = {
        type: 'line',
        x: this.drawStartX, y: this.drawStartY,
        w: dx, h: dy,
      }
    } else {
      engine.previewShape = {
        type: this.drawTool as 'rectangle' | 'ellipse' | 'frame' | 'line' | 'polygon',
        x: dx < 0 ? scene.x : this.drawStartX,
        y: dy < 0 ? scene.y : this.drawStartY,
        w: Math.abs(dx),
        h: Math.abs(dy),
      }
    }
    engine.markDirty()
  }

  private handleDragMove(scene: { x: number; y: number }, engine: SkiaEngine) {
    const dx = scene.x - this.dragStartSceneX
    const dy = scene.y - this.dragStartSceneY

    if (!this.dragMoved) {
      const screenDist = Math.hypot(dx * engine.zoom, dy * engine.zoom)
      if (screenDist < DRAG_THRESHOLD) return
      this.dragMoved = true
      engine.dragSyncSuppressed = true
      this.dragPrevDx = 0
      this.dragPrevDy = 0
      this.dragAllIds = new Set(this.dragNodeIds)
      for (const id of this.dragNodeIds) {
        const collectDescs = (nodeId: string) => {
          const n = useDocumentStore.getState().getNodeById(nodeId)
          if (n && 'children' in n && n.children) {
            for (const child of n.children) {
              this.dragAllIds!.add(child.id)
              collectDescs(child.id)
            }
          }
        }
        collectDescs(id)
      }
    }

    const incrDx = dx - this.dragPrevDx
    const incrDy = dy - this.dragPrevDy
    this.dragPrevDx = dx
    this.dragPrevDy = dy

    for (const rn of engine.renderNodes) {
      if (this.dragAllIds!.has(rn.node.id)) {
        rn.absX += incrDx
        rn.absY += incrDy
        rn.node = { ...rn.node, x: rn.absX, y: rn.absY }
      }
    }
    engine.spatialIndex.rebuild(engine.renderNodes)
    engine.markDirty()
  }

  private handleMarqueeMove(scene: { x: number; y: number }, engine: SkiaEngine) {
    engine.marquee!.x2 = scene.x
    engine.marquee!.y2 = scene.y
    engine.markDirty()

    const marqueeHits = engine.spatialIndex.searchRect(
      engine.marquee!.x1, engine.marquee!.y1,
      engine.marquee!.x2, engine.marquee!.y2,
    )
    const ids = marqueeHits.map((rn) => rn.node.id)
    useCanvasStore.getState().setSelection(ids, ids[0] ?? null)
  }

  private handleHoverCursor(scene: { x: number; y: number }, engine: SkiaEngine) {
    const arcHoverHit = hitTestArcHandle(engine,scene.x, scene.y)
    if (arcHoverHit) {
      this.canvasEl.style.cursor = 'pointer'
      return
    }
    const handleHit = hitTestHandle(engine,scene.x, scene.y)
    if (handleHit) {
      this.canvasEl.style.cursor = handleCursors[handleHit.dir]
    } else if (hitTestRotation(engine,scene.x, scene.y)) {
      this.canvasEl.style.cursor = 'url("data:image/svg+xml,%3Csvg xmlns=\'http://www.w3.org/2000/svg\' width=\'24\' height=\'24\' viewBox=\'0 0 24 24\' fill=\'none\' stroke=\'black\' stroke-width=\'2\'%3E%3Cpath d=\'M21 2v6h-6\'/%3E%3Cpath d=\'M21 13a9 9 0 1 1-3-7.7L21 8\'/%3E%3C/svg%3E") 12 12, crosshair'
    } else {
      const hoverHits = engine.spatialIndex.hitTest(scene.x, scene.y)
      const newHoveredId = hoverHits.length > 0 ? hoverHits[0].node.id : null
      this.canvasEl.style.cursor = newHoveredId ? 'move' : 'default'
      if (newHoveredId !== engine.hoveredNodeId) {
        engine.hoveredNodeId = newHoveredId
        useCanvasStore.getState().setHoveredId(newHoveredId)
        engine.markDirty()
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Mouse up
  // ---------------------------------------------------------------------------

  private onMouseUp = () => {
    const engine = this.getEngine()

    if (this.penTool.onMouseUp()) return

    if (this.isPanning) {
      this.isPanning = false
      this.canvasEl.style.cursor = this.spacePressed ? 'grab' : toolToCursor(this.getTool())
    }

    if (this.isResizing) {
      this.isResizing = false
      this.resizeHandle = null
      this.resizeNodeId = null
      this.canvasEl.style.cursor = toolToCursor(this.getTool())
    }

    if (this.isDraggingArc) {
      this.isDraggingArc = false
      this.arcHandleType = null
      this.arcNodeId = null
      this.canvasEl.style.cursor = toolToCursor(this.getTool())
    }

    if (this.isRotating) {
      this.isRotating = false
      this.rotateNodeId = null
      this.canvasEl.style.cursor = toolToCursor(this.getTool())
    }

    if (this.isDrawing && engine?.previewShape) {
      const { type, x, y, w, h } = engine.previewShape
      engine.previewShape = null
      engine.markDirty()
      this.isDrawing = false

      const minSize = type === 'line'
        ? Math.hypot(w, h) >= 2
        : w >= 2 && h >= 2
      if (minSize) {
        const node = createNodeForTool(this.drawTool, x, y, w, h)
        if (node) {
          useDocumentStore.getState().addNode(null, node)
          useCanvasStore.getState().setSelection([node.id], node.id)
        }
      }
      useCanvasStore.getState().setActiveTool('select')
      return
    }
    this.isDrawing = false

    // Select tool: end drag / marquee
    if (this.isDragging && this.dragMoved && this.dragOrigPositions.length > 0 && engine) {
      this.handleDragEnd(engine)
    } else if (engine) {
      engine.dragSyncSuppressed = false
    }
    this.isDragging = false
    this.dragNodeIds = []
    this.dragOrigPositions = []
    this.dragAllIds = null
    if (this.isMarquee && engine) {
      engine.marquee = null
      engine.markDirty()
    }
    this.isMarquee = false
  }

  private handleDragEnd(engine: SkiaEngine) {
    const dx = this.dragPrevDx
    const dy = this.dragPrevDy
    const docStore = useDocumentStore.getState()

    for (const orig of this.dragOrigPositions) {
      const parent = docStore.getParentOf(orig.id)
      const draggedRN = engine.renderNodes.find((rn) => rn.node.id === orig.id)
      const objBounds = draggedRN
        ? { x: draggedRN.absX, y: draggedRN.absY, w: draggedRN.absW, h: draggedRN.absH }
        : { x: orig.x + dx, y: orig.y + dy, w: 100, h: 100 }

      // Check if dragged completely outside parent → reparent
      if (parent) {
        const parentRN = engine.renderNodes.find((rn) => rn.node.id === parent.id)
        if (parentRN) {
          const pBounds = { x: parentRN.absX, y: parentRN.absY, w: parentRN.absW, h: parentRN.absH }
          const outside =
            objBounds.x + objBounds.w <= pBounds.x ||
            objBounds.x >= pBounds.x + pBounds.w ||
            objBounds.y + objBounds.h <= pBounds.y ||
            objBounds.y >= pBounds.y + pBounds.h

          if (outside) {
            docStore.updateNode(orig.id, { x: objBounds.x, y: objBounds.y } as Partial<PenNode>)
            docStore.moveNode(orig.id, null, 0)
            continue
          }
        }
      }

      const parentLayout = parent
        ? ((parent as PenNode & ContainerProps).layout || inferLayout(parent))
        : undefined

      if (parentLayout && parentLayout !== 'none' && parent) {
        const siblings = ('children' in parent ? parent.children ?? [] : [])
          .filter((c) => c.id !== orig.id)
        const isVertical = parentLayout === 'vertical'

        let newIndex = siblings.length
        for (let i = 0; i < siblings.length; i++) {
          const sibRN = engine.renderNodes.find((rn) => rn.node.id === siblings[i].id)
          const sibMid = sibRN
            ? (isVertical ? sibRN.absY + sibRN.absH / 2 : sibRN.absX + sibRN.absW / 2)
            : 0
          const dragMid = isVertical
            ? objBounds.y + objBounds.h / 2
            : objBounds.x + objBounds.w / 2
          if (dragMid < sibMid) {
            newIndex = i
            break
          }
        }
        docStore.moveNode(orig.id, parent.id, newIndex)
      } else {
        docStore.updateNode(orig.id, {
          x: orig.x + dx,
          y: orig.y + dy,
        } as Partial<PenNode>)
      }
    }

    engine.dragSyncSuppressed = false
    engine.syncFromDocument()
  }

  // ---------------------------------------------------------------------------
  // Double click — text editing
  // ---------------------------------------------------------------------------

  private onDblClick = (e: MouseEvent) => {
    const engine = this.getEngine()
    if (!engine) return

    if (this.penTool.onDblClick()) return

    if (this.getTool() !== 'select') return

    const scene = this.getScene(e)
    if (!scene) return

    const hits = engine.spatialIndex.hitTest(scene.x, scene.y)
    if (hits.length === 0) return

    const topHit = hits[0]
    const currentSelection = useCanvasStore.getState().selection.selectedIds

    // Double-click on a selected group/frame → enter it and select the child
    if (currentSelection.length === 1) {
      const selectedNode = useDocumentStore.getState().getNodeById(currentSelection[0])
      if (
        selectedNode
        && (selectedNode.type === 'frame' || selectedNode.type === 'group')
        && 'children' in selectedNode && selectedNode.children?.length
      ) {
        const childId = topHit.node.id
        if (childId !== currentSelection[0]) {
          useCanvasStore.getState().setSelection([childId], childId)
          return
        }
      }
    }

    if (topHit.node.type !== 'text') return

    const tNode = topHit.node as TextNode
    const fills = tNode.fill
    const firstFill = Array.isArray(fills) ? fills[0] : undefined
    const color = firstFill?.type === 'solid' ? firstFill.color : '#000000'

    this.onEditText({
      nodeId: topHit.node.id,
      x: topHit.absX * engine.zoom + engine.panX,
      y: topHit.absY * engine.zoom + engine.panY,
      w: topHit.absW * engine.zoom,
      h: topHit.absH * engine.zoom,
      content: typeof tNode.content === 'string'
        ? tNode.content
        : Array.isArray(tNode.content)
          ? tNode.content.map((s) => s.text ?? '').join('')
          : '',
      fontSize: (tNode.fontSize ?? 16) * engine.zoom,
      fontFamily: tNode.fontFamily ?? 'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif',
      fontWeight: String(tNode.fontWeight ?? '400'),
      textAlign: tNode.textAlign ?? 'left',
      color,
      lineHeight: tNode.lineHeight ?? 1.4,
    })
  }

  // ---------------------------------------------------------------------------
  // Keyboard: space for panning
  // ---------------------------------------------------------------------------

  private onKeyDown = (e: KeyboardEvent) => {
    if (this.penTool.onKeyDown(e.key)) {
      e.preventDefault()
      return
    }
    if (e.code === 'Space' && !e.repeat) {
      this.spacePressed = true
      this.canvasEl.style.cursor = 'grab'
    }
  }

  private onKeyUp = (e: KeyboardEvent) => {
    if (e.code === 'Space') {
      this.spacePressed = false
      this.isPanning = false
      this.canvasEl.style.cursor = toolToCursor(this.getTool())
    }
  }

  // ---------------------------------------------------------------------------
  // Attach / detach event listeners
  // ---------------------------------------------------------------------------

  attach(): () => void {
    const canvasEl = this.canvasEl

    const onContextMenu = (e: MouseEvent) => e.preventDefault()

    // Tool change → cursor + cancel pen if switching away
    const unsubTool = useCanvasStore.subscribe((state) => {
      if (!this.spacePressed && !this.isResizing) canvasEl.style.cursor = toolToCursor(state.activeTool)
      this.penTool.onToolChange(state.activeTool)
    })

    document.addEventListener('keydown', this.onKeyDown)
    document.addEventListener('keyup', this.onKeyUp)
    canvasEl.addEventListener('mousedown', this.onMouseDown)
    canvasEl.addEventListener('dblclick', this.onDblClick)
    canvasEl.addEventListener('contextmenu', onContextMenu)
    window.addEventListener('mousemove', this.onMouseMove)
    window.addEventListener('mouseup', this.onMouseUp)

    return () => {
      document.removeEventListener('keydown', this.onKeyDown)
      document.removeEventListener('keyup', this.onKeyUp)
      canvasEl.removeEventListener('mousedown', this.onMouseDown)
      canvasEl.removeEventListener('dblclick', this.onDblClick)
      canvasEl.removeEventListener('contextmenu', onContextMenu)
      window.removeEventListener('mousemove', this.onMouseMove)
      window.removeEventListener('mouseup', this.onMouseUp)
      unsubTool()
    }
  }
}
