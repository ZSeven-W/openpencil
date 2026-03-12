import {
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
  PEN_CLOSE_HIT_THRESHOLD,
} from '../canvas-constants'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, generateId } from '@/stores/document-store'
import type { PenAnchor } from './skia-overlays'
import type { SkiaEngine } from './skia-engine'

export class SkiaPenTool {
  private penActive = false
  private penPoints: PenAnchor[] = []
  private penDraggingHandle = false
  private penCursorPos: { x: number; y: number } | null = null
  private engineGetter: () => SkiaEngine | null

  constructor(engineGetter: () => SkiaEngine | null) {
    this.engineGetter = engineGetter
  }

  get isActive(): boolean {
    return this.penActive
  }

  cancel(): boolean {
    if (!this.penActive) return false
    this.penActive = false
    this.penPoints = []
    this.penDraggingHandle = false
    this.penCursorPos = null
    const engine = this.engineGetter()
    if (engine) { engine.penPreview = null; engine.markDirty() }
    useCanvasStore.getState().setActiveTool('select')
    return true
  }

  onToolChange(tool: string): void {
    if (this.penActive && tool !== 'path') {
      this.penActive = false
      this.penPoints = []
      this.penDraggingHandle = false
      this.penCursorPos = null
      const engine = this.engineGetter()
      if (engine) { engine.penPreview = null; engine.markDirty() }
    }
  }

  onMouseDown(scene: { x: number; y: number }, zoom: number): boolean {
    if (!this.penActive) {
      // First click — start a new path
      this.penActive = true
      this.penPoints = [{ x: scene.x, y: scene.y, handleIn: null, handleOut: null }]
      this.penDraggingHandle = true
      this.penCursorPos = scene
      this.updatePenPreview()
      return true
    }

    // Check if clicking near the first point to close the path
    if (this.penPoints.length >= 3) {
      const first = this.penPoints[0]
      const threshold = PEN_CLOSE_HIT_THRESHOLD / zoom
      if (Math.hypot(scene.x - first.x, scene.y - first.y) < threshold) {
        this.finalizePen(true)
        return true
      }
    }

    // Add a new anchor point
    this.penPoints.push({ x: scene.x, y: scene.y, handleIn: null, handleOut: null })
    this.penDraggingHandle = true
    this.updatePenPreview()
    return true
  }

  onMouseMove(scene: { x: number; y: number }): boolean {
    if (!this.penActive || this.penPoints.length === 0) return false

    if (this.penDraggingHandle) {
      const pt = this.penPoints[this.penPoints.length - 1]
      const dx = scene.x - pt.x
      const dy = scene.y - pt.y
      if (Math.hypot(dx, dy) > 2) {
        pt.handleOut = { x: dx, y: dy }
        pt.handleIn = { x: -dx, y: -dy }
      } else {
        pt.handleOut = null
        pt.handleIn = null
      }
    }
    this.penCursorPos = scene
    this.updatePenPreview()
    return true
  }

  onMouseUp(): boolean {
    if (!this.penActive) return false
    this.penDraggingHandle = false
    this.updatePenPreview()
    return true
  }

  onDblClick(): boolean {
    if (!this.penActive) return false
    // Remove the extra point added by the second click of the double-click
    if (this.penPoints.length > 1) this.penPoints.pop()
    this.finalizePen(false)
    return true
  }

  onKeyDown(key: string): boolean {
    if (!this.penActive) return false

    if (key === 'Enter') {
      this.finalizePen(false)
      return true
    }
    if (key === 'Escape') {
      this.cancel()
      return true
    }
    if (key === 'Backspace') {
      if (this.penPoints.length > 1) {
        this.penPoints.pop()
        this.updatePenPreview()
      } else {
        this.cancel()
      }
      return true
    }
    return false
  }

  private updatePenPreview(): void {
    const engine = this.engineGetter()
    if (!engine) return
    if (!this.penActive || this.penPoints.length === 0) {
      engine.penPreview = null
    } else {
      engine.penPreview = {
        points: this.penPoints.map((p) => ({ ...p })),
        cursorPos: this.penCursorPos,
        isDraggingHandle: this.penDraggingHandle,
      }
    }
    engine.markDirty()
  }

  private finalizePen(closed: boolean): void {
    const engine = this.engineGetter()
    if (this.penPoints.length < 2) {
      this.penActive = false
      this.penPoints = []
      this.penDraggingHandle = false
      this.penCursorPos = null
      if (engine) { engine.penPreview = null; engine.markDirty() }
      useCanvasStore.getState().setActiveTool('select')
      return
    }

    // Build absolute path
    const buildD = (pts: PenAnchor[], close: boolean) => {
      const parts: string[] = [`M ${pts[0].x} ${pts[0].y}`]
      for (let i = 1; i < pts.length; i++) {
        const prev = pts[i - 1], curr = pts[i]
        if (!prev.handleOut && !curr.handleIn) {
          parts.push(`L ${curr.x} ${curr.y}`)
        } else {
          const cx1 = prev.x + (prev.handleOut?.x ?? 0)
          const cy1 = prev.y + (prev.handleOut?.y ?? 0)
          const cx2 = curr.x + (curr.handleIn?.x ?? 0)
          const cy2 = curr.y + (curr.handleIn?.y ?? 0)
          parts.push(`C ${cx1} ${cy1} ${cx2} ${cy2} ${curr.x} ${curr.y}`)
        }
      }
      if (close && pts.length > 1) {
        const last = pts[pts.length - 1], first = pts[0]
        if (!last.handleOut && !first.handleIn) {
          parts.push(`L ${first.x} ${first.y}`)
        } else {
          const cx1 = last.x + (last.handleOut?.x ?? 0)
          const cy1 = last.y + (last.handleOut?.y ?? 0)
          const cx2 = first.x + (first.handleIn?.x ?? 0)
          const cy2 = first.y + (first.handleIn?.y ?? 0)
          parts.push(`C ${cx1} ${cy1} ${cx2} ${cy2} ${first.x} ${first.y}`)
        }
        parts.push('Z')
      }
      return parts.join(' ')
    }

    // Compute bbox via a temporary SVG element
    const absD = buildD(this.penPoints, closed)
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
    const pathEl = document.createElementNS('http://www.w3.org/2000/svg', 'path')
    pathEl.setAttribute('d', absD)
    svg.appendChild(pathEl)
    document.body.appendChild(svg)
    const bbox = pathEl.getBBox()
    document.body.removeChild(svg)

    if (bbox.width < 1 && bbox.height < 1) {
      this.penActive = false
      this.penPoints = []
      this.penDraggingHandle = false
      this.penCursorPos = null
      if (engine) { engine.penPreview = null; engine.markDirty() }
      useCanvasStore.getState().setActiveTool('select')
      return
    }

    // Normalize to (0,0) origin
    const normalized = this.penPoints.map((pt) => ({
      ...pt,
      x: pt.x - bbox.x,
      y: pt.y - bbox.y,
    }))
    const d = buildD(normalized, closed)

    useDocumentStore.getState().addNode(null, {
      id: generateId(),
      type: 'path',
      name: 'Path',
      x: bbox.x,
      y: bbox.y,
      d,
      width: Math.round(bbox.width),
      height: Math.round(bbox.height),
      fill: [{ type: 'solid', color: 'transparent' }],
      stroke: {
        thickness: DEFAULT_STROKE_WIDTH,
        fill: [{ type: 'solid', color: DEFAULT_STROKE }],
      },
    })

    this.penActive = false
    this.penPoints = []
    this.penDraggingHandle = false
    this.penCursorPos = null
    if (engine) { engine.penPreview = null; engine.markDirty() }
    useCanvasStore.getState().setActiveTool('select')
  }
}
