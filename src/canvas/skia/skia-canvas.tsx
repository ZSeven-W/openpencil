import { useRef, useEffect, useState } from 'react'
import { loadCanvasKit } from './skia-init'
import { SkiaEngine, screenToScene } from './skia-engine'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { createNodeForTool, isDrawingTool } from '../canvas-node-creator'
import { inferLayout } from '../canvas-layout-engine'
import { SkiaPenTool } from './skia-pen-tool'
import type { ToolType } from '@/types/canvas'

interface TextEditState {
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

function toolToCursor(tool: ToolType): string {
  switch (tool) {
    case 'hand': return 'grab'
    case 'text': return 'text'
    case 'select': return 'default'
    default: return 'crosshair'
  }
}

export default function SkiaCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const engineRef = useRef<SkiaEngine | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [editingText, setEditingText] = useState<TextEditState | null>(null)

  // Initialize CanvasKit + engine
  useEffect(() => {
    let disposed = false

    async function init() {
      try {
        const ck = await loadCanvasKit()
        if (disposed) return

        const canvasEl = canvasRef.current
        if (!canvasEl) return

        const engine = new SkiaEngine(ck)
        engine.init(canvasEl)
        engineRef.current = engine

        // Initial sync
        engine.syncFromDocument()
        requestAnimationFrame(() => engine.zoomToFitContent())

      } catch (err) {
        console.error('SkiaCanvas init failed:', err)
        setError(String(err))
      }
    }

    init()

    return () => {
      disposed = true
      engineRef.current?.dispose()
      engineRef.current = null
    }
  }, [])

  // Resize observer
  useEffect(() => {
    const container = containerRef.current
    if (!container) return
    const observer = new ResizeObserver((entries) => {
      const engine = engineRef.current
      if (!engine) return
      for (const entry of entries) {
        const { width, height } = entry.contentRect
        engine.resize(width, height)
      }
    })
    observer.observe(container)
    return () => observer.disconnect()
  }, [])

  // Document sync: re-render when document changes
  useEffect(() => {
    const unsub = useDocumentStore.subscribe(() => {
      engineRef.current?.syncFromDocument()
    })
    return unsub
  }, [])

  // Page sync: re-render when active page changes
  useEffect(() => {
    let prevPageId = useCanvasStore.getState().activePageId
    const unsub = useCanvasStore.subscribe((state) => {
      if (state.activePageId !== prevPageId) {
        prevPageId = state.activePageId
        engineRef.current?.syncFromDocument()
      }
    })
    return unsub
  }, [])

  // Selection sync: re-render when selection changes
  useEffect(() => {
    let prevIds = useCanvasStore.getState().selection.selectedIds
    const unsub = useCanvasStore.subscribe((state) => {
      if (state.selection.selectedIds !== prevIds) {
        prevIds = state.selection.selectedIds
        engineRef.current?.markDirty()
      }
    })
    return unsub
  }, [])

  // ---- Event handlers ----

  // Wheel: zoom + pan
  useEffect(() => {
    const canvasEl = canvasRef.current
    if (!canvasEl) return

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault()
      e.stopPropagation()
      const engine = engineRef.current
      if (!engine) return

      if (e.ctrlKey || e.metaKey) {
        let delta = -e.deltaY
        if (e.deltaMode === 1) delta *= 40
        const factor = Math.pow(1.005, delta)
        const newZoom = engine.zoom * factor
        engine.zoomToPoint(e.clientX, e.clientY, newZoom)
      } else {
        let dx = -e.deltaX
        let dy = -e.deltaY
        if (e.deltaMode === 1) { dx *= 40; dy *= 40 }
        engine.pan(dx, dy)
      }
    }

    canvasEl.addEventListener('wheel', handleWheel, { passive: false })
    return () => canvasEl.removeEventListener('wheel', handleWheel)
  }, [])

  // Mouse interactions: select, move, resize, draw, hover
  useEffect(() => {
    const canvasEl = canvasRef.current
    if (!canvasEl) return

    // --- Shared state ---
    let isPanning = false
    let spacePressed = false
    let lastX = 0
    let lastY = 0

    // --- Select tool state ---
    let isDragging = false
    let dragMoved = false
    let isMarquee = false
    let dragNodeIds: string[] = []
    let dragStartSceneX = 0
    let dragStartSceneY = 0
    let dragOrigPositions: { id: string; x: number; y: number }[] = []
    let dragPrevDx = 0
    let dragPrevDy = 0
    /** Set of node IDs being dragged (including descendants). */
    let dragAllIds: Set<string> | null = null
    const DRAG_THRESHOLD = 3

    // --- Resize handle state ---
    type HandleDir = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w'
    let isResizing = false
    let resizeHandle: HandleDir | null = null
    let resizeNodeId: string | null = null
    let resizeOrigX = 0
    let resizeOrigY = 0
    let resizeOrigW = 0
    let resizeOrigH = 0
    let resizeStartSceneX = 0
    let resizeStartSceneY = 0

    const HANDLE_HIT_RADIUS = 8 // larger hit area than visual
    const ROTATE_OUTER_RADIUS = 16 // rotation zone is outside corner handles

    const handleCursors: Record<HandleDir, string> = {
      nw: 'nwse-resize', n: 'ns-resize', ne: 'nesw-resize', e: 'ew-resize',
      se: 'nwse-resize', s: 'ns-resize', sw: 'nesw-resize', w: 'ew-resize',
    }

    // --- Rotation state ---
    let isRotating = false
    let rotateNodeId: string | null = null
    let rotateOrigAngle = 0
    let rotateCenterX = 0
    let rotateCenterY = 0
    let rotateStartAngle = 0

    // --- Drawing tool state ---
    let isDrawing = false
    let drawTool: ToolType = 'select'
    let drawStartX = 0
    let drawStartY = 0

    // --- Pen tool ---
    const penTool = new SkiaPenTool(() => engineRef.current)

    const getEngine = () => engineRef.current
    const getTool = () => useCanvasStore.getState().activeTool

    const getScene = (e: MouseEvent) => {
      const engine = getEngine()
      if (!engine) return null
      const rect = engine.getCanvasRect()
      if (!rect) return null
      return screenToScene(e.clientX, e.clientY, rect, {
        zoom: engine.zoom, panX: engine.panX, panY: engine.panY,
      })
    }

    /** Get the selected node's render info (single selection only). */
    const getSelectedRN = () => {
      const engine = getEngine()
      if (!engine) return null
      const { selectedIds } = useCanvasStore.getState().selection
      if (selectedIds.length !== 1) return null
      return engine.spatialIndex.get(selectedIds[0]) ?? null
    }

    /** Check if a scene point hits a resize handle of the selected node. */
    const hitTestHandle = (sceneX: number, sceneY: number): { dir: HandleDir; nodeId: string } | null => {
      const engine = getEngine()
      if (!engine) return null
      const rn = getSelectedRN()
      if (!rn) return null

      const hitR = HANDLE_HIT_RADIUS / engine.zoom
      const { absX: x, absY: y, absW: w, absH: h } = rn
      const handles: [HandleDir, number, number][] = [
        ['nw', x, y], ['n', x + w / 2, y], ['ne', x + w, y],
        ['w', x, y + h / 2], ['e', x + w, y + h / 2],
        ['sw', x, y + h], ['s', x + w / 2, y + h], ['se', x + w, y + h],
      ]
      for (const [dir, hx, hy] of handles) {
        if (Math.abs(sceneX - hx) <= hitR && Math.abs(sceneY - hy) <= hitR) {
          return { dir, nodeId: rn.node.id }
        }
      }
      return null
    }

    /** Check if a scene point is in the rotation zone (just outside corner handles). */
    const hitTestRotation = (sceneX: number, sceneY: number): { nodeId: string } | null => {
      const engine = getEngine()
      if (!engine) return null
      const rn = getSelectedRN()
      if (!rn) return null

      const innerR = HANDLE_HIT_RADIUS / engine.zoom
      const outerR = ROTATE_OUTER_RADIUS / engine.zoom
      const { absX: x, absY: y, absW: w, absH: h } = rn
      const corners = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]]
      for (const [cx, cy] of corners) {
        const dist = Math.hypot(sceneX - cx, sceneY - cy)
        if (dist > innerR && dist <= outerR) {
          return { nodeId: rn.node.id }
        }
      }
      return null
    }

    // --- Keyboard: space for panning ---
    const onKeyDown = (e: KeyboardEvent) => {
      // Pen tool keyboard shortcuts
      if (penTool.onKeyDown(e.key)) {
        e.preventDefault()
        return
      }

      if (e.code === 'Space' && !e.repeat) {
        spacePressed = true
        canvasEl.style.cursor = 'grab'
      }
    }
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.code === 'Space') {
        spacePressed = false
        isPanning = false
        canvasEl.style.cursor = toolToCursor(getTool())
      }
    }
    document.addEventListener('keydown', onKeyDown)
    document.addEventListener('keyup', onKeyUp)

    // Tool change → cursor + cancel pen if switching away
    const unsubTool = useCanvasStore.subscribe((state) => {
      if (!spacePressed && !isResizing) canvasEl.style.cursor = toolToCursor(state.activeTool)
      penTool.onToolChange(state.activeTool)
    })

    // =====================================================================
    // MOUSE DOWN
    // =====================================================================
    const onMouseDown = (e: MouseEvent) => {
      const engine = getEngine()
      if (!engine) return

      // Ignore right-click — only handle left (0) and middle (1)
      if (e.button === 2) return

      // --- Pan: space+click, hand tool, or middle mouse ---
      if (spacePressed || getTool() === 'hand' || e.button === 1) {
        isPanning = true
        lastX = e.clientX
        lastY = e.clientY
        canvasEl.style.cursor = 'grabbing'
        return
      }

      const tool = getTool()
      const scene = getScene(e)
      if (!scene) return

      // --- Text tool: click to create immediately ---
      if (tool === 'text') {
        const node = createNodeForTool('text', scene.x, scene.y, 0, 0)
        if (node) {
          useDocumentStore.getState().addNode(null, node)
          useCanvasStore.getState().setSelection([node.id], node.id)
        }
        useCanvasStore.getState().setActiveTool('select')
        return
      }

      // --- Pen tool ---
      if (tool === 'path') {
        penTool.onMouseDown(scene, engine.zoom || 1)
        return
      }

      // --- Drawing tools: start rubber-band ---
      if (isDrawingTool(tool)) {
        isDrawing = true
        drawTool = tool
        drawStartX = scene.x
        drawStartY = scene.y
        engine.previewShape = {
          type: tool as any,
          x: scene.x, y: scene.y, w: 0, h: 0,
        }
        engine.markDirty()
        return
      }

      // --- Select tool ---
      if (tool === 'select') {
        // Check resize handle first (only for single selection)
        const handleHit = hitTestHandle(scene.x, scene.y)
        if (handleHit) {
          isResizing = true
          resizeHandle = handleHit.dir
          resizeNodeId = handleHit.nodeId
          resizeStartSceneX = scene.x
          resizeStartSceneY = scene.y
          const docNode = useDocumentStore.getState().getNodeById(handleHit.nodeId)
          resizeOrigX = docNode?.x ?? 0
          resizeOrigY = docNode?.y ?? 0
          // Use resolved dimensions from spatial index — document store may have
          // string values like 'fill_container' that break arithmetic.
          const resizeRN = engine.spatialIndex.get(handleHit.nodeId)
          resizeOrigW = resizeRN?.absW ?? (typeof (docNode as any)?.width === 'number' ? (docNode as any).width : 100)
          resizeOrigH = resizeRN?.absH ?? (typeof (docNode as any)?.height === 'number' ? (docNode as any).height : 100)
          canvasEl.style.cursor = handleCursors[handleHit.dir]
          return
        }

        // Check rotation zone (just outside corner handles)
        const rotHit = hitTestRotation(scene.x, scene.y)
        if (rotHit) {
          isRotating = true
          rotateNodeId = rotHit.nodeId
          const docNode = useDocumentStore.getState().getNodeById(rotHit.nodeId)
          rotateOrigAngle = docNode?.rotation ?? 0
          const rn = getSelectedRN()!
          rotateCenterX = rn.absX + rn.absW / 2
          rotateCenterY = rn.absY + rn.absH / 2
          rotateStartAngle = Math.atan2(scene.y - rotateCenterY, scene.x - rotateCenterX) * 180 / Math.PI
          canvasEl.style.cursor = 'grabbing'
          return
        }

        const hits = engine.spatialIndex.hitTest(scene.x, scene.y)

        if (hits.length > 0) {
          const topHit = hits[0]
          let nodeId = topHit.node.id
          const currentSelection = useCanvasStore.getState().selection.selectedIds
          const docStore = useDocumentStore.getState()

          // If the hit node is a descendant of an already-selected node,
          // keep the current selection so the whole group moves together.
          const isChildOfSelected = currentSelection.some(
            (selId) => selId !== nodeId && docStore.isDescendantOf(nodeId, selId),
          )
          if (isChildOfSelected) {
            // Don't change selection — proceed to drag the selected parent
          } else if (!currentSelection.includes(nodeId)) {
            // Walk up the tree to find the top-level parent (group behavior):
            // clicking a child inside a group selects the group, not the child.
            const parent = docStore.getParentOf(nodeId)
            if (parent && (parent.type === 'frame' || parent.type === 'group')) {
              // Check if parent is a top-level node (its parent is the page root)
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

          // Start drag — move all selected nodes
          const selectedIds = useCanvasStore.getState().selection.selectedIds
          isDragging = true
          dragMoved = false
          dragNodeIds = selectedIds
          dragStartSceneX = scene.x
          dragStartSceneY = scene.y
          dragOrigPositions = selectedIds.map((id) => {
            const n = useDocumentStore.getState().getNodeById(id)
            return { id, x: n?.x ?? 0, y: n?.y ?? 0 }
          })
        } else {
          // Empty space → start marquee or clear selection
          if (!e.shiftKey) {
            useCanvasStore.getState().clearSelection()
          }
          isMarquee = true
          lastX = scene.x
          lastY = scene.y
          engine.marquee = { x1: scene.x, y1: scene.y, x2: scene.x, y2: scene.y }
        }
      }
    }

    // =====================================================================
    // MOUSE MOVE
    // =====================================================================
    const onMouseMove = (e: MouseEvent) => {
      const engine = getEngine()
      if (!engine) return

      if (isPanning) {
        const dx = e.clientX - lastX
        const dy = e.clientY - lastY
        lastX = e.clientX
        lastY = e.clientY
        engine.pan(dx, dy)
        return
      }

      const scene = getScene(e)
      if (!scene) return

      // --- Pen tool move ---
      if (penTool.onMouseMove(scene)) return

      // --- Resize handle drag ---
      if (isResizing && resizeHandle && resizeNodeId) {
        const dx = scene.x - resizeStartSceneX
        const dy = scene.y - resizeStartSceneY
        let newX = resizeOrigX
        let newY = resizeOrigY
        let newW = resizeOrigW
        let newH = resizeOrigH

        // Compute new bounds based on which handle is dragged
        const dir = resizeHandle
        if (dir.includes('w')) { newX = resizeOrigX + dx; newW = resizeOrigW - dx }
        if (dir.includes('e')) { newW = resizeOrigW + dx }
        if (dir.includes('n')) { newY = resizeOrigY + dy; newH = resizeOrigH - dy }
        if (dir.includes('s')) { newH = resizeOrigH + dy }

        // Enforce minimum size
        const MIN = 2
        if (newW < MIN) { if (dir.includes('w')) newX = resizeOrigX + resizeOrigW - MIN; newW = MIN }
        if (newH < MIN) { if (dir.includes('n')) newY = resizeOrigY + resizeOrigH - MIN; newH = MIN }

        // For text nodes, switching to fixed-width enables auto-wrap
        const resizedNode = useDocumentStore.getState().getNodeById(resizeNodeId)
        const updates: Record<string, unknown> = { x: newX, y: newY, width: newW, height: newH }
        if (resizedNode?.type === 'text' && !(resizedNode as any).textGrowth) {
          updates.textGrowth = 'fixed-width'
        }
        useDocumentStore.getState().updateNode(resizeNodeId, updates as any)

        // Scale children proportionally when resizing a container with children
        if (
          resizedNode
          && 'children' in resizedNode
          && resizedNode.children?.length
        ) {
          // Use resolved dimensions — document store may have string values
          const resizeRN2 = engine.spatialIndex.get(resizeNodeId)
          const oldW = resizeRN2?.absW ?? (typeof (resizedNode as any).width === 'number' ? (resizedNode as any).width : 0)
          const oldH = resizeRN2?.absH ?? (typeof (resizedNode as any).height === 'number' ? (resizedNode as any).height : 0)
          if (oldW > 0 && oldH > 0) {
            const scaleX = newW / oldW
            const scaleY = newH / oldH
            useDocumentStore.getState().scaleDescendantsInStore(resizeNodeId, scaleX, scaleY)
          }
        }
        return
      }

      // --- Rotation drag ---
      if (isRotating && rotateNodeId) {
        const currentAngle = Math.atan2(scene.y - rotateCenterY, scene.x - rotateCenterX) * 180 / Math.PI
        let newAngle = rotateOrigAngle + (currentAngle - rotateStartAngle)
        // Snap to 15° increments when holding shift
        if (e.shiftKey) {
          newAngle = Math.round(newAngle / 15) * 15
        }
        useDocumentStore.getState().updateNode(rotateNodeId, { rotation: newAngle } as any)
        return
      }

      // --- Drawing tool preview ---
      if (isDrawing && engine.previewShape) {
        const dx = scene.x - drawStartX
        const dy = scene.y - drawStartY

        if (drawTool === 'line') {
          engine.previewShape = {
            type: 'line',
            x: drawStartX, y: drawStartY,
            w: dx, h: dy,
          }
        } else {
          // Rectangle / ellipse / frame: handle negative drag direction
          engine.previewShape = {
            type: drawTool as any,
            x: dx < 0 ? scene.x : drawStartX,
            y: dy < 0 ? scene.y : drawStartY,
            w: Math.abs(dx),
            h: Math.abs(dy),
          }
        }
        engine.markDirty()
        return
      }

      // --- Select tool: drag move (all selected nodes) ---
      if (isDragging && dragNodeIds.length > 0) {
        const dx = scene.x - dragStartSceneX
        const dy = scene.y - dragStartSceneY

        if (!dragMoved) {
          const screenDist = Math.hypot(dx * engine.zoom, dy * engine.zoom)
          if (screenDist < DRAG_THRESHOLD) return
          dragMoved = true
          // Suppress store→sync loop so layout engine doesn't override positions
          engine.dragSyncSuppressed = true
          dragPrevDx = 0
          dragPrevDy = 0
          // Collect all node IDs being moved (selected + their descendants)
          dragAllIds = new Set(dragNodeIds)
          for (const id of dragNodeIds) {
            const collectDescs = (nodeId: string) => {
              const n = useDocumentStore.getState().getNodeById(nodeId)
              if (n && 'children' in n && n.children) {
                for (const child of n.children) {
                  dragAllIds!.add(child.id)
                  collectDescs(child.id)
                }
              }
            }
            collectDescs(id)
          }
        }

        // Apply incremental delta directly to render nodes for immediate feedback
        const incrDx = dx - dragPrevDx
        const incrDy = dy - dragPrevDy
        dragPrevDx = dx
        dragPrevDy = dy

        for (const rn of engine.renderNodes) {
          if (dragAllIds!.has(rn.node.id)) {
            rn.absX += incrDx
            rn.absY += incrDy
            rn.node = { ...rn.node, x: rn.absX, y: rn.absY }
          }
        }
        engine.spatialIndex.rebuild(engine.renderNodes)
        engine.markDirty()
        return
      }

      // --- Marquee ---
      if (isMarquee && engine.marquee) {
        engine.marquee.x2 = scene.x
        engine.marquee.y2 = scene.y
        engine.markDirty()

        const marqueeHits = engine.spatialIndex.searchRect(
          engine.marquee.x1, engine.marquee.y1,
          engine.marquee.x2, engine.marquee.y2,
        )
        const ids = marqueeHits.map((rn) => rn.node.id)
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null)
        return
      }

      // --- Hover + handle cursor (select tool only) ---
      if (getTool() === 'select' && !spacePressed) {
        // Check handle hover for cursor
        const handleHit = hitTestHandle(scene.x, scene.y)
        if (handleHit) {
          canvasEl.style.cursor = handleCursors[handleHit.dir]
        } else if (hitTestRotation(scene.x, scene.y)) {
          // Rotation cursor — rotate icon via CSS
          canvasEl.style.cursor = 'url("data:image/svg+xml,%3Csvg xmlns=\'http://www.w3.org/2000/svg\' width=\'24\' height=\'24\' viewBox=\'0 0 24 24\' fill=\'none\' stroke=\'black\' stroke-width=\'2\'%3E%3Cpath d=\'M21 2v6h-6\'/%3E%3Cpath d=\'M21 13a9 9 0 1 1-3-7.7L21 8\'/%3E%3C/svg%3E") 12 12, crosshair'
        } else {
          const hoverHits = engine.spatialIndex.hitTest(scene.x, scene.y)
          const newHoveredId = hoverHits.length > 0 ? hoverHits[0].node.id : null
          canvasEl.style.cursor = newHoveredId ? 'move' : 'default'
          if (newHoveredId !== engine.hoveredNodeId) {
            engine.hoveredNodeId = newHoveredId
            useCanvasStore.getState().setHoveredId(newHoveredId)
            engine.markDirty()
          }
        }
      }
    }

    // =====================================================================
    // MOUSE UP
    // =====================================================================
    const onMouseUp = () => {
      const engine = getEngine()

      // --- Pen tool: end handle drag ---
      if (penTool.onMouseUp()) return

      // --- Pan end ---
      if (isPanning) {
        isPanning = false
        canvasEl.style.cursor = spacePressed ? 'grab' : toolToCursor(getTool())
      }

      // --- Resize end ---
      if (isResizing) {
        isResizing = false
        resizeHandle = null
        resizeNodeId = null
        canvasEl.style.cursor = toolToCursor(getTool())
      }

      // --- Rotation end ---
      if (isRotating) {
        isRotating = false
        rotateNodeId = null
        canvasEl.style.cursor = toolToCursor(getTool())
      }

      // --- Drawing tool: create node ---
      if (isDrawing && engine?.previewShape) {
        const { type, x, y, w, h } = engine.previewShape
        engine.previewShape = null
        engine.markDirty()
        isDrawing = false

        // Ignore tiny accidental clicks (< 2px)
        const minSize = type === 'line'
          ? Math.hypot(w, h) >= 2
          : w >= 2 && h >= 2
        if (minSize) {
          const node = createNodeForTool(drawTool, x, y, w, h)
          if (node) {
            useDocumentStore.getState().addNode(null, node)
            useCanvasStore.getState().setSelection([node.id], node.id)
          }
        }
        useCanvasStore.getState().setActiveTool('select')
        return
      }
      isDrawing = false

      // --- Select tool: end drag / marquee ---
      if (isDragging && dragMoved && dragOrigPositions.length > 0 && engine) {
        const dx = dragPrevDx
        const dy = dragPrevDy
        const docStore = useDocumentStore.getState()

        for (const orig of dragOrigPositions) {
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
                // Reparent to root level with absolute position
                docStore.updateNode(orig.id, { x: objBounds.x, y: objBounds.y } as any)
                docStore.moveNode(orig.id, null, 0)
                continue
              }
            }
          }

          // Check if node is inside a layout container
          const parentLayout = parent
            ? ((parent as any).layout || inferLayout(parent))
            : undefined

          if (parentLayout && parentLayout !== 'none' && parent) {
            // Layout child: reorder based on drop position
            const siblings = ('children' in parent ? parent.children ?? [] : [])
              .filter((c: any) => c.id !== orig.id)
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
            // Non-layout node: freely set position
            docStore.updateNode(orig.id, {
              x: orig.x + dx,
              y: orig.y + dy,
            } as any)
          }
        }

        // Re-enable sync and do a full rebuild
        engine.dragSyncSuppressed = false
        engine.syncFromDocument()
      } else if (engine) {
        engine.dragSyncSuppressed = false
      }
      isDragging = false
      dragNodeIds = []
      dragOrigPositions = []
      dragAllIds = null
      if (isMarquee && engine) {
        engine.marquee = null
        engine.markDirty()
      }
      isMarquee = false
    }

    // =====================================================================
    // DOUBLE CLICK — text editing
    // =====================================================================
    const onDblClick = (e: MouseEvent) => {
      const engine = getEngine()
      if (!engine) return

      // Pen tool: double-click finalizes the path (open)
      if (penTool.onDblClick()) return

      if (getTool() !== 'select') return

      const scene = getScene(e)
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
          // Find the direct child under the cursor
          const childId = topHit.node.id
          if (childId !== currentSelection[0]) {
            useCanvasStore.getState().setSelection([childId], childId)
            return
          }
        }
      }

      if (topHit.node.type !== 'text') return

      const tNode = topHit.node as any
      const fills = tNode.fill
      const color = typeof fills === 'string'
        ? fills
        : (Array.isArray(fills) && fills[0]?.color ? fills[0].color : '#000000')

      setEditingText({
        nodeId: topHit.node.id,
        x: topHit.absX * engine.zoom + engine.panX,
        y: topHit.absY * engine.zoom + engine.panY,
        w: topHit.absW * engine.zoom,
        h: topHit.absH * engine.zoom,
        content: typeof tNode.content === 'string'
          ? tNode.content
          : Array.isArray(tNode.content)
            ? tNode.content.map((s: any) => s.text ?? '').join('')
            : '',
        fontSize: (tNode.fontSize ?? 16) * engine.zoom,
        fontFamily: tNode.fontFamily ?? 'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif',
        fontWeight: String(tNode.fontWeight ?? '400'),
        textAlign: tNode.textAlign ?? 'left',
        color,
        lineHeight: tNode.lineHeight ?? 1.4,
      })
    }

    const onContextMenu = (e: MouseEvent) => e.preventDefault()

    canvasEl.addEventListener('mousedown', onMouseDown)
    canvasEl.addEventListener('dblclick', onDblClick)
    canvasEl.addEventListener('contextmenu', onContextMenu)
    window.addEventListener('mousemove', onMouseMove)
    window.addEventListener('mouseup', onMouseUp)

    return () => {
      document.removeEventListener('keydown', onKeyDown)
      document.removeEventListener('keyup', onKeyUp)
      canvasEl.removeEventListener('mousedown', onMouseDown)
      canvasEl.removeEventListener('dblclick', onDblClick)
      canvasEl.removeEventListener('contextmenu', onContextMenu)
      window.removeEventListener('mousemove', onMouseMove)
      window.removeEventListener('mouseup', onMouseUp)
      unsubTool()
    }
  }, [])

  return (
    <div
      ref={containerRef}
      className="flex-1 relative overflow-hidden bg-muted"
    >
      <canvas
        ref={canvasRef}
        className="absolute inset-0 w-full h-full"
      />
      {editingText && (
        <textarea
          autoFocus
          defaultValue={editingText.content}
          style={{
            position: 'absolute',
            left: editingText.x,
            top: editingText.y,
            width: Math.max(editingText.w, 40),
            minHeight: Math.max(editingText.h, 24),
            fontSize: editingText.fontSize,
            fontFamily: editingText.fontFamily,
            fontWeight: editingText.fontWeight,
            textAlign: editingText.textAlign as CanvasTextAlign,
            color: editingText.color,
            lineHeight: editingText.lineHeight,
            background: 'rgba(255,255,255,0.9)',
            border: '2px solid #0d99ff',
            borderRadius: 2,
            outline: 'none',
            resize: 'none',
            padding: '0 1px',
            margin: 0,
            overflow: 'hidden',
            zIndex: 10,
            boxSizing: 'border-box',
          }}
          onBlur={(e) => {
            const newContent = e.target.value
            if (newContent !== editingText.content) {
              useDocumentStore.getState().updateNode(editingText.nodeId, { content: newContent } as any)
            }
            setEditingText(null)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              setEditingText(null)
            }
            e.stopPropagation()
          }}
        />
      )}

      {error && (
        <div className="absolute inset-0 flex items-center justify-center text-destructive">
          Failed to load CanvasKit: {error}
        </div>
      )}
    </div>
  )
}
