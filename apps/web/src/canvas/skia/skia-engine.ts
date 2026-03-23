import type { CanvasKit, Surface } from 'canvaskit-wasm'
import type { EllipseNode } from '@/types/pen'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, getActivePageChildren, getAllChildren } from '@/stores/document-store'
import { resolveNodeForCanvas, getDefaultTheme } from '@/variables/resolve-variables'
import { getCanvasBackground, MIN_ZOOM, MAX_ZOOM } from '../canvas-constants'
import { setRootChildrenProvider } from '../canvas-layout-engine'
import { SkiaRenderer, type RenderNode } from './skia-renderer'
import {
  SpatialIndex,
  parseColor,
  viewportMatrix,
  zoomToPoint as vpZoomToPoint,
  flattenToRenderNodes,
  resolveRefs,
  premeasureTextHeights,
  collectReusableIds,
  collectInstanceIds,
  getViewportBounds,
  isRectInViewport,
} from '@zseven-w/pen-renderer'
import {
  getActiveAgentIndicators,
  getActiveAgentFrames,
  isPreviewNode,
} from '../agent-indicator'
import { isNodeBorderReady, getNodeRevealTime } from '@/services/ai/design-animation'
import { lookupIconByName } from '@/services/ai/icon-resolver'

// Re-export for use by canvas component
export { screenToScene } from '@zseven-w/pen-renderer'
export { SpatialIndex } from '@zseven-w/pen-renderer'

// ---------------------------------------------------------------------------
// SkiaEngine — ties rendering, viewport, hit testing together
// ---------------------------------------------------------------------------

export class SkiaEngine {
  ck: CanvasKit
  surface: Surface | null = null
  renderer: SkiaRenderer
  spatialIndex = new SpatialIndex()
  renderNodes: RenderNode[] = []

  // Component/instance IDs for colored frame labels
  private reusableIds = new Set<string>()
  private instanceIds = new Set<string>()

  // Agent animation: track start time so glow only pulses ~2 times
  private agentAnimStart = 0

  private canvasEl: HTMLCanvasElement | null = null
  private animFrameId = 0
  private dirty = true

  // Viewport
  zoom = 1
  panX = 0
  panY = 0

  // Drag suppression — prevents syncFromDocument during drag
  // so the layout engine doesn't override visual positions
  dragSyncSuppressed = false

  // Interaction state
  hoveredNodeId: string | null = null
  marquee: { x1: number; y1: number; x2: number; y2: number } | null = null
  previewShape: {
    type: 'rectangle' | 'ellipse' | 'frame' | 'line' | 'polygon'
    x: number; y: number; w: number; h: number
  } | null = null
  penPreview: import('./skia-overlays').PenPreviewData | null = null

  constructor(ck: CanvasKit) {
    this.ck = ck
    this.renderer = new SkiaRenderer(ck)
    // Wire up icon lookup for icon_font nodes
    this.renderer.setIconLookup(lookupIconByName)
    // Wire up root children provider for layout engine fill-width fallback
    setRootChildrenProvider(() => useDocumentStore.getState().document.children)
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  init(canvasEl: HTMLCanvasElement) {
    this.canvasEl = canvasEl
    const dpr = window.devicePixelRatio || 1
    canvasEl.width = canvasEl.clientWidth * dpr
    canvasEl.height = canvasEl.clientHeight * dpr

    this.surface = this.ck.MakeWebGLCanvasSurface(canvasEl)
    if (!this.surface) {
      // Fallback to software
      this.surface = this.ck.MakeSWCanvasSurface(canvasEl)
    }
    if (!this.surface) {
      console.error('SkiaEngine: Failed to create surface')
      return
    }

    this.renderer.init()
    this.renderer.setRedrawCallback(() => this.markDirty())
    // Re-render when async font loading completes
    ;(this.renderer as any)._onFontLoaded = () => this.markDirty()
    // Pre-load default fonts for vector text rendering.
    // Noto Sans SC is loaded alongside Inter so CJK glyphs are always available
    // in the fallback chain — system CJK fonts (PingFang SC, Microsoft YaHei, etc.)
    // are skipped from Google Fonts, and without Noto Sans SC the fallback chain
    // would only contain Inter which has no CJK coverage, causing tofu.
    this.renderer.fontManager.ensureFont('Inter').then(() => this.markDirty())
    this.renderer.fontManager.ensureFont('Noto Sans SC').then(() => this.markDirty())
    this.startRenderLoop()
  }

  dispose() {
    if (this.animFrameId) cancelAnimationFrame(this.animFrameId)
    this.renderer.dispose()
    this.surface?.delete()
    this.surface = null
  }

  resize(width: number, height: number) {
    if (!this.canvasEl) return
    const dpr = window.devicePixelRatio || 1
    this.canvasEl.width = width * dpr
    this.canvasEl.height = height * dpr

    // Recreate surface
    this.surface?.delete()
    this.surface = this.ck.MakeWebGLCanvasSurface(this.canvasEl)
    if (!this.surface) {
      this.surface = this.ck.MakeSWCanvasSurface(this.canvasEl)
    }
    this.render()
  }

  // ---------------------------------------------------------------------------
  // Document sync
  // ---------------------------------------------------------------------------

  syncFromDocument() {
    if (this.dragSyncSuppressed) return
    const docState = useDocumentStore.getState()
    const activePageId = useCanvasStore.getState().activePageId
    const pageChildren = getActivePageChildren(docState.document, activePageId)
    const allNodes = getAllChildren(docState.document)

    // Collect reusable/instance IDs from raw tree (before ref resolution strips them)
    this.reusableIds.clear()
    this.instanceIds.clear()
    collectReusableIds(pageChildren, this.reusableIds)
    collectInstanceIds(pageChildren, this.instanceIds)

    // Resolve refs, variables, then flatten
    const resolved = resolveRefs(pageChildren, allNodes)

    // Resolve design variables
    const variables = docState.document.variables ?? {}
    const themes = docState.document.themes
    const defaultTheme = getDefaultTheme(themes)
    const variableResolved = resolved.map((n) =>
      resolveNodeForCanvas(n, variables, defaultTheme),
    )

    // Only premeasure text HEIGHTS for fixed-width text (where wrapping
    // estimation may differ from Canvas 2D). Never touch widths or
    // container-relative sizing to maintain layout consistency with Fabric.js.
    const measured = premeasureTextHeights(variableResolved)

    this.renderNodes = flattenToRenderNodes(measured)

    this.spatialIndex.rebuild(this.renderNodes)
    this.markDirty()
  }

  // ---------------------------------------------------------------------------
  // Render loop
  // ---------------------------------------------------------------------------

  markDirty() {
    this.dirty = true
  }

  private startRenderLoop() {
    const loop = () => {
      this.animFrameId = requestAnimationFrame(loop)
      if (!this.dirty || !this.surface) return
      this.dirty = false
      this.render()
    }
    this.animFrameId = requestAnimationFrame(loop)
  }

  private render() {
    if (!this.surface || !this.canvasEl) return
    const canvas = this.surface.getCanvas()
    const ck = this.ck

    const dpr = window.devicePixelRatio || 1
    const selectedIds = new Set(useCanvasStore.getState().selection.selectedIds)

    // Clear
    const bgColor = getCanvasBackground()
    canvas.clear(parseColor(ck, bgColor))

    // Apply viewport transform
    canvas.save()
    canvas.scale(dpr, dpr)
    canvas.concat(viewportMatrix({ zoom: this.zoom, panX: this.panX, panY: this.panY }))

    // Pass current zoom to renderer for zoom-aware text rasterization
    this.renderer.zoom = this.zoom

    const vpBounds = getViewportBounds(
      { zoom: this.zoom, panX: this.panX, panY: this.panY },
      this.canvasEl.clientWidth,
      this.canvasEl.clientHeight,
      64 / this.zoom
    )
    // Draw all render nodes
    for (const rn of this.renderNodes) {
      // Skip nodes outside the viewport
      if (!isRectInViewport({ x: rn.absX, y: rn.absY, w: rn.absW, h: rn.absH }, vpBounds)) continue
      this.renderer.drawNodeWithSelection(canvas, rn, selectedIds)
    }

    // Draw agent indicators (glow, badges, node borders, preview fills)
    const agentIndicators = getActiveAgentIndicators()
    const agentFrames = getActiveAgentFrames()
    const hasAgentOverlays = agentIndicators.size > 0 || agentFrames.size > 0

    if (!hasAgentOverlays) {
      this.agentAnimStart = 0
    }

    if (hasAgentOverlays) {
      const now = Date.now()
      if (this.agentAnimStart === 0) this.agentAnimStart = now
      const elapsed = now - this.agentAnimStart
      // Frame glow: smooth fade-in → fade-out (single bell, ~1.2s)
      const GLOW_DURATION = 1200
      const glowT = Math.min(1, elapsed / GLOW_DURATION)
      const breath = Math.sin(glowT * Math.PI) // 0 → 1 → 0

      // Agent node borders and preview fills (per-element fade-in → fade-out)
      const NODE_FADE_DURATION = 1000
      for (const rn of this.renderNodes) {
        const indicator = agentIndicators.get(rn.node.id)
        if (!indicator) continue
        if (!isNodeBorderReady(rn.node.id)) continue

        const revealAt = getNodeRevealTime(rn.node.id)
        if (revealAt === undefined) continue
        const nodeElapsed = now - revealAt
        if (nodeElapsed > NODE_FADE_DURATION) continue

        // Smooth bell curve: fade in then fade out
        const nodeT = Math.min(1, nodeElapsed / NODE_FADE_DURATION)
        const nodeBreath = Math.sin(nodeT * Math.PI)

        if (isPreviewNode(rn.node.id)) {
          this.renderer.drawAgentPreviewFill(
            canvas, rn.absX, rn.absY, rn.absW, rn.absH,
            indicator.color, now,
          )
        }

        this.renderer.drawAgentNodeBorder(
          canvas, rn.absX, rn.absY, rn.absW, rn.absH,
          indicator.color, nodeBreath, this.zoom,
        )
      }

      // Agent frame glow and badges
      for (const rn of this.renderNodes) {
        const frame = agentFrames.get(rn.node.id)
        if (!frame) continue

        this.renderer.drawAgentGlow(
          canvas, rn.absX, rn.absY, rn.absW, rn.absH,
          frame.color, breath, this.zoom,
        )
        this.renderer.drawAgentBadge(
          canvas, frame.name,
          rn.absX, rn.absY, rn.absW,
          frame.color, this.zoom, now,
        )
      }
    }

    // Hover outline
    if (this.hoveredNodeId && !selectedIds.has(this.hoveredNodeId)) {
      const hovered = this.spatialIndex.get(this.hoveredNodeId)
      if (hovered) {
        this.renderer.drawHoverOutline(canvas, hovered.absX, hovered.absY, hovered.absW, hovered.absH)
      }
    }

    // Arc handles for selected ellipse
    if (selectedIds.size === 1) {
      const selId = selectedIds.values().next().value as string
      const selRN = this.spatialIndex.get(selId)
      if (selRN && selRN.node.type === 'ellipse') {
        const eNode = selRN.node as EllipseNode
        this.renderer.drawArcHandles(
          canvas,
          selRN.absX, selRN.absY, selRN.absW, selRN.absH,
          eNode.startAngle ?? 0, eNode.sweepAngle ?? 360, eNode.innerRadius ?? 0,
          this.zoom,
        )
      }
    }

    // Drawing preview shape
    if (this.previewShape) {
      this.renderer.drawPreview(canvas, this.previewShape)
    }

    // Pen tool preview
    if (this.penPreview) {
      this.renderer.drawPenPreview(canvas, this.penPreview, this.zoom)
    }

    // Selection marquee
    if (this.marquee) {
      this.renderer.drawSelectionMarquee(
        canvas,
        this.marquee.x1, this.marquee.y1,
        this.marquee.x2, this.marquee.y2,
      )
    }

    canvas.restore()

    // Draw frame labels outside viewport transform so fontSize stays constant
    // (avoids Math.ceil(12/zoom) integer-boundary jumps causing label size flicker)
    canvas.save()
    canvas.scale(dpr, dpr)
    for (const rn of this.renderNodes) {
      if (!rn.node.name) continue
      const isRootFrame = rn.node.type === 'frame' && !rn.clipRect
      const isReusable = this.reusableIds.has(rn.node.id)
      const isInstance = this.instanceIds.has(rn.node.id)
      if (!isRootFrame && !isReusable && !isInstance) continue
      const sx = rn.absX * this.zoom + this.panX
      const sy = rn.absY * this.zoom + this.panY
      this.renderer.drawFrameLabelColored(canvas, rn.node.name, sx, sy, isReusable, isInstance, 1)
    }
    canvas.restore()

    this.surface.flush()

    // Keep animating while agent overlays are active (spinning dot + node flashes)
    if (hasAgentOverlays) {
      this.markDirty()
    }
  }

  // ---------------------------------------------------------------------------
  // Viewport control
  // ---------------------------------------------------------------------------

  setViewport(zoom: number, panX: number, panY: number) {
    this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom))
    this.panX = panX
    this.panY = panY
    useCanvasStore.getState().setZoom(this.zoom)
    useCanvasStore.getState().setPan(this.panX, this.panY)
    this.markDirty()
  }

  zoomToPoint(screenX: number, screenY: number, newZoom: number) {
    if (!this.canvasEl) return
    const rect = this.canvasEl.getBoundingClientRect()
    const vp = vpZoomToPoint(
      { zoom: this.zoom, panX: this.panX, panY: this.panY },
      screenX, screenY, rect, newZoom,
    )
    this.setViewport(vp.zoom, vp.panX, vp.panY)
  }

  pan(dx: number, dy: number) {
    this.setViewport(this.zoom, this.panX + dx, this.panY + dy)
  }

  getCanvasRect(): DOMRect | null {
    return this.canvasEl?.getBoundingClientRect() ?? null
  }

  getCanvasSize(): { width: number; height: number } {
    return {
      width: this.canvasEl?.clientWidth ?? 800,
      height: this.canvasEl?.clientHeight ?? 600,
    }
  }

  zoomToFitContent() {
    if (!this.canvasEl || this.renderNodes.length === 0) return
    const FIT_PADDING = 64
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
    for (const rn of this.renderNodes) {
      if (rn.clipRect) continue // skip children, only root bounds
      minX = Math.min(minX, rn.absX)
      minY = Math.min(minY, rn.absY)
      maxX = Math.max(maxX, rn.absX + rn.absW)
      maxY = Math.max(maxY, rn.absY + rn.absH)
    }
    if (!isFinite(minX)) return
    const contentW = maxX - minX
    const contentH = maxY - minY
    const cw = this.canvasEl.clientWidth
    const ch = this.canvasEl.clientHeight
    const scaleX = (cw - FIT_PADDING * 2) / contentW
    const scaleY = (ch - FIT_PADDING * 2) / contentH
    let zoom = Math.min(scaleX, scaleY, 1)
    zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom))
    const centerX = (minX + maxX) / 2
    const centerY = (minY + maxY) / 2
    this.setViewport(
      zoom,
      cw / 2 - centerX * zoom,
      ch / 2 - centerY * zoom,
    )
  }
}

