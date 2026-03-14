import type { CanvasKit, Image as SkImage } from 'canvaskit-wasm'

/**
 * Async image loader for CanvasKit. Loads images via browser's native Image
 * element (supports all browser-supported formats), rasterizes to Canvas 2D,
 * then converts to CanvasKit Image for GPU rendering.
 */
export class SkiaImageLoader {
  private ck: CanvasKit
  private cache = new Map<string, SkImage | null>()
  private loading = new Set<string>()
  private onLoaded: (() => void) | null = null

  constructor(ck: CanvasKit) {
    this.ck = ck
  }

  /** Set callback to trigger re-render when an image finishes loading. */
  setOnLoaded(cb: () => void) {
    this.onLoaded = cb
  }

  /** Get a cached image, or null if not loaded / failed. Returns undefined if not yet requested. */
  get(src: string): SkImage | null | undefined {
    return this.cache.get(src)
  }

  /** Start loading an image if not already cached or in progress. */
  request(src: string) {
    if (this.cache.has(src) || this.loading.has(src)) return
    this.loading.add(src)
    this.loadAsync(src)
  }

  dispose() {
    for (const img of this.cache.values()) {
      img?.delete()
    }
    this.cache.clear()
    this.loading.clear()
  }

  private async loadAsync(src: string) {
    try {
      // Use browser Image element — supports all browser-supported formats
      const htmlImg = await this.loadHtmlImage(src)
      const skImg = this.htmlImageToSkia(htmlImg)
      this.cache.set(src, skImg)
      this.loading.delete(src)
      this.onLoaded?.()
    } catch (e) {
      console.warn('Failed to load image:', src.slice(0, 80), e)
      this.cache.set(src, null)
      this.loading.delete(src)
    }
  }

  private loadHtmlImage(src: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
      const img = new Image()
      img.crossOrigin = 'anonymous'
      img.onload = () => resolve(img)
      img.onerror = (e) => reject(new Error(`Image load failed: ${e}`))
      img.src = src
    })
  }

  /** Rasterize an HTML Image to Canvas 2D, then convert to CanvasKit Image. */
  private htmlImageToSkia(htmlImg: HTMLImageElement): SkImage | null {
    const w = htmlImg.naturalWidth || htmlImg.width
    const h = htmlImg.naturalHeight || htmlImg.height
    if (w <= 0 || h <= 0) return null

    const canvas = document.createElement('canvas')
    canvas.width = w
    canvas.height = h
    const ctx = canvas.getContext('2d')!
    ctx.drawImage(htmlImg, 0, 0, w, h)
    const imageData = ctx.getImageData(0, 0, w, h)

    return this.ck.MakeImage(
      {
        width: w,
        height: h,
        alphaType: this.ck.AlphaType.Unpremul,
        colorType: this.ck.ColorType.RGBA_8888,
        colorSpace: this.ck.ColorSpace.SRGB,
      },
      imageData.data,
      w * 4,
    ) ?? null
  }
}
