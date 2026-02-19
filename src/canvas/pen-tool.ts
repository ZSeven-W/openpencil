import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, generateId } from '@/stores/document-store'
import {
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
  SELECTION_BLUE,
} from './canvas-constants'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PenAnchorPoint {
  x: number
  y: number
  /** Incoming control handle (offset relative to anchor). null = straight. */
  handleIn: { x: number; y: number } | null
  /** Outgoing control handle (offset relative to anchor). null = straight. */
  handleOut: { x: number; y: number } | null
}

interface PenToolState {
  isActive: boolean
  points: PenAnchorPoint[]
  isDraggingHandle: boolean
  cursorPos: { x: number; y: number } | null
}

// ---------------------------------------------------------------------------
// Module-level state
// ---------------------------------------------------------------------------

let state: PenToolState = {
  isActive: false,
  points: [],
  isDraggingHandle: false,
  cursorPos: null,
}

// Temporary Fabric objects for visual feedback
let previewPath: fabric.FabricObject | null = null
let rubberBandLine: fabric.FabricObject | null = null
let anchorCircles: fabric.FabricObject[] = []
let handleLines: fabric.FabricObject[] = []
let handleDots: fabric.FabricObject[] = []

// ---------------------------------------------------------------------------
// Visual constants
// ---------------------------------------------------------------------------

const PREVIEW_STROKE = SELECTION_BLUE
const PREVIEW_STROKE_WIDTH = 1.5
const RUBBER_BAND_STROKE = 'rgba(13, 153, 255, 0.5)'
const RUBBER_BAND_DASH = [4, 4]
const ANCHOR_RADIUS = 4
const ANCHOR_FILL = '#ffffff'
const ANCHOR_STROKE = SELECTION_BLUE
const ANCHOR_FIRST_RADIUS = 5
const HANDLE_DOT_RADIUS = 3
const HANDLE_LINE_STROKE = '#888888'
const CLOSE_HIT_THRESHOLD = 8 // screen pixels

// ---------------------------------------------------------------------------
// Exported query
// ---------------------------------------------------------------------------

export function isPenToolActive(): boolean {
  return state.isActive
}

// ---------------------------------------------------------------------------
// Exported event handlers
// ---------------------------------------------------------------------------

export function penToolPointerDown(
  canvas: fabric.Canvas,
  scenePoint: { x: number; y: number },
): void {
  if (!state.isActive) {
    // First click — start a new path
    state.isActive = true
    state.points = [
      { x: scenePoint.x, y: scenePoint.y, handleIn: null, handleOut: null },
    ]
    state.isDraggingHandle = true
    state.cursorPos = scenePoint
    renderPreview(canvas)
    return
  }

  // Check if clicking near the first point to close the path
  if (state.points.length >= 3) {
    const first = state.points[0]
    const zoom = useCanvasStore.getState().viewport.zoom || 1
    const threshold = CLOSE_HIT_THRESHOLD / zoom
    const dist = Math.hypot(
      scenePoint.x - first.x,
      scenePoint.y - first.y,
    )
    if (dist < threshold) {
      finalizePath(canvas, true)
      return
    }
  }

  // Add a new anchor point
  state.points.push({
    x: scenePoint.x,
    y: scenePoint.y,
    handleIn: null,
    handleOut: null,
  })
  state.isDraggingHandle = true
  renderPreview(canvas)
}

export function penToolPointerMove(
  canvas: fabric.Canvas,
  scenePoint: { x: number; y: number },
): void {
  if (!state.isActive) return

  // Ignore if panning
  const { isPanning } = useCanvasStore.getState().interaction
  if (isPanning) return

  if (state.isDraggingHandle && state.points.length > 0) {
    // Update handle for the current (last) point
    const pt = state.points[state.points.length - 1]
    const dx = scenePoint.x - pt.x
    const dy = scenePoint.y - pt.y

    // Only set handles if the drag is significant (> 2px)
    if (Math.hypot(dx, dy) > 2) {
      pt.handleOut = { x: dx, y: dy }
      pt.handleIn = { x: -dx, y: -dy }
    } else {
      pt.handleOut = null
      pt.handleIn = null
    }
  }

  state.cursorPos = scenePoint
  renderPreview(canvas)
}

export function penToolPointerUp(canvas: fabric.Canvas): void {
  if (!state.isActive) return
  state.isDraggingHandle = false
  renderPreview(canvas)
}

export function penToolDoubleClick(canvas: fabric.Canvas): void {
  if (!state.isActive) return

  // The double-click adds an extra point from the second click of the pair.
  // Remove it so we don't get a duplicate degenerate segment.
  if (state.points.length > 1) {
    state.points.pop()
  }

  finalizePath(canvas, false)
}

/**
 * Handle keyboard events during pen drawing.
 * Returns true if the event was consumed.
 */
export function penToolKeyDown(
  canvas: fabric.Canvas,
  key: string,
): boolean {
  if (!state.isActive) return false

  switch (key) {
    case 'Enter':
      finalizePath(canvas, false)
      return true

    case 'Escape':
      cancelPenTool(canvas)
      return true

    case 'Backspace': {
      if (state.points.length > 1) {
        state.points.pop()
        renderPreview(canvas)
      } else {
        // Only one point left — cancel entirely
        cancelPenTool(canvas)
      }
      return true
    }

    default:
      return false
  }
}

export function cancelPenTool(canvas: fabric.Canvas): void {
  clearPreviewObjects(canvas)
  state = {
    isActive: false,
    points: [],
    isDraggingHandle: false,
    cursorPos: null,
  }
  useCanvasStore.getState().setActiveTool('select')
}

// ---------------------------------------------------------------------------
// Internal: path data construction
// ---------------------------------------------------------------------------

function buildPathData(
  points: PenAnchorPoint[],
  closed: boolean,
): string {
  if (points.length === 0) return ''

  const parts: string[] = []
  const first = points[0]
  parts.push(`M ${first.x} ${first.y}`)

  for (let i = 1; i < points.length; i++) {
    const prev = points[i - 1]
    const curr = points[i]
    appendSegment(parts, prev, curr)
  }

  if (closed && points.length > 1) {
    // Close segment from last point back to first
    const last = points[points.length - 1]
    appendSegment(parts, last, first)
    parts.push('Z')
  }

  return parts.join(' ')
}

function appendSegment(
  parts: string[],
  from: PenAnchorPoint,
  to: PenAnchorPoint,
): void {
  const hasHandleOut = from.handleOut !== null
  const hasHandleIn = to.handleIn !== null

  if (!hasHandleOut && !hasHandleIn) {
    // Straight line
    parts.push(`L ${to.x} ${to.y}`)
  } else {
    // Cubic bezier
    const cx1 = from.x + (from.handleOut?.x ?? 0)
    const cy1 = from.y + (from.handleOut?.y ?? 0)
    const cx2 = to.x + (to.handleIn?.x ?? 0)
    const cy2 = to.y + (to.handleIn?.y ?? 0)
    parts.push(`C ${cx1} ${cy1} ${cx2} ${cy2} ${to.x} ${to.y}`)
  }
}

// ---------------------------------------------------------------------------
// Internal: bounding box via browser SVG engine
// ---------------------------------------------------------------------------

function getPathBBox(d: string): {
  x: number
  y: number
  w: number
  h: number
} {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
  const pathEl = document.createElementNS(
    'http://www.w3.org/2000/svg',
    'path',
  )
  pathEl.setAttribute('d', d)
  svg.appendChild(pathEl)
  document.body.appendChild(svg)
  const bbox = pathEl.getBBox()
  document.body.removeChild(svg)
  return { x: bbox.x, y: bbox.y, w: bbox.width, h: bbox.height }
}

// ---------------------------------------------------------------------------
// Internal: finalize path into a PenNode
// ---------------------------------------------------------------------------

function finalizePath(canvas: fabric.Canvas, closed: boolean): void {
  clearPreviewObjects(canvas)

  // Need at least 2 points for a valid path
  if (state.points.length < 2) {
    state = {
      isActive: false,
      points: [],
      isDraggingHandle: false,
      cursorPos: null,
    }
    useCanvasStore.getState().setActiveTool('select')
    return
  }

  // Build path data in absolute scene coordinates
  const absD = buildPathData(state.points, closed)
  const bbox = getPathBBox(absD)

  // Guard against degenerate paths
  if (bbox.w < 1 && bbox.h < 1) {
    state = {
      isActive: false,
      points: [],
      isDraggingHandle: false,
      cursorPos: null,
    }
    useCanvasStore.getState().setActiveTool('select')
    return
  }

  // Normalize: translate all points so the path origin is at (0,0)
  const normalized = state.points.map((pt) => ({
    ...pt,
    x: pt.x - bbox.x,
    y: pt.y - bbox.y,
  }))
  const d = buildPathData(normalized, closed)

  useDocumentStore.getState().addNode(null, {
    id: generateId(),
    type: 'path',
    name: 'Path',
    x: bbox.x,
    y: bbox.y,
    d,
    width: Math.round(bbox.w),
    height: Math.round(bbox.h),
    fill: [{ type: 'solid', color: 'transparent' }],
    stroke: {
      thickness: DEFAULT_STROKE_WIDTH,
      fill: [{ type: 'solid', color: DEFAULT_STROKE }],
    },
  })

  state = {
    isActive: false,
    points: [],
    isDraggingHandle: false,
    cursorPos: null,
  }
  useCanvasStore.getState().setActiveTool('select')
}

// ---------------------------------------------------------------------------
// Internal: visual preview rendering
// ---------------------------------------------------------------------------

function clearPreviewObjects(canvas: fabric.Canvas): void {
  const objs = [
    previewPath,
    rubberBandLine,
    ...anchorCircles,
    ...handleLines,
    ...handleDots,
  ].filter(Boolean) as fabric.FabricObject[]

  for (const obj of objs) {
    canvas.remove(obj)
  }

  previewPath = null
  rubberBandLine = null
  anchorCircles = []
  handleLines = []
  handleDots = []
}

function renderPreview(canvas: fabric.Canvas): void {
  clearPreviewObjects(canvas)

  const { points, cursorPos } = state
  if (points.length === 0) return

  const baseProps = {
    selectable: false,
    evented: false,
    objectCaching: false,
    originX: 'left' as const,
    originY: 'top' as const,
    excludeFromExport: true,
  }

  // --- Preview path (constructed segments so far) ---
  if (points.length > 1) {
    const d = buildPathData(points, false)
    try {
      previewPath = new fabric.Path(d, {
        ...baseProps,
        fill: 'transparent',
        stroke: PREVIEW_STROKE,
        strokeWidth: PREVIEW_STROKE_WIDTH,
        strokeUniform: true,
      })
      canvas.add(previewPath)
    } catch {
      // Invalid path data — skip preview
    }
  }

  // --- Rubber-band line from last point to cursor ---
  const last = points[points.length - 1]
  if (cursorPos && !state.isDraggingHandle) {
    rubberBandLine = new fabric.Line(
      [last.x, last.y, cursorPos.x, cursorPos.y],
      {
        ...baseProps,
        fill: '',
        stroke: RUBBER_BAND_STROKE,
        strokeWidth: 1,
        strokeDashArray: RUBBER_BAND_DASH,
        strokeUniform: true,
      },
    )
    canvas.add(rubberBandLine)
  }

  // --- Anchor circles ---
  for (let i = 0; i < points.length; i++) {
    const pt = points[i]
    const isFirst = i === 0
    const r = isFirst ? ANCHOR_FIRST_RADIUS : ANCHOR_RADIUS
    const circle = new fabric.Circle({
      ...baseProps,
      left: pt.x - r,
      top: pt.y - r,
      radius: r,
      fill: ANCHOR_FILL,
      stroke: ANCHOR_STROKE,
      strokeWidth: 1.5,
      strokeUniform: true,
    })
    anchorCircles.push(circle)
    canvas.add(circle)
  }

  // --- Handle lines and dots ---
  for (const pt of points) {
    if (pt.handleOut) {
      const hx = pt.x + pt.handleOut.x
      const hy = pt.y + pt.handleOut.y
      const line = new fabric.Line([pt.x, pt.y, hx, hy], {
        ...baseProps,
        fill: '',
        stroke: HANDLE_LINE_STROKE,
        strokeWidth: 1,
        strokeUniform: true,
      })
      handleLines.push(line)
      canvas.add(line)

      const dot = new fabric.Circle({
        ...baseProps,
        left: hx - HANDLE_DOT_RADIUS,
        top: hy - HANDLE_DOT_RADIUS,
        radius: HANDLE_DOT_RADIUS,
        fill: SELECTION_BLUE,
        stroke: '#ffffff',
        strokeWidth: 1,
        strokeUniform: true,
      })
      handleDots.push(dot)
      canvas.add(dot)
    }

    if (pt.handleIn) {
      const hx = pt.x + pt.handleIn.x
      const hy = pt.y + pt.handleIn.y
      const line = new fabric.Line([pt.x, pt.y, hx, hy], {
        ...baseProps,
        fill: '',
        stroke: HANDLE_LINE_STROKE,
        strokeWidth: 1,
        strokeUniform: true,
      })
      handleLines.push(line)
      canvas.add(line)

      const dot = new fabric.Circle({
        ...baseProps,
        left: hx - HANDLE_DOT_RADIUS,
        top: hy - HANDLE_DOT_RADIUS,
        radius: HANDLE_DOT_RADIUS,
        fill: SELECTION_BLUE,
        stroke: '#ffffff',
        strokeWidth: 1,
        strokeUniform: true,
      })
      handleDots.push(dot)
      canvas.add(dot)
    }
  }

  canvas.renderAll()
}
