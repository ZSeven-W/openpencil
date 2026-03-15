import type { CanvasKit, Canvas, Paint, Font, Typeface, Image as SkImage, Paragraph } from 'canvaskit-wasm'
import type { PenNode, ContainerProps, TextNode, EllipseNode, LineNode, PolygonNode, PathNode, ImageNode, IconFontNode } from '@/types/pen'
import type { PenFill, PenStroke, PenEffect, ShadowEffect, ImageFill } from '@/types/styles'
import { DEFAULT_FILL, DEFAULT_STROKE, DEFAULT_STROKE_WIDTH } from '../canvas-constants'
import { defaultLineHeight } from '../canvas-text-measure'
import { lookupIconByName } from '@/services/ai/icon-resolver'
import { buildEllipseArcPath, isArcEllipse } from '@/utils/arc-path'
import { SkiaImageLoader } from './skia-image-loader'
import { SkiaFontManager } from './skia-font-manager'
import {
  parseColor,
  cornerRadiusValue,
  cornerRadii,
  resolveFillColor,
  resolveStrokeColor,
  resolveStrokeWidth,
  wrapLine,
  cssFontFamily,
} from './skia-paint-utils'
import { sanitizeSvgPath, hasInvalidNumbers, tryManualPathParse } from './skia-path-utils'
import {
  drawSelectionBorder as _drawSelectionBorder,
  drawFrameLabel as _drawFrameLabel,
  drawFrameLabelColored as _drawFrameLabelColored,
  drawHoverOutline as _drawHoverOutline,
  drawSelectionMarquee as _drawSelectionMarquee,
  drawGuide as _drawGuide,
  drawPenPreview as _drawPenPreview,
  drawAgentGlow as _drawAgentGlow,
  drawAgentBadge as _drawAgentBadge,
  drawAgentNodeBorder as _drawAgentNodeBorder,
  drawAgentPreviewFill as _drawAgentPreviewFill,
  type PenPreviewData,
} from './skia-overlays'

export interface RenderNode {
  node: PenNode
  absX: number
  absY: number
  absW: number
  absH: number
  clipRect?: { x: number; y: number; w: number; h: number; rx: number }
}

export class SkiaRenderer {
  private ck: CanvasKit
  private defaultTypeface: Typeface | null = null
  private defaultFont: Font | null = null

  // Text rasterization cache (Canvas 2D → CanvasKit Image)
  private textCache = new Map<string, SkImage | null>()
  private textCacheOrder: string[] = []
  private static TEXT_CACHE_MAX = 300

  // Paragraph cache for vector text (keyed by content+style, caches Paragraph objects)
  private paraCache = new Map<string, Paragraph | null>()
  private paraCacheOrder: string[] = []
  private static PARA_CACHE_MAX = 200

  // Current viewport zoom (set by engine before each render frame)
  zoom = 1

  // Font manager for vector text rendering
  fontManager: SkiaFontManager

  // Image loader
  imageLoader: SkiaImageLoader

  constructor(ck: CanvasKit) {
    this.ck = ck
    this.imageLoader = new SkiaImageLoader(ck)
    this.fontManager = new SkiaFontManager(ck)
  }

  init() {
    this.defaultFont = new this.ck.Font(null, 16)
  }

  /** Set callback to trigger re-render when async images finish loading. */
  setRedrawCallback(cb: () => void) {
    this.imageLoader.setOnLoaded(cb)
  }

  dispose() {
    this.defaultFont?.delete()
    this.defaultFont = null
    this.defaultTypeface?.delete()
    this.defaultTypeface = null
    this.clearTextCache()
    this.clearParaCache()
    this.fontManager.dispose()
    this.imageLoader.dispose()
  }

  clearTextCache() {
    for (const img of this.textCache.values()) {
      img?.delete()
    }
    this.textCache.clear()
    this.textCacheOrder = []
  }

  clearParaCache() {
    for (const p of this.paraCache.values()) {
      p?.delete()
    }
    this.paraCache.clear()
    this.paraCacheOrder = []
  }

  private evictParaCache() {
    while (this.paraCacheOrder.length > SkiaRenderer.PARA_CACHE_MAX) {
      const key = this.paraCacheOrder.shift()!
      const p = this.paraCache.get(key)
      p?.delete()
      this.paraCache.delete(key)
    }
  }

  private evictTextCache() {
    while (this.textCacheOrder.length > SkiaRenderer.TEXT_CACHE_MAX) {
      const key = this.textCacheOrder.shift()!
      const img = this.textCache.get(key)
      img?.delete()
      this.textCache.delete(key)
    }
  }

  // Fill paint

  private makeFillPaint(
    fills: PenFill[] | string | undefined,
    w: number,
    h: number,
    opacity: number,
    absX: number,
    absY: number,
  ): { paint: Paint; imageFillDraw?: { fill: ImageFill; w: number; h: number; absX: number; absY: number; opacity: number } } {
    const ck = this.ck
    const paint = new ck.Paint()
    paint.setStyle(ck.PaintStyle.Fill)
    paint.setAntiAlias(true)

    if (typeof fills === 'string') {
      const c = parseColor(ck, fills)
      c[3] *= opacity
      paint.setColor(c)
      return { paint }
    }
    if (!fills || fills.length === 0) {
      const c = parseColor(ck, DEFAULT_FILL)
      c[3] *= opacity
      paint.setColor(c)
      return { paint }
    }

    const first = fills[0]
    if (first.type === 'solid') {
      const c = parseColor(ck, first.color)
      c[3] *= (first.opacity ?? 1) * opacity
      paint.setColor(c)
    } else if (first.type === 'linear_gradient') {
      const stops = first.stops ?? []
      const fillOpacity = (first.opacity ?? 1) * opacity
      if (stops.length >= 2) {
        const angleDeg = first.angle ?? 0
        const rad = ((angleDeg - 90) * Math.PI) / 180
        const cos = Math.cos(rad)
        const sin = Math.sin(rad)
        const x1 = absX + w / 2 - (cos * w) / 2
        const y1 = absY + h / 2 - (sin * h) / 2
        const x2 = absX + w / 2 + (cos * w) / 2
        const y2 = absY + h / 2 + (sin * h) / 2
        const colors = stops.map((s) => {
          const c = parseColor(ck, s.color)
          c[3] *= fillOpacity
          return c
        })
        const positions = stops.map((s) => Math.max(0, Math.min(1, s.offset)))
        const shader = ck.Shader.MakeLinearGradient(
          [x1, y1], [x2, y2],
          colors, positions,
          ck.TileMode.Clamp,
        )
        if (shader) paint.setShader(shader)
      } else {
        const c = parseColor(ck, stops[0]?.color ?? DEFAULT_FILL)
        c[3] *= fillOpacity
        paint.setColor(c)
      }
    } else if (first.type === 'radial_gradient') {
      const stops = first.stops ?? []
      const fillOpacity = (first.opacity ?? 1) * opacity
      if (stops.length >= 2) {
        const cx = absX + (first.cx ?? 0.5) * w
        const cy = absY + (first.cy ?? 0.5) * h
        const r = (first.radius ?? 0.5) * Math.max(w, h)
        const colors = stops.map((s) => {
          const c = parseColor(ck, s.color)
          c[3] *= fillOpacity
          return c
        })
        const positions = stops.map((s) => Math.max(0, Math.min(1, s.offset)))
        const shader = ck.Shader.MakeRadialGradient(
          [cx, cy], r,
          colors, positions,
          ck.TileMode.Clamp,
        )
        if (shader) paint.setShader(shader)
      } else {
        const c = parseColor(ck, stops[0]?.color ?? DEFAULT_FILL)
        c[3] *= fillOpacity
        paint.setColor(c)
      }
    } else if (first.type === 'image') {
      const result = this.applyImageFillToPaint(paint, first, w, h, opacity, absX, absY)
      if (result.needsDrawImageRect && result.fill) {
        return { paint, imageFillDraw: { fill: result.fill, w: result.w!, h: result.h!, absX: result.absX!, absY: result.absY!, opacity: result.opacity! } }
      }
    }

    return { paint }
  }

  /**
   * Apply an image fill to a Paint object using an image shader.
   * If the image is not yet loaded, a placeholder color is used.
   */
  /**
   * Apply an image fill to a Paint object.
   * For tile mode: uses a shader with TileMode.Repeat.
   * For fill/fit/crop/stretch: sets a placeholder paint and returns
   * draw info so the caller can use drawImageRect (shader scaling
   * is unreliable in CanvasKit for Clamp/Decal tile modes).
   */
  private applyImageFillToPaint(
    paint: Paint,
    fill: ImageFill,
    w: number, h: number,
    opacity: number,
    absX: number, absY: number,
  ): { needsDrawImageRect: boolean; fill?: ImageFill; w?: number; h?: number; absX?: number; absY?: number; opacity?: number } {
    const ck = this.ck
    const fillOpacity = (fill.opacity ?? 1) * opacity
    const url = fill.url
    if (!url) {
      const c = parseColor(ck, '#e5e7eb')
      c[3] *= fillOpacity
      paint.setColor(c)
      return { needsDrawImageRect: false }
    }

    const cached = this.imageLoader.get(url)
    if (cached === undefined) {
      this.imageLoader.request(url)
    }
    if (!cached) {
      const c = parseColor(ck, '#e5e7eb')
      c[3] *= fillOpacity
      paint.setColor(c)
      return { needsDrawImageRect: false }
    }

    const imgW = cached.width()
    const imgH = cached.height()
    if (imgW <= 0 || imgH <= 0) return { needsDrawImageRect: false }

    const mode = fill.mode ?? 'fill'

    // Tile mode: use shader (works reliably with Repeat + translation matrix)
    if (mode === 'tile') {
      const dispX = absX + (w - imgW) / 2
      const dispY = absY + (h - imgH) / 2
      const localMatrix = Float32Array.of(
        1, 0, -dispX,
        0, 1, -dispY,
        0, 0, 1,
      )
      const shader = cached.makeShaderOptions(
        ck.TileMode.Repeat, ck.TileMode.Repeat,
        ck.FilterMode.Linear, ck.MipmapMode.None,
        localMatrix,
      )
      if (shader) {
        paint.setShader(shader)
        if (fillOpacity < 1) paint.setAlphaf(fillOpacity)
        const cf = this.buildImageAdjustmentFilter(fill)
        if (cf) paint.setColorFilter(cf)
      }
      return { needsDrawImageRect: false }
    }

    // For fill/fit/crop/stretch: use transparent paint, caller draws image via drawImageRect
    paint.setColor(Float32Array.of(0, 0, 0, 0))
    return { needsDrawImageRect: true, fill, w, h, absX, absY, opacity: fillOpacity }
  }

  /**
   * Draw an image fill using drawImageRect (for fill/fit/crop/stretch modes).
   * Must be called after clipping to the shape bounds.
   */
  private drawImageFillRect(
    canvas: Canvas,
    fill: ImageFill,
    w: number, h: number,
    absX: number, absY: number,
    fillOpacity: number,
  ) {
    const ck = this.ck
    const url = fill.url
    if (!url) return

    const cached = this.imageLoader.get(url)
    if (!cached) return

    const imgW = cached.width()
    const imgH = cached.height()
    if (imgW <= 0 || imgH <= 0) return

    const mode = fill.mode ?? 'fill'
    const paint = new ck.Paint()
    paint.setAntiAlias(true)
    if (fillOpacity < 1) paint.setAlphaf(fillOpacity)

    const adjFilter = this.buildImageAdjustmentFilter(fill)
    if (adjFilter) paint.setColorFilter(adjFilter)

    if (mode === 'fit') {
      // Contain: entire image visible, centered, with letterbox
      const scale = Math.min(w / imgW, h / imgH)
      const dw = imgW * scale
      const dh = imgH * scale
      const dx = absX + (w - dw) / 2
      const dy = absY + (h - dh) / 2
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      )
    } else if (mode === 'stretch') {
      // Stretch: distort to fill entire area
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(absX, absY, absX + w, absY + h),
        paint,
      )
    } else {
      // 'fill', 'crop': cover, centered, excess clipped by parent clip
      const scale = Math.max(w / imgW, h / imgH)
      const dw = imgW * scale
      const dh = imgH * scale
      const dx = absX + (w - dw) / 2
      const dy = absY + (h - dh) / 2
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      )
    }

    paint.delete()
  }

  /**
   * Build a CanvasKit ColorFilter from image adjustment values.
   * Builds a single 4x5 color matrix combining all adjustments.
   *
   * Matrix layout (row-major 4×5):
   *   R' = m[0]*r + m[1]*g + m[2]*b  + m[3]*a + m[4]
   *   G' = m[5]*r + m[6]*g + m[7]*b  + m[8]*a + m[9]
   *   B' = m[10]*r+ m[11]*g+ m[12]*b + m[13]*a+ m[14]
   *   A' = m[15]*r+ m[16]*g+ m[17]*b + m[18]*a+ m[19]
   */
  private buildImageAdjustmentFilter(adj: {
    exposure?: number; contrast?: number; saturation?: number
    temperature?: number; tint?: number; highlights?: number; shadows?: number
  }) {
    const ck = this.ck
    const exp = (adj.exposure ?? 0) / 100
    const con = (adj.contrast ?? 0) / 100
    const sat = (adj.saturation ?? 0) / 100
    const temp = (adj.temperature ?? 0) / 100
    const tintVal = (adj.tint ?? 0) / 100
    const hi = (adj.highlights ?? 0) / 100
    const sh = (adj.shadows ?? 0) / 100

    if (exp === 0 && con === 0 && sat === 0 && temp === 0 && tintVal === 0 && hi === 0 && sh === 0) {
      return null
    }

    // Exposure: brightness multiplier
    const e = 1 + exp * 1.5

    // Contrast: scale around 0.5 midpoint
    const c = 1 + con
    const cOff = 0.5 * (1 - c)

    // Saturation: luminance-preserving mix
    const s = 1 + sat
    const lr = 0.2126, lg = 0.7152, lb = 0.0722
    const sr = (1 - s) * lr, sg = (1 - s) * lg, sb = (1 - s) * lb

    // Combined scale factor for each matrix cell: contrast * exposure * saturation
    // Order: saturate → exposure → contrast
    // saturated_R = (sr+s)*r + sg*g + sb*b
    // exposed_R   = e * saturated_R
    // final_R     = c * exposed_R + cOff + offsets
    const f = c * e

    // Offsets: temperature (warm/cool), tint, highlights, shadows
    const offR = cOff + temp * 0.15 + (hi + sh * 0.5) * 0.1
    const offG = cOff + tintVal * 0.15 + (hi + sh * 0.5) * 0.1
    const offB = cOff - temp * 0.15 + (hi + sh * 0.5) * 0.1

    const m = [
      f * (sr + s), f * sg,       f * sb,       0, offR,
      f * sr,       f * (sg + s), f * sb,       0, offG,
      f * sr,       f * sg,       f * (sb + s), 0, offB,
      0,            0,            0,            1, 0,
    ]

    return ck.ColorFilter.MakeMatrix(m)
  }

  // Stroke paint

  private makeStrokePaint(
    stroke: PenStroke | undefined,
    opacity: number,
  ): Paint | null {
    if (!stroke) return null
    const strokeColor = resolveStrokeColor(stroke)
    const strokeWidth = resolveStrokeWidth(stroke)
    if (!strokeColor || strokeWidth <= 0) return null

    const ck = this.ck
    const paint = new ck.Paint()
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setAntiAlias(true)
    paint.setStrokeWidth(strokeWidth)

    const c = parseColor(ck, strokeColor)
    c[3] *= opacity
    paint.setColor(c)

    if (stroke.join === 'round') paint.setStrokeJoin(ck.StrokeJoin.Round)
    else if (stroke.join === 'bevel') paint.setStrokeJoin(ck.StrokeJoin.Bevel)

    if (stroke.cap === 'round') paint.setStrokeCap(ck.StrokeCap.Round)
    else if (stroke.cap === 'square') paint.setStrokeCap(ck.StrokeCap.Square)

    if (stroke.dashPattern && stroke.dashPattern.length >= 2) {
      const effect = ck.PathEffect.MakeDash(stroke.dashPattern, 0)
      if (effect) paint.setPathEffect(effect)
    }

    return paint
  }

  // Shadow / blur

  // applyShadowDirect is used instead of saveLayer approach

  // Draw a single render node

  drawNode(canvas: Canvas, rn: RenderNode, selectedIds: Set<string>) {
    const { node, absX, absY, absW, absH, clipRect } = rn
    const ck = this.ck
    const opacity = typeof node.opacity === 'number' ? node.opacity : 1

    if (('visible' in node ? node.visible : undefined) === false) return

    // Apply clipping from parent frame
    let clipped = false
    if (clipRect) {
      canvas.save()
      clipped = true
      const radii = clipRect.rx
      if (radii > 0) {
        const rrect = ck.RRectXY(
          ck.LTRBRect(clipRect.x, clipRect.y, clipRect.x + clipRect.w, clipRect.y + clipRect.h),
          radii, radii,
        )
        canvas.clipRRect(rrect, ck.ClipOp.Intersect, true)
      } else {
        canvas.clipRect(
          ck.LTRBRect(clipRect.x, clipRect.y, clipRect.x + clipRect.w, clipRect.y + clipRect.h),
          ck.ClipOp.Intersect,
          true,
        )
      }
    }

    // Apply flip (flipX / flipY from Figma import)
    const flipX = node.flipX === true
    const flipY = node.flipY === true
    if (flipX || flipY) {
      canvas.save()
      canvas.translate(absX + absW / 2, absY + absH / 2)
      canvas.scale(flipX ? -1 : 1, flipY ? -1 : 1)
      canvas.translate(-(absX + absW / 2), -(absY + absH / 2))
    }

    // Apply rotation
    const rotation = node.rotation ?? 0
    if (rotation !== 0) {
      canvas.save()
      canvas.rotate(rotation, absX + absW / 2, absY + absH / 2)
    }

    // Apply shadow (text uses glyph-shaped shadow, not rectangle)
    const effects = 'effects' in node ? (node as PenNode & { effects?: PenEffect[] }).effects : undefined
    if (node.type !== 'text') {
      this.applyShadowDirect(canvas, effects, absX, absY, absW, absH)
    }

    switch (node.type) {
      case 'frame':
      case 'rectangle':
      case 'group':
        this.drawRect(canvas, node, absX, absY, absW, absH, opacity)
        break
      case 'ellipse':
        this.drawEllipse(canvas, node, absX, absY, absW, absH, opacity)
        break
      case 'line':
        this.drawLine(canvas, node, absX, absY, opacity)
        break
      case 'polygon':
        this.drawPolygon(canvas, node, absX, absY, absW, absH, opacity)
        break
      case 'path':
        this.drawPath(canvas, node, absX, absY, absW, absH, opacity, clipRect)
        break
      case 'icon_font':
        this.drawIconFont(canvas, node, absX, absY, absW, absH, opacity)
        break
      case 'text':
        this.drawText(canvas, node, absX, absY, absW, absH, opacity, effects)
        break
      case 'image':
        this.drawImage(canvas, node, absX, absY, absW, absH, opacity)
        break
    }

    // Selection highlight
    if (selectedIds.has(node.id)) {
      this.drawSelectionBorder(canvas, absX, absY, absW, absH)
    }

    if (rotation !== 0) canvas.restore()
    if (flipX || flipY) canvas.restore()
    if (clipped) canvas.restore()
  }

  // Shadow (direct, not saveLayer)

  private applyShadowDirect(
    canvas: Canvas,
    effects: PenEffect[] | undefined,
    x: number, y: number, w: number, h: number,
  ): boolean {
    if (!effects) return false
    const shadow = effects.find((e): e is ShadowEffect => e.type === 'shadow')
    if (!shadow) return false

    const ck = this.ck
    const paint = new ck.Paint()
    paint.setStyle(ck.PaintStyle.Fill)
    paint.setAntiAlias(true)
    const c = parseColor(ck, shadow.color)
    paint.setColor(c)
    const filter = ck.MaskFilter.MakeBlur(ck.BlurStyle.Normal, shadow.blur / 2, true)
    paint.setMaskFilter(filter)
    canvas.drawRect(
      ck.LTRBRect(
        x + shadow.offsetX - shadow.spread,
        y + shadow.offsetY - shadow.spread,
        x + w + shadow.offsetX + shadow.spread,
        y + h + shadow.offsetY + shadow.spread,
      ),
      paint,
    )
    paint.delete()
    return true
  }

  // Shape drawing

  private drawRect(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
  ) {
    const ck = this.ck
    const container = node as PenNode & ContainerProps
    const cr = cornerRadii(container.cornerRadius)
    const fills = container.fill
    const stroke = container.stroke

    // For frames/groups without explicit fill, use transparent (no visible background)
    const hasFill = fills && fills.length > 0
    const isContainer = node.type === 'frame' || node.type === 'group'

    // Fill
    const { paint: fillPaint, imageFillDraw } = this.makeFillPaint(
      hasFill ? fills : (isContainer ? 'transparent' : undefined),
      w, h, opacity, x, y,
    )

    const hasRoundedCorners = cr.some((r) => r > 0)
    if (hasRoundedCorners) {
      const maxR = Math.min(w / 2, h / 2)
      const rrect = ck.RRectXY(
        ck.LTRBRect(x, y, x + w, y + h),
        Math.min(cr[0], maxR), Math.min(cr[0], maxR),
      )
      canvas.drawRRect(rrect, fillPaint)
    } else {
      canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), fillPaint)
    }
    fillPaint.delete()

    // Image fill (fill/fit/crop/stretch): draw via drawImageRect with clipping
    if (imageFillDraw) {
      canvas.save()
      if (hasRoundedCorners) {
        const maxR = Math.min(w / 2, h / 2)
        canvas.clipRRect(
          ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), Math.min(cr[0], maxR), Math.min(cr[0], maxR)),
          ck.ClipOp.Intersect, true,
        )
      } else {
        canvas.clipRect(ck.LTRBRect(x, y, x + w, y + h), ck.ClipOp.Intersect, true)
      }
      this.drawImageFillRect(canvas, imageFillDraw.fill, imageFillDraw.w, imageFillDraw.h, imageFillDraw.absX, imageFillDraw.absY, imageFillDraw.opacity)
      canvas.restore()
    }

    // Stroke
    const strokePaint = this.makeStrokePaint(stroke, opacity)
    if (strokePaint) {
      if (hasRoundedCorners) {
        const maxR = Math.min(w / 2, h / 2)
        const rrect = ck.RRectXY(
          ck.LTRBRect(x, y, x + w, y + h),
          Math.min(cr[0], maxR), Math.min(cr[0], maxR),
        )
        canvas.drawRRect(rrect, strokePaint)
      } else {
        canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), strokePaint)
      }
      strokePaint.delete()
    }
  }

  private drawEllipse(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
  ) {
    const ck = this.ck
    const eNode = node as EllipseNode
    const fills = eNode.fill
    const stroke = eNode.stroke

    if (isArcEllipse(eNode.startAngle, eNode.sweepAngle, eNode.innerRadius)) {
      const arcD = buildEllipseArcPath(w, h, eNode.startAngle ?? 0, eNode.sweepAngle ?? 360, eNode.innerRadius ?? 0)
      const path = ck.Path.MakeFromSVGString(arcD)
      if (path) {
        path.offset(x, y)
        const { paint: fillPaint } = this.makeFillPaint(fills, w, h, opacity, x, y)
        fillPaint.setAntiAlias(true)
        canvas.drawPath(path, fillPaint)
        fillPaint.delete()
        path.delete()
      }
      return
    }

    const { paint: fillPaint } = this.makeFillPaint(fills, w, h, opacity, x, y)
    canvas.drawOval(ck.LTRBRect(x, y, x + w, y + h), fillPaint)
    fillPaint.delete()

    const strokePaint = this.makeStrokePaint(stroke, opacity)
    if (strokePaint) {
      canvas.drawOval(ck.LTRBRect(x, y, x + w, y + h), strokePaint)
      strokePaint.delete()
    }
  }

  private drawLine(
    canvas: Canvas, node: PenNode,
    x: number, y: number,
    opacity: number,
  ) {
    const ck = this.ck
    const lNode = node as LineNode
    const x2 = lNode.x2 ?? x + 100
    const y2 = lNode.y2 ?? y
    const strokeColor = resolveStrokeColor(lNode.stroke) ?? DEFAULT_STROKE
    const strokeWidth = resolveStrokeWidth(lNode.stroke) || DEFAULT_STROKE_WIDTH

    const paint = new ck.Paint()
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setAntiAlias(true)
    paint.setStrokeWidth(strokeWidth)
    const c = parseColor(ck, strokeColor)
    c[3] *= opacity
    paint.setColor(c)

    canvas.drawLine(x, y, x2, y2, paint)
    paint.delete()
  }

  private drawPolygon(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
  ) {
    const ck = this.ck
    const pNode = node as PolygonNode
    const count = pNode.polygonCount || 6
    const fills = pNode.fill
    const stroke = pNode.stroke

    const path = new ck.Path()
    for (let i = 0; i < count; i++) {
      const angle = (i * 2 * Math.PI) / count - Math.PI / 2
      const px = x + (w / 2) * Math.cos(angle) + w / 2
      const py = y + (h / 2) * Math.sin(angle) + h / 2
      if (i === 0) path.moveTo(px, py)
      else path.lineTo(px, py)
    }
    path.close()

    const { paint: fillPaint } = this.makeFillPaint(fills, w, h, opacity, x, y)
    canvas.drawPath(path, fillPaint)
    fillPaint.delete()

    const strokePaint = this.makeStrokePaint(stroke, opacity)
    if (strokePaint) {
      canvas.drawPath(path, strokePaint)
      strokePaint.delete()
    }
    path.delete()
  }

  private drawPath(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
    _clipRect?: { x: number; y: number; w: number; h: number; rx: number },
  ) {
    const ck = this.ck
    const pNode = node as PathNode
    const rawD = typeof pNode.d === 'string' && pNode.d.trim().length > 0 ? pNode.d : 'M0 0 L0 0'
    const fills = pNode.fill
    const stroke = pNode.stroke

    // If path contains NaN/Infinity (e.g. corrupted Figma binary data),
    // go straight to manual parser which skips invalid commands (like Canvas 2D does).
    let path: ReturnType<typeof ck.Path.MakeFromSVGString> = null
    if (hasInvalidNumbers(rawD)) {
      path = tryManualPathParse(ck, rawD)
    } else {
      // Sanitize and try CanvasKit's native parser first
      const d = sanitizeSvgPath(rawD)
      path = ck.Path.MakeFromSVGString(d)
      if (!path && d !== rawD) {
        path = ck.Path.MakeFromSVGString(rawD)
      }
      if (!path) {
        path = tryManualPathParse(ck, rawD)
      }
    }
    if (!path) {
      // Render fallback with the node's fill color (not debug red)
      if (w > 0 && h > 0) {
        const { paint: fillPaint } = this.makeFillPaint(fills, w, h, opacity, x, y)
        canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), fillPaint)
        fillPaint.delete()
      }
      return
    }

    // Get native bounds and scale to target size
    const bounds = path.getBounds()
    const nativeW = bounds[2] - bounds[0]
    const nativeH = bounds[3] - bounds[1]

    if (w > 0 && h > 0 && nativeW > 0.01 && nativeH > 0.01) {
      // Icons (with iconId): uniform scaling to preserve aspect ratio.
      // All other paths: non-uniform scaling to fill target bounding box exactly.
      // Figma vector paths may have unscaled normalized coordinates that need
      // different X/Y scaling to match the node's target width/height.
      const isIcon = !!pNode.iconId
      const sx = isIcon ? Math.min(w / nativeW, h / nativeH) : w / nativeW
      const sy = isIcon ? sx : h / nativeH
      const matrix = ck.Matrix.multiply(
        ck.Matrix.translated(x - bounds[0] * sx, y - bounds[1] * sy),
        ck.Matrix.scaled(sx, sy),
      )
      path.transform(matrix)
    } else if (nativeW > 0.01 || nativeH > 0.01) {
      // Degenerate path (one dimension is ~0, e.g. horizontal/vertical line)
      const sx = nativeW > 0.01 && w > 0 ? w / nativeW : 1
      const sy = nativeH > 0.01 && h > 0 ? h / nativeH : 1
      const matrix = ck.Matrix.multiply(
        ck.Matrix.translated(x - bounds[0] * sx, y - bounds[1] * sy),
        ck.Matrix.scaled(sx, sy),
      )
      path.transform(matrix)
    } else {
      path.offset(x, y)
    }

    const hasExplicitFill = fills && fills.length > 0
    const strokeColor = resolveStrokeColor(stroke)
    const strokeWidth = resolveStrokeWidth(stroke)
    const hasVisibleStroke = strokeWidth > 0 && !!strokeColor

    // Fill — use EvenOdd for compound paths (multiple sub-paths), Winding for simple paths
    if (hasExplicitFill || !hasVisibleStroke) {
      const { paint: fillPaint } = this.makeFillPaint(
        hasExplicitFill ? fills : undefined,
        w, h, opacity, x, y,
      )
      // Detect compound paths: more than one close command indicates multiple sub-paths
      const closeCount = (rawD.match(/Z/gi) || []).length
      const isCompound = closeCount > 1
      path.setFillType(isCompound ? ck.FillType.EvenOdd : ck.FillType.Winding)
      canvas.drawPath(path, fillPaint)
      fillPaint.delete()
    }

    // Stroke
    if (hasVisibleStroke) {
      const strokePaint = this.makeStrokePaint(stroke, opacity)
      if (strokePaint) {
        canvas.drawPath(path, strokePaint)
        strokePaint.delete()
      }
    }

    path.delete()
  }

  private drawIconFont(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
  ) {
    const ck = this.ck
    const iNode = node as IconFontNode
    const iconName = iNode.iconFontName ?? iNode.name ?? ''
    const iconMatch = lookupIconByName(iconName)
    const iconD = iconMatch?.d ?? 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0'
    const iconStyle = iconMatch?.style ?? 'stroke'

    const rawFill = iNode.fill
    const iconFillColor = typeof rawFill === 'string'
      ? rawFill
      : Array.isArray(iNode.fill) && iNode.fill.length > 0
        ? resolveFillColor(iNode.fill)
        : '#64748B'

    // Sanitize path data and try multiple parse strategies (same as drawPath)
    const sanitizedIconD = sanitizeSvgPath(iconD)
    let path = ck.Path.MakeFromSVGString(sanitizedIconD)
    if (!path && sanitizedIconD !== iconD) {
      path = ck.Path.MakeFromSVGString(iconD)
    }
    if (!path) {
      path = tryManualPathParse(ck, iconD)
    }
    if (!path) return

    const bounds = path.getBounds()
    const nativeW = bounds[2] - bounds[0]
    const nativeH = bounds[3] - bounds[1]
    if (w > 0 && h > 0 && nativeW > 0 && nativeH > 0) {
      const uniformScale = Math.min(w / nativeW, h / nativeH)
      const matrix = ck.Matrix.multiply(
        ck.Matrix.translated(x - bounds[0] * uniformScale, y - bounds[1] * uniformScale),
        ck.Matrix.scaled(uniformScale, uniformScale),
      )
      path.transform(matrix)
    } else {
      path.offset(x, y)
    }

    if (iconStyle === 'stroke') {
      const paint = new ck.Paint()
      paint.setStyle(ck.PaintStyle.Stroke)
      paint.setAntiAlias(true)
      paint.setStrokeWidth(2)
      paint.setStrokeCap(ck.StrokeCap.Round)
      paint.setStrokeJoin(ck.StrokeJoin.Round)
      const c = parseColor(ck, iconFillColor)
      c[3] *= opacity
      paint.setColor(c)
      canvas.drawPath(path, paint)
      paint.delete()
    } else {
      const paint = new ck.Paint()
      paint.setStyle(ck.PaintStyle.Fill)
      paint.setAntiAlias(true)
      const c = parseColor(ck, iconFillColor)
      c[3] *= opacity
      paint.setColor(c)
      path.setFillType(ck.FillType.EvenOdd)
      canvas.drawPath(path, paint)
      paint.delete()
    }

    path.delete()
  }

  /**
   * Render text as true vector glyphs using CanvasKit's Paragraph API.
   * Returns true if rendered, false if font not available (caller should fallback).
   */
  private drawTextVector(
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

    // Check if primary font family is loaded; if not, try async load
    const primaryFamily = fontFamily.split(',')[0].trim().replace(/['"]/g, '')
    if (!this.fontManager.isFontReady(primaryFamily)) {
      // System fonts can't be loaded into CanvasKit — use bitmap rendering
      // which supports all OS-installed fonts via Canvas 2D API
      if (this.fontManager.isSystemFont(primaryFamily)) {
        return false
      }
      this.fontManager.ensureFont(primaryFamily).then((ok) => {
        if (ok) {
          this.clearParaCache()
          ;(this as any)._onFontLoaded?.()
        }
      })
      // If no fallback font is available, fall back to bitmap rendering
      if (!this.fontManager.hasAnyFallback(primaryFamily)) {
        return false
      }
    }

    // Fixed-width text uses node width for wrapping; paragraph handles alignment.
    // Auto-width text uses unbounded layout (no wrapping); alignment is handled
    // by manually offsetting the draw position after layout.
    const isFixedWidth = textGrowth === 'fixed-width' || textGrowth === 'fixed-width-height'
    // Add tolerance for fixed-width text to prevent unwanted wrapping from
    // font metric differences between design tools (Figma) and CanvasKit/Skia.
    // Use 5% of width capped at half fontSize to avoid affecting intentional wrapping.
    const fwTolerance = isFixedWidth ? Math.min(Math.ceil(w * 0.05), Math.ceil(fontSize * 0.5)) : 0
    const layoutWidth = isFixedWidth && w > 0 ? w + fwTolerance : 1e6
    // For auto-width text, force LEFT alignment in the paragraph to prevent
    // centering within the 1e6 layout width. We manually offset x when drawing.
    const effectiveAlign = isFixedWidth ? textAlign : 'left'

    // Cache key for paragraph object
    const cacheKey = `p|${content}|${fontSize}|${fillColor}|${fontWeight}|${fontFamily}|${effectiveAlign}|${Math.round(layoutWidth)}|${letterSpacing}|${lineHeightMul}`

    let para = this.paraCache.get(cacheKey)
    if (para === undefined) {
      const color = parseColor(ck, fillColor)

      // Map text alignment
      let ckAlign = ck.TextAlign.Left
      if (effectiveAlign === 'center') ckAlign = ck.TextAlign.Center
      else if (effectiveAlign === 'right') ckAlign = ck.TextAlign.Right
      else if (effectiveAlign === 'justify') ckAlign = ck.TextAlign.Justify

      // Map font weight
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

      // Build font fallback chain: primary font → Inter (has latin-ext for ₦ etc.)
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
      } catch {
        para = null
      }

      this.paraCache.set(cacheKey, para ?? null)
      this.paraCacheOrder.push(cacheKey)
      this.evictParaCache()
    }

    if (!para) return false

    // For auto-width text with non-left alignment, manually offset draw position
    // (the paragraph uses LEFT alignment to avoid centering in infinite space)
    let drawX = x
    if (!isFixedWidth && w > 0 && textAlign !== 'left') {
      const longestLine = para.getLongestLine()
      if (textAlign === 'center') drawX = x + Math.max(0, (w - longestLine) / 2)
      else if (textAlign === 'right') drawX = x + Math.max(0, w - longestLine)
    }

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
   * Draw text shadow as a blurred copy of the actual text glyphs,
   * matching Figma's drop-shadow behavior (shadow follows glyph outlines).
   */
  private drawTextShadow(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
    shadow: ShadowEffect,
  ) {
    const ck = this.ck
    const tNode = node as TextNode

    // Create a shadow-colored version of the text node
    const shadowFillColor = shadow.color ?? '#00000066'
    const shadowNode = {
      ...tNode,
      fill: [{ type: 'solid' as const, color: shadowFillColor }],
    } as PenNode

    const sx = x + shadow.offsetX
    const sy = y + shadow.offsetY

    if (shadow.blur > 0) {
      // Use saveLayer with blur ImageFilter to blur the text glyphs
      const paint = new ck.Paint()
      if (opacity < 1) paint.setAlphaf(opacity)
      const sigma = shadow.blur / 2
      const filter = ck.ImageFilter.MakeBlur(sigma, sigma, ck.TileMode.Decal, null)
      paint.setImageFilter(filter)
      canvas.saveLayer(paint)
      paint.delete()

      // Draw shadow text (vector path first, then bitmap fallback)
      const vectorOk = this.drawTextVector(canvas, shadowNode, sx, sy, w, h, 1)
      if (!vectorOk) {
        this.drawTextBitmap(canvas, shadowNode, sx, sy, w, h, 1)
      }

      canvas.restore()
    } else {
      // No blur — just draw offset text with shadow color
      const vectorOk = this.drawTextVector(canvas, shadowNode, sx, sy, w, h, opacity)
      if (!vectorOk) {
        this.drawTextBitmap(canvas, shadowNode, sx, sy, w, h, opacity)
      }
    }
  }

  /**
   * Render text using browser Canvas 2D API (supports all system fonts including CJK),
   * then draw the rasterized result as a CanvasKit image. Results are cached.
   */
  private drawText(
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

    // Try vector text first (true Skia Paragraph API — no pixelation at any zoom)
    const vectorOk = this.drawTextVector(canvas, node, x, y, w, h, opacity)
    if (vectorOk) return

    // Fallback to bitmap text rendering
    this.drawTextBitmap(canvas, node, x, y, w, h, opacity)
  }

  /** Bitmap text rendering fallback — supports all system fonts via Canvas 2D API. */
  private drawTextBitmap(
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

    // Match Fabric.js wrapping logic (isFixedWidthText in canvas-object-factory):
    // Only wrap when textGrowth is explicitly 'fixed-width'/'fixed-width-height',
    // or textAlign is non-left AND textGrowth isn't explicitly 'auto'.
    // textGrowth='auto' means auto-width (no wrapping) regardless of textAlign,
    // since for auto-sized text centering is a no-op anyway.
    const isFixedWidth = textGrowth === 'fixed-width' || textGrowth === 'fixed-width-height'
      || (textGrowth !== 'auto' && textAlign !== 'left' && textAlign !== undefined)
    const shouldWrap = isFixedWidth && w > 0

    // Set up measurement context
    const measureCanvas = document.createElement('canvas')
    const mCtx = measureCanvas.getContext('2d')!
    mCtx.font = `${fontWeight} ${fontSize}px ${cssFontFamily(fontFamily)}`

    const rawLines = content.split('\n')
    let wrappedLines: string[]
    let renderW: number

    if (shouldWrap) {
      // Fixed-width text: wrap with tolerance for font metric differences
      renderW = Math.max(w + fontSize * 0.2, 10)
      wrappedLines = []
      for (const raw of rawLines) {
        if (!raw) { wrappedLines.push(''); continue }
        wrapLine(mCtx, raw, renderW, wrappedLines)
      }
    } else {
      // Auto-sized text: don't wrap, measure actual width from Canvas 2D
      wrappedLines = rawLines.length > 0 ? rawLines : ['']
      let maxLineWidth = 0
      for (const line of wrappedLines) {
        if (line) maxLineWidth = Math.max(maxLineWidth, mCtx.measureText(line).width)
      }
      renderW = Math.max(maxLineWidth + 2, w, 10)
    }

    // Match Fabric.js: _fontSizeMult = 1.13 for the base glyph height.
    // lineHeight only adds spacing BETWEEN lines, not below the last line.
    const FABRIC_FONT_MULT = 1.13
    const glyphH = fontSize * FABRIC_FONT_MULT
    const textH = Math.max(h,
      wrappedLines.length <= 1
        ? glyphH + 2
        : (wrappedLines.length - 1) * lineHeight + glyphH + 2,
    )

    // Zoom-aware rasterization scale: quantized to 2/4/8 for cache efficiency.
    // At zoom ≤ 1 with 2× DPR → scale 2 (1:1 pixel mapping on Retina).
    // Higher zoom → 4 or 8 so text remains sharp when zoomed in.
    const dpr = window.devicePixelRatio || 1
    const rawScale = this.zoom * dpr
    const scale = rawScale <= 2 ? 2 : rawScale <= 4 ? 4 : 8

    // Cache key — includes rasterization scale so zoom changes use fresh textures
    const cacheKey = `${content}|${fontSize}|${fillColor}|${fontWeight}|${fontFamily}|${textAlign}|${Math.round(renderW)}|${Math.round(textH)}|${scale}`

    let img = this.textCache.get(cacheKey)
    if (img === undefined) {
      let effectiveScale = scale
      let cw = Math.ceil(renderW * effectiveScale)
      let ch = Math.ceil(textH * effectiveScale)
      if (cw <= 0 || ch <= 0) { this.textCache.set(cacheKey, null); return }
      // Cap texture dimensions to avoid exceeding browser canvas limits
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
      // Premultiply alpha for correct CanvasKit texture blending.
      // Canvas 2D getImageData returns unpremultiplied RGBA, but CanvasKit's
      // WebGL backend handles Premul textures more reliably than Unpremul,
      // avoiding gray-background artifacts on transparent text images.
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
        // a === 0 → all zeros (already initialized)
      }
      img = ck.MakeImage(
        { width: cw, height: ch, alphaType: ck.AlphaType.Premul, colorType: ck.ColorType.RGBA_8888, colorSpace: ck.ColorSpace.SRGB },
        premul, cw * 4,
      ) ?? null

      this.textCache.set(cacheKey, img)
      this.textCacheOrder.push(cacheKey)
      this.evictTextCache()
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

  private drawImage(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
  ) {
    const ck = this.ck
    const iNode = node as ImageNode
    const src: string | undefined = iNode.src
    const cr = cornerRadiusValue(iNode.cornerRadius)

    if (!src) {
      this.drawImageFallback(canvas, x, y, w, h, cr, opacity)
      return
    }

    // Check cache / start loading
    const cached = this.imageLoader.get(src)
    if (cached === undefined) {
      // Not yet requested — start loading
      this.imageLoader.request(src)
      this.drawImageFallback(canvas, x, y, w, h, cr, opacity)
      return
    }
    if (!cached) {
      // Still loading or failed
      this.drawImageFallback(canvas, x, y, w, h, cr, opacity)
      return
    }

    // Draw loaded image with objectFit and optional corner radius clipping
    const imgW = cached.width()
    const imgH = cached.height()

    // Clip for corner radius
    if (cr > 0) {
      canvas.save()
      const maxR = Math.min(cr, w / 2, h / 2)
      canvas.clipRRect(
        ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), maxR, maxR),
        ck.ClipOp.Intersect, true,
      )
    } else {
      canvas.save()
      canvas.clipRect(ck.LTRBRect(x, y, x + w, y + h), ck.ClipOp.Intersect, true)
    }

    const paint = new ck.Paint()
    paint.setAntiAlias(true)
    if (opacity < 1) paint.setAlphaf(opacity)

    // Apply image adjustments if any
    const adjFilter = this.buildImageAdjustmentFilter(iNode)
    if (adjFilter) paint.setColorFilter(adjFilter)

    const fit = iNode.objectFit ?? 'fill'

    if (fit === 'tile') {
      // Tile: repeat image at its original pixel size
      const tileMatrix = Float32Array.of(1, 0, -x, 0, 1, -y, 0, 0, 1)
      const shader = cached.makeShaderOptions(
        ck.TileMode.Repeat, ck.TileMode.Repeat,
        ck.FilterMode.Linear, ck.MipmapMode.None,
        tileMatrix,
      )
      if (shader) {
        paint.setShader(shader)
        canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), paint)
      }
    } else if (fit === 'fit') {
      // Fit (contain): scale uniformly so entire image is visible, centered
      // Draw a subtle background so letterbox areas are visible
      const bgPaint = new ck.Paint()
      bgPaint.setStyle(ck.PaintStyle.Fill)
      bgPaint.setColor(parseColor(ck, '#f3f4f6'))
      if (opacity < 1) bgPaint.setAlphaf(opacity * 0.3)
      else bgPaint.setAlphaf(0.3)
      canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), bgPaint)
      bgPaint.delete()

      const scale = Math.min(w / imgW, h / imgH)
      const dw = imgW * scale
      const dh = imgH * scale
      const dx = x + (w - dw) / 2
      const dy = y + (h - dh) / 2
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      )
    } else {
      // 'fill' and 'crop' (cover): scale uniformly to fill entire area, centered, excess clipped
      const scale = Math.max(w / imgW, h / imgH)
      const dw = imgW * scale
      const dh = imgH * scale
      const dx = x + (w - dw) / 2
      const dy = y + (h - dh) / 2
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      )
    }

    paint.delete()
    canvas.restore()
  }

  private drawImageFallback(
    canvas: Canvas,
    x: number, y: number, w: number, h: number,
    cr: number, opacity: number,
  ) {
    const ck = this.ck
    const paint = new ck.Paint()
    paint.setStyle(ck.PaintStyle.Fill)
    paint.setAntiAlias(true)
    const c = parseColor(ck, '#e5e7eb')
    c[3] *= opacity
    paint.setColor(c)

    if (cr > 0) {
      const maxR = Math.min(cr, w / 2, h / 2)
      canvas.drawRRect(ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), maxR, maxR), paint)
    } else {
      canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), paint)
    }
    paint.delete()
  }

  // Drawing preview (semi-transparent shape while user drags to create)
  drawPreview(
    canvas: Canvas,
    shape: { type: string; x: number; y: number; w: number; h: number },
  ) {
    const ck = this.ck
    const fillPaint = new ck.Paint()
    fillPaint.setStyle(ck.PaintStyle.Fill)
    fillPaint.setAntiAlias(true)
    fillPaint.setColor(parseColor(ck, 'rgba(59, 130, 246, 0.1)'))

    const strokePaint = new ck.Paint()
    strokePaint.setStyle(ck.PaintStyle.Stroke)
    strokePaint.setAntiAlias(true)
    strokePaint.setStrokeWidth(1)
    strokePaint.setColor(parseColor(ck, '#3b82f6'))

    const { x, y, w, h } = shape
    if (shape.type === 'line') {
      canvas.drawLine(x, y, x + w, y + h, strokePaint)
    } else if (shape.type === 'ellipse') {
      canvas.drawOval(ck.LTRBRect(x, y, x + w, y + h), fillPaint)
      canvas.drawOval(ck.LTRBRect(x, y, x + w, y + h), strokePaint)
    } else {
      // rectangle / frame
      canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), fillPaint)
      canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), strokePaint)
    }

    fillPaint.delete()
    strokePaint.delete()
  }

  // Overlay drawing (delegated to skia-overlays.ts)

  drawSelectionBorder(canvas: Canvas, x: number, y: number, w: number, h: number) {
    _drawSelectionBorder(this.ck, canvas, x, y, w, h)
  }

  drawFrameLabel(canvas: Canvas, name: string, x: number, y: number) {
    _drawFrameLabel(this.ck, canvas, name, x, y)
  }

  drawHoverOutline(canvas: Canvas, x: number, y: number, w: number, h: number) {
    _drawHoverOutline(this.ck, canvas, x, y, w, h)
  }

  drawSelectionMarquee(canvas: Canvas, x1: number, y1: number, x2: number, y2: number) {
    _drawSelectionMarquee(this.ck, canvas, x1, y1, x2, y2)
  }

  drawGuide(canvas: Canvas, x1: number, y1: number, x2: number, y2: number, zoom: number) {
    _drawGuide(this.ck, canvas, x1, y1, x2, y2, zoom)
  }

  drawPenPreview(canvas: Canvas, data: PenPreviewData, zoom: number) {
    _drawPenPreview(this.ck, canvas, data, zoom)
  }

  drawFrameLabelColored(
    canvas: Canvas, name: string, x: number, y: number,
    isReusable: boolean, isInstance: boolean, zoom = 1,
  ) {
    _drawFrameLabelColored(this.ck, canvas, name, x, y, isReusable, isInstance, zoom)
  }

  drawAgentGlow(
    canvas: Canvas, x: number, y: number, w: number, h: number,
    color: string, breath: number, zoom: number,
  ) {
    _drawAgentGlow(this.ck, canvas, x, y, w, h, color, breath, zoom)
  }

  drawAgentBadge(
    canvas: Canvas, name: string,
    frameX: number, frameY: number, frameW: number,
    color: string, zoom: number, time: number,
  ) {
    _drawAgentBadge(this.ck, canvas, name, frameX, frameY, frameW, color, zoom, time)
  }

  drawAgentNodeBorder(
    canvas: Canvas, x: number, y: number, w: number, h: number,
    color: string, breath: number, zoom: number,
  ) {
    _drawAgentNodeBorder(this.ck, canvas, x, y, w, h, color, breath, zoom)
  }

  drawAgentPreviewFill(
    canvas: Canvas, x: number, y: number, w: number, h: number,
    color: string, time: number,
  ) {
    _drawAgentPreviewFill(this.ck, canvas, x, y, w, h, color, time)
  }
}
