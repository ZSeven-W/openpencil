import { useEffect, useRef, type RefObject } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'
import { getCanvasBackground, SELECTION_BLUE, MIN_ZOOM, MAX_ZOOM } from './canvas-constants'
import { setupRotationCursorHandler } from './canvas-controls'

const FIT_PADDING = 64

function nodeSize(v: unknown): number {
  if (typeof v === 'number') return v
  if (typeof v === 'string') {
    const m = v.match(/\((\d+(?:\.\d+)?)\)/)
    if (m) return parseFloat(m[1])
    const n = parseFloat(v)
    if (!isNaN(n)) return n
  }
  return 0
}

/** Compute the bounding box of all document nodes (recursive). */
function computeDocBounds(nodes: PenNode[], ox = 0, oy = 0) {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity

  for (const node of nodes) {
    const nx = (node.x ?? 0) + ox
    const ny = (node.y ?? 0) + oy
    const nw = 'width' in node ? nodeSize(node.width) : 0
    const nh = 'height' in node ? nodeSize(node.height) : 0

    minX = Math.min(minX, nx)
    minY = Math.min(minY, ny)
    maxX = Math.max(maxX, nx + (nw || 100))
    maxY = Math.max(maxY, ny + (nh || 100))

    if ('children' in node && node.children && node.children.length > 0) {
      const child = computeDocBounds(node.children, nx, ny)
      minX = Math.min(minX, child.minX)
      minY = Math.min(minY, child.minY)
      maxX = Math.max(maxX, child.maxX)
      maxY = Math.max(maxY, child.maxY)
    }
  }

  return { minX, minY, maxX, maxY }
}

/**
 * Zoom and pan so all document content fits in the visible canvas area
 * with some padding. Call after canvas init, newDocument(), or loadDocument().
 */
export function zoomToFitContent() {
  const canvas = useCanvasStore.getState().fabricCanvas
  if (!canvas) return

  const children = useDocumentStore.getState().document.children
  if (children.length === 0) return

  const { minX, minY, maxX, maxY } = computeDocBounds(children)
  if (!isFinite(minX)) return

  const contentW = maxX - minX
  const contentH = maxY - minY
  const cw = canvas.getWidth()
  const ch = canvas.getHeight()

  // Calculate zoom to fit content with padding
  const scaleX = (cw - FIT_PADDING * 2) / contentW
  const scaleY = (ch - FIT_PADDING * 2) / contentH
  let zoom = Math.min(scaleX, scaleY)
  zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom))
  // Don't zoom in beyond 1x for small content
  zoom = Math.min(zoom, 1)

  const centerX = (minX + maxX) / 2
  const centerY = (minY + maxY) / 2
  const panX = cw / 2 - centerX * zoom
  const panY = ch / 2 - centerY * zoom

  const vpt = canvas.viewportTransform
  vpt[0] = zoom
  vpt[3] = zoom
  vpt[4] = panX
  vpt[5] = panY
  canvas.setViewportTransform(vpt)
  useCanvasStore.getState().setZoom(zoom)
  useCanvasStore.getState().setPan(panX, panY)
  canvas.requestRenderAll()
}

export function useFabricCanvas(
  canvasRef: RefObject<HTMLCanvasElement | null>,
  containerRef: RefObject<HTMLDivElement | null>,
) {
  const initialized = useRef(false)

  useEffect(() => {
    const el = canvasRef.current
    const container = containerRef.current
    if (!el || !container || initialized.current) return

    initialized.current = true

    const canvas = new fabric.Canvas(el, {
      width: container.clientWidth,
      height: container.clientHeight,
      backgroundColor: getCanvasBackground(),
      selection: true,
      preserveObjectStacking: true,
      stopContextMenu: true,
      fireRightClick: true,
    })

    // Selection marquee styling
    canvas.selectionColor = 'rgba(13, 153, 255, 0.06)'
    canvas.selectionBorderColor = SELECTION_BLUE
    canvas.selectionLineWidth = 1

    useCanvasStore.getState().setFabricCanvas(canvas)
    setupRotationCursorHandler(canvas)
    canvas.requestRenderAll()

    // Center viewport on the default frame after a tick (sync needs to run first)
    requestAnimationFrame(() => zoomToFitContent())

    // Resize observer
    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect
        canvas.setDimensions({ width, height })
        canvas.requestRenderAll()
      }
    })
    resizeObserver.observe(container)

    // Watch theme changes on <html> class to update canvas background
    const themeObserver = new MutationObserver(() => {
      canvas.backgroundColor = getCanvasBackground()
      canvas.requestRenderAll()
    })
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })

    return () => {
      resizeObserver.disconnect()
      themeObserver.disconnect()
      useCanvasStore.getState().setFabricCanvas(null)
      canvas.dispose()
      initialized.current = false
    }
  }, [canvasRef, containerRef])
}
