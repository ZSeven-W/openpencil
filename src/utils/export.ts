import type { Canvas } from 'fabric'

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
