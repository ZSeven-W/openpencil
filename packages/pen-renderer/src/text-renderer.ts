import type { CanvasKit, Canvas, Image as SkImage, Paragraph } from 'canvaskit-wasm'
import type { PenNode, TextNode } from '@zseven-w/pen-types'
import type { PenEffect, ShadowEffect } from '@zseven-w/pen-types'
import { defaultLineHeight, cssFontFamily } from '@zseven-w/pen-core'
import { parseColor, resolveFillColor, wrapLine } from './paint-utils.js'
import { SkiaFontManager, type FontManagerOptions } from './font-manager.js'

/**
 * Text rendering sub-system for SkiaNodeRenderer.
 * Handles both vector (Paragraph API) and bitmap (Canvas 2D) text rendering
 * with caching for performance.
 */
export class SkiaTextRenderer {
  private ck: CanvasKit

  // Text rasterization cache (Canvas 2D -> CanvasKit Image)
  // FIFO eviction via Map insertion order; bytes tracked separately against TEXT_CACHE_BYTE_LIMIT.
  private textCache = new Map<string, SkImage | null>()
  private textCacheBytes = 0
  // 256 MB — each bitmap entry is ~cw*ch*4 bytes (RGBA pixels)
  private static TEXT_CACHE_BYTE_LIMIT = 256 * 1024 * 1024

  // Paragraph cache for vector text (keyed by content+style)
  // FIFO eviction via Map insertion order; bytes estimated from content length against PARA_CACHE_BYTE_LIMIT.
  private paraCache = new Map<string, Paragraph | null>()
  private paraCacheBytes = 0
  // 64 MB — each entry is estimated as content.length*64+4096 bytes (WASM heap approximation)
  private static PARA_CACHE_BYTE_LIMIT = 64 * 1024 * 1024

  // Pre-rasterized paragraph image cache (SkImage, same key as paraCache, zoom-independent)
  // Allows drawImageRect instead of drawParagraph on every frame — avoids per-frame glyph rasterization.
  private paraImageCache = new Map<string, SkImage | null>()
  private paraImageCacheBytes = 0
  // 128 MB — each entry is sw*sh*4 bytes (RGBA pixels at up to 2x DPR scale)
  private static PARA_IMAGE_CACHE_BYTE_LIMIT = 128 * 1024 * 1024

  private static estimateParaBytes(content: string): number {
    return content.length * 64 + 4096
  }

  // Current viewport zoom (set by engine before each render frame)
  zoom = 1

  // Device pixel ratio override
  devicePixelRatio: number | undefined

  private get _dpr(): number {
    return this.devicePixelRatio ?? (typeof window !== 'undefined' ? window.devicePixelRatio : 1) ?? 1
  }

  // Font manager for vector text rendering
  fontManager: SkiaFontManager

  constructor(ck: CanvasKit, fontOptions?: FontManagerOptions) {
    this.ck = ck
    this.fontManager = new SkiaFontManager(ck, fontOptions)
  }

  clearTextCache() {
    for (const img of this.textCache.values()) {
      img?.delete()
    }
    this.textCache.clear()
    this.textCacheBytes = 0
  }

  clearParaCache() {
    for (const p of this.paraCache.values()) {
      p?.delete()
    }
    this.paraCache.clear()
    this.paraCacheBytes = 0
    for (const img of this.paraImageCache.values()) {
      img?.delete()
    }
    this.paraImageCache.clear()
    this.paraImageCacheBytes = 0
  }

  // Evict oldest entries (Map head = first inserted) until there is room for `incoming` bytes.
  private evictParaCache(incoming: number) {
    while (this.paraCacheBytes + incoming > SkiaTextRenderer.PARA_CACHE_BYTE_LIMIT && this.paraCache.size > 0) {
      const [key, para] = this.paraCache.entries().next().value!
      para?.delete()
      this.paraCache.delete(key)
      this.paraCacheBytes -= SkiaTextRenderer.estimateParaBytes(key.split('|')[1] ?? '')
    }
  }

  private evictParaImageCache(incoming: number) {
    while (this.paraImageCacheBytes + incoming > SkiaTextRenderer.PARA_IMAGE_CACHE_BYTE_LIMIT && this.paraImageCache.size > 0) {
      const [key, img] = this.paraImageCache.entries().next().value!
      if (img) {
        this.paraImageCacheBytes -= img.width() * img.height() * 4
        img.delete()
      }
      this.paraImageCache.delete(key)
    }
  }

  private evictTextCache(incoming: number) {
    while (this.textCacheBytes + incoming > SkiaTextRenderer.TEXT_CACHE_BYTE_LIMIT && this.textCache.size > 0) {
      const [key, img] = this.textCache.entries().next().value!
      if (img) {
        this.textCacheBytes -= img.width() * img.height() * 4
        img.delete()
      }
      this.textCache.delete(key)
    }
  }

  dispose() {
    this.clearTextCache()
    this.clearParaCache()
    this.fontManager.dispose()
  }

  /**
   * Main text drawing entry — tries vector, falls back to bitmap.
   */
  drawText(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
    effects?: PenEffect[],
  ) {
    // Draw text shadow as blurred copy of the text glyphs (not a rectangle)
    const shadow = effects?.find((e): e is ShadowEffect => e.type === 'shadow')
    if (shadow) {
      this.drawTextShadow(canvas, node, x, y, w, h, opacity, shadow)
    }

    // Try vector text first (true Skia Paragraph API)
    const vectorOk = this.drawTextVector(canvas, node, x, y, w, h, opacity)
    if (vectorOk) return

    // Fallback to bitmap text rendering
    this.drawTextBitmap(canvas, node, x, y, w, h, opacity)
  }

  /**
   * Render text as true vector glyphs using CanvasKit's Paragraph API.
   * Returns true if rendered, false if font not available (caller should fallback).
   */
  drawTextVector(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, _h: number,
    opacity: number,
  ): boolean {
    const ck = this.ck
    const tNode = node as TextNode
    const content = typeof tNode.content === 'string'
      ? tNode.content
      : Array.isArray(tNode.content)
        ? tNode.content.map((s) => s.text ?? '').join('')
        : ''
    if (!content) return true

    const fontSize = tNode.fontSize ?? 16
    const fillColor = resolveFillColor(tNode.fill)
    const fontWeight = tNode.fontWeight ?? '400'
    const fontFamily = tNode.fontFamily ?? 'Inter'
    const textAlign: string = tNode.textAlign ?? 'left'
    const lineHeightMul = tNode.lineHeight ?? defaultLineHeight(fontSize)
    const textGrowth = tNode.textGrowth
    const letterSpacing = tNode.letterSpacing ?? 0

    const primaryFamily = fontFamily.split(',')[0].trim().replace(/['"]/g, '')
    if (!this.fontManager.isFontReady(primaryFamily)) {
      if (this.fontManager.isSystemFont(primaryFamily)) {
        return false
      }
      this.fontManager.ensureFont(primaryFamily).then((ok) => {
        if (ok) {
          this.clearParaCache()
          ;(this as any)._onFontLoaded?.()
        }
      })
      if (!this.fontManager.hasAnyFallback(primaryFamily)) {
        return false
      }
    }

    const isFixedWidth = textGrowth === 'fixed-width' || textGrowth === 'fixed-width-height'
    const fwTolerance = isFixedWidth ? Math.min(Math.ceil(w * 0.05), Math.ceil(fontSize * 0.5)) : 0
    const layoutWidth = isFixedWidth && w > 0 ? w + fwTolerance : 1e6
    const effectiveAlign = isFixedWidth ? textAlign : 'left'

    const cacheKey = `p|${content}|${fontSize}|${fillColor}|${fontWeight}|${fontFamily}|${effectiveAlign}|${Math.round(layoutWidth)}|${letterSpacing}|${lineHeightMul}`

    let para = this.paraCache.get(cacheKey)
    if (para === undefined) {
      const color = parseColor(ck, fillColor)

      let ckAlign = ck.TextAlign.Left
      if (effectiveAlign === 'center') ckAlign = ck.TextAlign.Center
      else if (effectiveAlign === 'right') ckAlign = ck.TextAlign.Right
      else if (effectiveAlign === 'justify') ckAlign = ck.TextAlign.Justify

      const weightNum = typeof fontWeight === 'number' ? fontWeight : parseInt(fontWeight as string, 10) || 400
      let ckWeight = ck.FontWeight.Normal
      if (weightNum <= 100) ckWeight = ck.FontWeight.Thin
      else if (weightNum <= 200) ckWeight = ck.FontWeight.ExtraLight
      else if (weightNum <= 300) ckWeight = ck.FontWeight.Light
      else if (weightNum <= 400) ckWeight = ck.FontWeight.Normal
      else if (weightNum <= 500) ckWeight = ck.FontWeight.Medium
      else if (weightNum <= 600) ckWeight = ck.FontWeight.SemiBold
      else if (weightNum <= 700) ckWeight = ck.FontWeight.Bold
      else if (weightNum <= 800) ckWeight = ck.FontWeight.ExtraBold
      else ckWeight = ck.FontWeight.Black

      const fallbackFamilies = this.fontManager.getFallbackChain(primaryFamily)

      const paraStyle = new ck.ParagraphStyle({
        textAlign: ckAlign,
        textStyle: {
          color,
          fontSize,
          fontFamilies: fallbackFamilies,
          fontStyle: { weight: ckWeight },
          letterSpacing,
          heightMultiplier: lineHeightMul,
          halfLeading: true,
        },
      })

      try {
        const builder = ck.ParagraphBuilder.MakeFromFontProvider(
          paraStyle,
          this.fontManager.getProvider(),
        )

        // Handle styled segments
        if (Array.isArray(tNode.content) && tNode.content.some(s => s.fontFamily || s.fontSize || s.fontWeight || s.fill)) {
          for (const seg of tNode.content) {
            if (seg.fontFamily || seg.fontSize || seg.fontWeight || seg.fill) {
              const segColor = seg.fill ? parseColor(ck, seg.fill) : color
              const segWeight = seg.fontWeight
                ? (typeof seg.fontWeight === 'number' ? seg.fontWeight : parseInt(seg.fontWeight as string, 10) || weightNum)
                : weightNum
              const segPrimary = seg.fontFamily?.split(',')[0].trim().replace(/['"]/g, '') ?? primaryFamily
              builder.pushStyle(new ck.TextStyle({
                color: segColor,
                fontSize: seg.fontSize ?? fontSize,
                fontFamilies: this.fontManager.getFallbackChain(segPrimary),
                fontStyle: { weight: segWeight as any },
                letterSpacing,
                heightMultiplier: lineHeightMul,
                halfLeading: true,
              }))
              builder.addText(seg.text ?? '')
              builder.pop()
            } else {
              builder.addText(seg.text ?? '')
            }
          }
        } else {
          builder.addText(content)
        }

        para = builder.build()
        para.layout(layoutWidth)
        builder.delete()
        const entryBytes = SkiaTextRenderer.estimateParaBytes(content)
        this.evictParaCache(entryBytes)
        this.paraCacheBytes += entryBytes
      } catch {
        para = null
      }

      this.paraCache.set(cacheKey, para ?? null)
    }

    if (!para) return false

    // Compute drawX and surface dimensions
    let drawX = x
    let surfaceW: number
    if (!isFixedWidth) {
      const longestLine = para.getLongestLine()
      surfaceW = longestLine + 2
      if (w > 0 && textAlign !== 'left') {
        if (textAlign === 'center') drawX = x + Math.max(0, (w - longestLine) / 2)
        else if (textAlign === 'right') drawX = x + Math.max(0, w - longestLine)
      }
    } else {
      surfaceW = layoutWidth
    }
    const surfaceH = para.getHeight() + 2

    // Try paragraph image cache: drawImageRect is far cheaper than drawParagraph per frame
    const imgScale = Math.min(this._dpr, 2)
    let cachedImg = this.paraImageCache.get(cacheKey)
    if (cachedImg === undefined) {
      cachedImg = null
      const sw = Math.min(Math.ceil(surfaceW * imgScale), 4096)
      const sh = Math.min(Math.ceil(surfaceH * imgScale), 4096)
      if (sw > 0 && sh > 0) {
        const surf: any = (ck as any).MakeSurface?.(sw, sh)
        if (surf) {
          const offCanvas = surf.getCanvas()
          offCanvas.scale(imgScale, imgScale)
          offCanvas.drawParagraph(para, 0, 0)
          cachedImg = (surf.makeImageSnapshot() as SkImage | null) ?? null
          surf.delete()
          if (cachedImg) {
            const imgBytes = sw * sh * 4
            this.evictParaImageCache(imgBytes)
            this.paraImageCacheBytes += imgBytes
          }
        }
      }
      this.paraImageCache.set(cacheKey, cachedImg)
    }

    if (cachedImg) {
      const imgW = cachedImg.width() / imgScale
      const imgH = cachedImg.height() / imgScale
      const paint = new ck.Paint()
      paint.setAntiAlias(true)
      if (opacity < 1) paint.setAlphaf(opacity)
      canvas.drawImageRect(
        cachedImg,
        ck.LTRBRect(0, 0, cachedImg.width(), cachedImg.height()),
        ck.LTRBRect(drawX, y, drawX + imgW, y + imgH),
        paint,
      )
      paint.delete()
      return true
    }

    // Fallback: surface creation failed, draw directly
    if (opacity < 1) {
      const paint = new ck.Paint()
      paint.setAlphaf(opacity)
      canvas.saveLayer(paint)
      paint.delete()
      canvas.drawParagraph(para, drawX, y)
      canvas.restore()
    } else {
      canvas.drawParagraph(para, drawX, y)
    }

    return true
  }

  /**
   * Draw text shadow as a blurred copy of the actual text glyphs.
   */
  private drawTextShadow(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
    shadow: ShadowEffect,
  ) {
    const ck = this.ck
    const tNode = node as TextNode
    const shadowFillColor = shadow.color ?? '#00000066'
    const shadowNode = {
      ...tNode,
      fill: [{ type: 'solid' as const, color: shadowFillColor }],
    } as PenNode

    const sx = x + shadow.offsetX
    const sy = y + shadow.offsetY

    if (shadow.blur > 0) {
      const paint = new ck.Paint()
      if (opacity < 1) paint.setAlphaf(opacity)
      const sigma = shadow.blur / 2
      const filter = ck.ImageFilter.MakeBlur(sigma, sigma, ck.TileMode.Decal, null)
      paint.setImageFilter(filter)
      canvas.saveLayer(paint)
      paint.delete()

      const vectorOk = this.drawTextVector(canvas, shadowNode, sx, sy, w, h, 1)
      if (!vectorOk) {
        this.drawTextBitmap(canvas, shadowNode, sx, sy, w, h, 1)
      }

      canvas.restore()
    } else {
      const vectorOk = this.drawTextVector(canvas, shadowNode, sx, sy, w, h, opacity)
      if (!vectorOk) {
        this.drawTextBitmap(canvas, shadowNode, sx, sy, w, h, opacity)
      }
    }
  }

  /** Bitmap text rendering fallback — supports all system fonts via Canvas 2D API. */
  drawTextBitmap(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
  ) {
    const ck = this.ck
    const tNode = node as TextNode
    const content = typeof tNode.content === 'string'
      ? tNode.content
      : Array.isArray(tNode.content)
        ? tNode.content.map((s) => s.text ?? '').join('')
        : ''

    if (!content) return

    const fontSize = tNode.fontSize ?? 16
    const fillColor = resolveFillColor(tNode.fill)
    const fontWeight = tNode.fontWeight ?? '400'
    const fontFamily = tNode.fontFamily ?? 'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif'
    const textAlign: string = tNode.textAlign ?? 'left'
    const lineHeightMul = tNode.lineHeight ?? defaultLineHeight(fontSize)
    const lineHeight = lineHeightMul * fontSize
    const textGrowth = tNode.textGrowth

    const isFixedWidth = textGrowth === 'fixed-width' || textGrowth === 'fixed-width-height'
      || (textGrowth !== 'auto' && textAlign !== 'left' && textAlign !== undefined)
    const shouldWrap = isFixedWidth && w > 0

    const measureCanvas = document.createElement('canvas')
    const mCtx = measureCanvas.getContext('2d')!
    mCtx.font = `${fontWeight} ${fontSize}px ${cssFontFamily(fontFamily)}`

    const rawLines = content.split('\n')
    let wrappedLines: string[]
    let renderW: number

    if (shouldWrap) {
      renderW = Math.max(w + fontSize * 0.2, 10)
      wrappedLines = []
      for (const raw of rawLines) {
        if (!raw) { wrappedLines.push(''); continue }
        wrapLine(mCtx, raw, renderW, wrappedLines)
      }
    } else {
      wrappedLines = rawLines.length > 0 ? rawLines : ['']
      let maxLineWidth = 0
      for (const line of wrappedLines) {
        if (line) maxLineWidth = Math.max(maxLineWidth, mCtx.measureText(line).width)
      }
      renderW = Math.max(maxLineWidth + 2, w, 10)
    }

    const FABRIC_FONT_MULT = 1.13
    const glyphH = fontSize * FABRIC_FONT_MULT
    const textH = Math.max(h,
      wrappedLines.length <= 1
        ? glyphH + 2
        : (wrappedLines.length - 1) * lineHeight + glyphH + 2,
    )

    const rawScale = this.zoom * this._dpr
    const scale = rawScale <= 2 ? 2 : rawScale <= 4 ? 4 : 8

    const cacheKey = `${content}|${fontSize}|${fillColor}|${fontWeight}|${fontFamily}|${textAlign}|${Math.round(renderW)}|${Math.round(textH)}|${scale}`

    let img = this.textCache.get(cacheKey)
    if (img === undefined) {
      let effectiveScale = scale
      let cw = Math.ceil(renderW * effectiveScale)
      let ch = Math.ceil(textH * effectiveScale)
      if (cw <= 0 || ch <= 0) { this.textCache.set(cacheKey, null); return }
      const MAX_TEX = 4096
      if (cw > MAX_TEX || ch > MAX_TEX) {
        effectiveScale = Math.min(MAX_TEX / renderW, MAX_TEX / textH, effectiveScale)
        cw = Math.ceil(renderW * effectiveScale)
        ch = Math.ceil(textH * effectiveScale)
      }

      const tmp = document.createElement('canvas')
      tmp.width = cw
      tmp.height = ch
      const ctx = tmp.getContext('2d')!
      ctx.scale(effectiveScale, effectiveScale)
      ctx.font = `${fontWeight} ${fontSize}px ${cssFontFamily(fontFamily)}`
      ctx.fillStyle = fillColor
      ctx.textBaseline = 'top'
      ctx.textAlign = (textAlign || 'left') as CanvasTextAlign

      let cy = 0
      for (const line of wrappedLines) {
        if (!line) { cy += lineHeight; continue }
        let tx = 0
        if (textAlign === 'center') tx = renderW / 2
        else if (textAlign === 'right') tx = renderW
        ctx.fillText(line, tx, cy)
        cy += lineHeight
      }

      const imageData = ctx.getImageData(0, 0, cw, ch)
      // Premultiply alpha for correct CanvasKit texture blending
      const premul = new Uint8Array(imageData.data.length)
      for (let p = 0; p < premul.length; p += 4) {
        const a = imageData.data[p + 3]
        if (a === 255) {
          premul[p] = imageData.data[p]
          premul[p + 1] = imageData.data[p + 1]
          premul[p + 2] = imageData.data[p + 2]
          premul[p + 3] = 255
        } else if (a > 0) {
          const f = a / 255
          premul[p] = Math.round(imageData.data[p] * f)
          premul[p + 1] = Math.round(imageData.data[p + 1] * f)
          premul[p + 2] = Math.round(imageData.data[p + 2] * f)
          premul[p + 3] = a
        }
      }
      img = ck.MakeImage(
        { width: cw, height: ch, alphaType: ck.AlphaType.Premul, colorType: ck.ColorType.RGBA_8888, colorSpace: ck.ColorSpace.SRGB },
        premul, cw * 4,
      ) ?? null

      const imgBytes = img ? cw * ch * 4 : 0
      this.evictTextCache(imgBytes)
      this.textCache.set(cacheKey, img)
      this.textCacheBytes += imgBytes
    }

    if (!img) return

    const paint = new ck.Paint()
    paint.setAntiAlias(true)
    if (opacity < 1) paint.setAlphaf(opacity)
    canvas.drawImageRect(
      img,
      ck.LTRBRect(0, 0, img.width(), img.height()),
      ck.LTRBRect(x, y, x + renderW, y + textH),
      paint,
    )
    paint.delete()
  }
}
