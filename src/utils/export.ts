import type { Canvas } from 'fabric'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'

function downloadFile(url: string, filename: string) {
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}

export interface PNGExportOptions {
  multiplier?: number
  filename?: string
  selectedOnly?: boolean
}

export interface SVGExportOptions {
  filename?: string
  selectedOnly?: boolean
}

export function exportToPNG(canvas: Canvas, options?: PNGExportOptions) {
  const multiplier = options?.multiplier ?? 2
  const filename = options?.filename ?? 'design.png'

  if (options?.selectedOnly) {
    const active = canvas.getActiveObject()
    if (active) {
      const dataURL = active.toDataURL({
        format: 'png',
        multiplier,
      })
      downloadFile(dataURL, filename)
      return
    }
  }

  const dataURL = canvas.toDataURL({
    format: 'png',
    multiplier,
  })
  downloadFile(dataURL, filename)
}

export type RasterFormat = 'png' | 'jpeg' | 'webp'

export interface RasterExportOptions {
  format?: RasterFormat
  multiplier?: number
  filename?: string
  selectedOnly?: boolean
}

export function exportToRaster(canvas: Canvas, options?: RasterExportOptions) {
  const format = options?.format ?? 'png'
  const multiplier = options?.multiplier ?? 2
  const filename = options?.filename ?? `design.${format === 'jpeg' ? 'jpg' : format}`
  const quality = format === 'png' ? 1 : 0.92

  const exportOpts = { format, multiplier, quality } as Parameters<Canvas['toDataURL']>[0]

  if (options?.selectedOnly) {
    const active = canvas.getActiveObject()
    if (active) {
      const dataURL = active.toDataURL(exportOpts)
      downloadFile(dataURL, filename)
      return
    }
  }

  const dataURL = canvas.toDataURL(exportOpts)
  downloadFile(dataURL, filename)
}

/**
 * Export a layer (node + all descendants) as a raster image.
 * Nodes are flattened on canvas as individual Fabric objects. We render them
 * onto a fresh offscreen canvas to avoid viewport transform issues.
 */
export function exportLayerToRaster(
  canvas: Canvas,
  nodeId: string,
  descendantIds: Set<string>,
  options?: Omit<RasterExportOptions, 'selectedOnly'>,
) {
  const format = options?.format ?? 'png'
  const multiplier = options?.multiplier ?? 1
  const filename = options?.filename ?? `design.${format === 'jpeg' ? 'jpg' : format}`
  const quality = format === 'png' ? 1 : 0.92

  const allIds = new Set(descendantIds)
  allIds.add(nodeId)

  const allObjects = canvas.getObjects() as FabricObjectWithPenId[]

  // Find the root node's Fabric object to determine crop bounds
  const rootObj = allObjects.find((obj) => obj.penNodeId === nodeId)
  if (!rootObj) return

  const originX = rootObj.left ?? 0
  const originY = rootObj.top ?? 0
  const w = (rootObj.width ?? 0) * (rootObj.scaleX ?? 1)
  const h = (rootObj.height ?? 0) * (rootObj.scaleY ?? 1)

  // Collect layer objects in render order
  const layerObjects = allObjects.filter(
    (obj) => obj.penNodeId && allIds.has(obj.penNodeId),
  )

  // Render onto a fresh offscreen canvas — no viewport transform interference
  const offscreen = document.createElement('canvas')
  offscreen.width = Math.ceil(w * multiplier)
  offscreen.height = Math.ceil(h * multiplier)
  const ctx = offscreen.getContext('2d')!
  ctx.scale(multiplier, multiplier)
  ctx.translate(-originX, -originY)

  for (const obj of layerObjects) {
    obj.render(ctx)
  }

  const mimeType = format === 'jpeg' ? 'image/jpeg' : format === 'webp' ? 'image/webp' : 'image/png'
  const dataURL = offscreen.toDataURL(mimeType, quality)
  downloadFile(dataURL, filename)
}

export function exportToSVG(canvas: Canvas, options?: SVGExportOptions) {
  const filename = options?.filename ?? 'design.svg'

  if (options?.selectedOnly) {
    const active = canvas.getActiveObject()
    if (active) {
      const svg = active.toSVG()
      const width = active.width ?? 100
      const height = active.height ?? 100
      const fullSVG = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">${svg}</svg>`
      const blob = new Blob([fullSVG], { type: 'image/svg+xml' })
      downloadFile(URL.createObjectURL(blob), filename)
      return
    }
  }

  const svg = canvas.toSVG()
  const blob = new Blob([svg], { type: 'image/svg+xml' })
  downloadFile(URL.createObjectURL(blob), filename)
}
