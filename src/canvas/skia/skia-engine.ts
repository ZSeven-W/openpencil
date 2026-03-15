import type { CanvasKit, Surface } from 'canvaskit-wasm'
import type { PenNode, ContainerProps } from '@/types/pen'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, getActivePageChildren, getAllChildren } from '@/stores/document-store'
import { resolveNodeForCanvas, getDefaultTheme } from '@/variables/resolve-variables'
import { getCanvasBackground, MIN_ZOOM, MAX_ZOOM } from '../canvas-constants'
import {
  resolvePadding,
  isNodeVisible,
  getNodeWidth,
  getNodeHeight,
  computeLayoutPositions,
  inferLayout,
} from '../canvas-layout-engine'
import { parseSizing, defaultLineHeight } from '../canvas-text-measure'
import { SkiaRenderer, type RenderNode } from './skia-renderer'
import { SpatialIndex } from './skia-hit-test'
import { parseColor, wrapLine, cssFontFamily } from './skia-paint-utils'
import {
  viewportMatrix,
  zoomToPoint as vpZoomToPoint,
} from './skia-viewport'
import {
  getActiveAgentIndicators,
  getActiveAgentFrames,
  isPreviewNode,
} from '../agent-indicator'
import { isNodeBorderReady, getNodeRevealTime } from '@/services/ai/design-animation'

// Re-export for use by canvas component
export { screenToScene } from './skia-viewport'
export { SpatialIndex } from './skia-hit-test'

// ---------------------------------------------------------------------------
// Pre-measure text widths using Canvas 2D (browser fonts)
// ---------------------------------------------------------------------------

let _measureCtx: CanvasRenderingContext2D | null = null
function getMeasureCtx(): CanvasRenderingContext2D {
  if (!_measureCtx) {
    const c = document.createElement('canvas')
    _measureCtx = c.getContext('2d')!
  }
  return _measureCtx
}

/**
 * Walk the node tree and fix text HEIGHTS using actual Canvas 2D wrapping.
 *
 * Only targets fixed-width text with auto height — these are the cases where
 * estimateTextHeight may underestimate because its width estimation differs
 * from Canvas 2D's actual text measurement, leading to incorrect wrap counts.
 *
 * IMPORTANT: This function never touches WIDTH or container-relative sizing
 * strings (fill_container / fit_content). Changing widths breaks layout
 * resolution in computeLayoutPositions.
 */
function premeasureTextHeights(nodes: PenNode[]): PenNode[] {
  return nodes.map((node) => {
    let result = node

    if (node.type === 'text') {
      const tNode = node as import('@/types/pen').TextNode
      const hasFixedWidth = typeof tNode.width === 'number' && tNode.width > 0
      const isContainerHeight = typeof tNode.height === 'string'
        && (tNode.height === 'fill_container' || tNode.height === 'fit_content')
      const textGrowth = tNode.textGrowth
      const content = typeof tNode.content === 'string'
        ? tNode.content
        : Array.isArray(tNode.content)
          ? tNode.content.map((s) => s.text ?? '').join('')
          : ''

      // Match Fabric.js wrapping: only premeasure when text actually wraps.
      // textGrowth='auto' means auto-width (no wrapping) regardless of textAlign.
      // textGrowth=undefined with non-left textAlign uses fixed-width for alignment.
      const textAlign = tNode.textAlign
      const isFixedWidthText = textGrowth === 'fixed-width' || textGrowth === 'fixed-width-height'
        || (textGrowth !== 'auto' && textAlign != null && textAlign !== 'left')
      if (content && hasFixedWidth && isFixedWidthText && !isContainerHeight) {
        const fontSize = tNode.fontSize ?? 16
        const fontWeight = tNode.fontWeight ?? '400'
        const fontFamily = tNode.fontFamily ?? 'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif'
        const ctx = getMeasureCtx()
        ctx.font = `${fontWeight} ${fontSize}px ${cssFontFamily(fontFamily)}`

        // Fixed-width text with auto height: wrap and measure actual height
        const wrapWidth = (tNode.width as number) + fontSize * 0.2
        const rawLines = content.split('\n')
        const wrappedLines: string[] = []
        for (const raw of rawLines) {
          if (!raw) { wrappedLines.push(''); continue }
          wrapLine(ctx, raw, wrapWidth, wrappedLines)
        }
        const lineHeightMul = tNode.lineHeight ?? defaultLineHeight(fontSize)
        const lineHeight = lineHeightMul * fontSize
        const glyphH = fontSize * 1.13
        const measuredHeight = Math.ceil(
          wrappedLines.length <= 1
            ? glyphH + 2
            : (wrappedLines.length - 1) * lineHeight + glyphH + 2,
        )
        const currentHeight = typeof tNode.height === 'number' ? tNode.height : 0
        const explicitLineCount = rawLines.length
        const needsHeight = currentHeight <= 0 || wrappedLines.length > explicitLineCount
        if (needsHeight && measuredHeight > currentHeight) {
          result = { ...node, height: measuredHeight } as unknown as PenNode
        }
      }
    }

    // Recurse into children
    if ('children' in result && result.children) {
      const children = result.children
      const measured = premeasureTextHeights(children)
      if (measured !== children) {
        result = { ...result, children: measured } as unknown as PenNode
      }
    }

    return result
  })
}

// ---------------------------------------------------------------------------
// Flatten document tree → absolute-positioned RenderNode list
// ---------------------------------------------------------------------------

interface ClipInfo {
  x: number; y: number; w: number; h: number; rx: number
}

function sizeToNumber(val: number | string | undefined, fallback: number): number {
  if (typeof val === 'number') return val
  if (typeof val === 'string') {
    const m = val.match(/\((\d+(?:\.\d+)?)\)/)
    if (m) return parseFloat(m[1])
    const n = parseFloat(val)
    if (!isNaN(n)) return n
  }
  return fallback
}

function cornerRadiusVal(cr: number | [number, number, number, number] | undefined): number {
  if (cr === undefined) return 0
  if (typeof cr === 'number') return cr
  return cr[0]
}

/** Resolve RefNodes inline (same logic as use-canvas-sync.ts). */
function resolveRefs(
  nodes: PenNode[],
  rootNodes: PenNode[],
  findInTree: (nodes: PenNode[], id: string) => PenNode | null,
  visited = new Set<string>(),
): PenNode[] {
  return nodes.flatMap((node) => {
    if (node.type !== 'ref') {
      if ('children' in node && node.children) {
        return [{ ...node, children: resolveRefs(node.children, rootNodes, findInTree, visited) } as PenNode]
      }
      return [node]
    }
    if (visited.has(node.ref)) return []
    const component = findInTree(rootNodes, node.ref)
    if (!component) return []
    visited.add(node.ref)
    const resolved: Record<string, unknown> = { ...component }
    for (const [key, val] of Object.entries(node)) {
      if (key === 'type' || key === 'ref' || key === 'descendants' || key === 'children') continue
      if (val !== undefined) resolved[key] = val
    }
    resolved.type = component.type
    if (!resolved.name) resolved.name = component.name
    delete resolved.reusable
    const resolvedNode = resolved as unknown as PenNode
    if ('children' in component && component.children) {
      const refNode = node as import('@/types/pen').RefNode
      ;(resolvedNode as PenNode & ContainerProps).children = remapIds(component.children, node.id, refNode.descendants)
    }
    visited.delete(node.ref)
    return [resolvedNode]
  })
}

function remapIds(children: PenNode[], refId: string, overrides?: Record<string, Partial<PenNode>>): PenNode[] {
  return children.map((child) => {
    const virtualId = `${refId}__${child.id}`
    const ov = overrides?.[child.id] ?? {}
    const mapped = { ...child, ...ov, id: virtualId } as PenNode
    if ('children' in mapped && mapped.children) {
      (mapped as PenNode & ContainerProps).children = remapIds(mapped.children, refId, overrides)
    }
    return mapped
  })
}

export function flattenToRenderNodes(
  nodes: PenNode[],
  offsetX = 0,
  offsetY = 0,
  parentAvailW?: number,
  parentAvailH?: number,
  clipCtx?: ClipInfo,
  depth = 0,
): RenderNode[] {
  const result: RenderNode[] = []

  // Reverse order: children[0] = top layer = rendered last (frontmost)
  for (let i = nodes.length - 1; i >= 0; i--) {
    const node = nodes[i]
    if (!isNodeVisible(node)) continue

    // Resolve fill_container / fit_content
    let resolved = node
    if (parentAvailW !== undefined || parentAvailH !== undefined) {
      let changed = false
      const r: Record<string, unknown> = { ...node }
      if ('width' in node && typeof node.width !== 'number') {
        const s = parseSizing(node.width)
        if (s === 'fill' && parentAvailW) { r.width = parentAvailW; changed = true }
        else if (s === 'fit') { r.width = getNodeWidth(node, parentAvailW); changed = true }
      }
      if ('height' in node && typeof node.height !== 'number') {
        const s = parseSizing(node.height)
        if (s === 'fill' && parentAvailH) { r.height = parentAvailH; changed = true }
        else if (s === 'fit') { r.height = getNodeHeight(node, parentAvailH, parentAvailW); changed = true }
      }
      if (changed) resolved = r as unknown as PenNode
    }

    // Compute height for frames without explicit numeric height
    if (
      node.type === 'frame'
      && 'children' in node && node.children?.length
      && (!('height' in resolved) || typeof resolved.height !== 'number')
    ) {
      const computedH = getNodeHeight(resolved, parentAvailH, parentAvailW)
      if (computedH > 0) resolved = { ...resolved, height: computedH } as unknown as PenNode
    }

    const absX = (resolved.x ?? 0) + offsetX
    const absY = (resolved.y ?? 0) + offsetY
    const absW = 'width' in resolved ? sizeToNumber(resolved.width, 100) : 100
    const absH = 'height' in resolved ? sizeToNumber(resolved.height, 100) : 100

    result.push({
      node: { ...resolved, x: absX, y: absY } as PenNode,
      absX, absY, absW, absH,
      clipRect: clipCtx,
    })

    // Recurse into children
    const children = 'children' in node ? node.children : undefined
    if (children && children.length > 0) {
      const nodeW = getNodeWidth(resolved, parentAvailW)
      const nodeH = getNodeHeight(resolved, parentAvailH, parentAvailW)
      const pad = resolvePadding('padding' in resolved ? (resolved as PenNode & ContainerProps).padding : undefined)
      const childAvailW = Math.max(0, nodeW - pad.left - pad.right)
      const childAvailH = Math.max(0, nodeH - pad.top - pad.bottom)

      const layout = ('layout' in node ? (node as ContainerProps).layout : undefined) || inferLayout(node)
      const positioned = layout && layout !== 'none'
        ? computeLayoutPositions(resolved, children)
        : children

      // Clipping — only clip for root frames (artboard behavior).
      // Nested frames do NOT clip children, matching Fabric.js behavior.
      // Fabric.js doesn't implement frame-level clipping, so children always overflow.
      // TODO: add proper clipContent support once Fabric.js is fully replaced.
      let childClip = clipCtx
      const isRootFrame = node.type === 'frame' && depth === 0
      if (isRootFrame) {
        const crRaw = 'cornerRadius' in node ? cornerRadiusVal(node.cornerRadius) : 0
        const cr = Math.min(crRaw, nodeH / 2)
        childClip = { x: absX, y: absY, w: nodeW, h: nodeH, rx: cr }
      }

      const childRNs = flattenToRenderNodes(positioned, absX, absY, childAvailW, childAvailH, childClip, depth + 1)

      // Propagate parent flip to children: mirror positions within parent bounds
      // and toggle child flipX/flipY. Must run BEFORE rotation propagation.
      const parentFlipX = node.flipX === true
      const parentFlipY = node.flipY === true
      if (parentFlipX || parentFlipY) {
        const pcx = absX + nodeW / 2
        const pcy = absY + nodeH / 2
        for (const crn of childRNs) {
          const updates: Record<string, unknown> = {}
          if (parentFlipX) {
            const ccx = crn.absX + crn.absW / 2
            crn.absX = 2 * pcx - ccx - crn.absW / 2
            const childFlip = crn.node.flipX === true
            updates.flipX = !childFlip || undefined
          }
          if (parentFlipY) {
            const ccy = crn.absY + crn.absH / 2
            crn.absY = 2 * pcy - ccy - crn.absH / 2
            const childFlip = crn.node.flipY === true
            updates.flipY = !childFlip || undefined
          }
          crn.node = { ...crn.node, x: crn.absX, y: crn.absY, ...updates } as PenNode
        }
      }

      // Propagate parent rotation to children: rotate their positions around
      // the parent's center and accumulate the rotation angle.
      // Children are in the parent's LOCAL (unrotated) coordinate space, so we
      // need to apply the parent's rotation to get correct absolute positions.
      const parentRot = node.rotation ?? 0
      if (parentRot !== 0) {
        const cx = absX + nodeW / 2
        const cy = absY + nodeH / 2
        const rad = parentRot * Math.PI / 180
        const cosA = Math.cos(rad)
        const sinA = Math.sin(rad)

        for (const crn of childRNs) {
          // Rotate child CENTER around parent center
          const ccx = crn.absX + crn.absW / 2
          const ccy = crn.absY + crn.absH / 2
          const dx = ccx - cx
          const dy = ccy - cy
          const newCx = cx + dx * cosA - dy * sinA
          const newCy = cy + dx * sinA + dy * cosA
          crn.absX = newCx - crn.absW / 2
          crn.absY = newCy - crn.absH / 2
          // Accumulate rotation and update node position
          const childRot = crn.node.rotation ?? 0
          crn.node = { ...crn.node, x: crn.absX, y: crn.absY, rotation: childRot + parentRot } as PenNode
        }
      }

      result.push(...childRNs)
    }
  }

  return result
}

// ---------------------------------------------------------------------------
// Component / instance ID collection (from raw tree, before ref resolution)
// ---------------------------------------------------------------------------

function collectReusableIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if (node.type === 'frame' && node.reusable === true) {
      result.add(node.id)
    }
    if ('children' in node && node.children) {
      collectReusableIds(node.children, result)
    }
  }
}

function collectInstanceIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if (node.type === 'ref') {
      result.add(node.id)
    }
    if ('children' in node && node.children) {
      collectInstanceIds(node.children, result)
    }
  }
}

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
    type: 'rectangle' | 'ellipse' | 'frame' | 'line'
    x: number; y: number; w: number; h: number
  } | null = null
  penPreview: import('./skia-overlays').PenPreviewData | null = null

  constructor(ck: CanvasKit) {
    this.ck = ck
    this.renderer = new SkiaRenderer(ck)
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
    this.markDirty()
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

    // Simple findNodeInTree
    const findInTree = (nodes: PenNode[], id: string): PenNode | null => {
      for (const n of nodes) {
        if (n.id === id) return n
        if ('children' in n && n.children) {
          const found = findInTree(n.children, id)
          if (found) return found
        }
      }
      return null
    }

    // Collect reusable/instance IDs from raw tree (before ref resolution strips them)
    this.reusableIds.clear()
    this.instanceIds.clear()
    collectReusableIds(pageChildren, this.reusableIds)
    collectInstanceIds(pageChildren, this.instanceIds)

    // Resolve refs, variables, then flatten
    const resolved = resolveRefs(pageChildren, allNodes, findInTree)

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

    // Draw all render nodes
    for (const rn of this.renderNodes) {
      this.renderer.drawNode(canvas, rn, selectedIds)
    }

    // Draw frame labels (root frames + reusable components + instances at any depth)
    for (const rn of this.renderNodes) {
      if (!rn.node.name) continue
      const isRootFrame = rn.node.type === 'frame' && !rn.clipRect
      const isReusable = this.reusableIds.has(rn.node.id)
      const isInstance = this.instanceIds.has(rn.node.id)
      if (!isRootFrame && !isReusable && !isInstance) continue
      this.renderer.drawFrameLabelColored(
        canvas, rn.node.name, rn.absX, rn.absY,
        isReusable, isInstance, this.zoom,
      )
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

