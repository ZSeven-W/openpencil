import type { CanvasKit, Canvas, Paint, Font, Typeface, Image as SkImage } from 'canvaskit-wasm'
import type { PenNode } from '@/types/pen'
import type { PenFill, PenStroke, PenEffect, ShadowEffect } from '@/types/styles'
import { DEFAULT_FILL, DEFAULT_STROKE, DEFAULT_STROKE_WIDTH } from '../canvas-constants'
import { defaultLineHeight } from '../canvas-text-measure'
import { lookupIconByName } from '@/services/ai/icon-resolver'
import { buildEllipseArcPath, isArcEllipse } from '@/utils/arc-path'
import { SkiaImageLoader } from './skia-image-loader'
import {
  parseColor,
  cornerRadiusValue,
  cornerRadii,
  resolveFillColor,
  resolveStrokeColor,
  resolveStrokeWidth,
  wrapLine,
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

  // Image loader
  imageLoader: SkiaImageLoader

  constructor(ck: CanvasKit) {
    this.ck = ck
    this.imageLoader = new SkiaImageLoader(ck)
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
    this.imageLoader.dispose()
  }

  clearTextCache() {
    for (const img of this.textCache.values()) {
      img?.delete()
    }
    this.textCache.clear()
    this.textCacheOrder = []
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
  ): Paint {
    const ck = this.ck
    const paint = new ck.Paint()
    paint.setStyle(ck.PaintStyle.Fill)
    paint.setAntiAlias(true)

    if (typeof fills === 'string') {
      const c = parseColor(ck, fills)
      c[3] *= opacity
      paint.setColor(c)
      return paint
    }
    if (!fills || fills.length === 0) {
      const c = parseColor(ck, DEFAULT_FILL)
      c[3] *= opacity
      paint.setColor(c)
      return paint
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
    }

    return paint
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
    const flipX = ('flipX' in node && (node as any).flipX) === true
    const flipY = ('flipY' in node && (node as any).flipY) === true
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

    // Apply shadow
    const effects = 'effects' in node ? node.effects : undefined
    this.applyShadowDirect(canvas, effects, absX, absY, absW, absH)

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
        this.drawText(canvas, node, absX, absY, absW, absH, opacity)
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
    const cr = cornerRadii('cornerRadius' in node ? (node as any).cornerRadius : undefined)
    const fills = 'fill' in node ? (node as any).fill : undefined
    const stroke = 'stroke' in node ? (node as any).stroke : undefined

    // For frames/groups without explicit fill, use transparent (no visible background)
    const hasFill = typeof fills === 'string' ? fills.length > 0 : (fills && fills.length > 0)
    const isContainer = node.type === 'frame' || node.type === 'group'

    // Fill
    const fillPaint = this.makeFillPaint(
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
    const eNode = node as any
    const fills = eNode.fill
    const stroke = eNode.stroke

    if (isArcEllipse(eNode.startAngle, eNode.sweepAngle, eNode.innerRadius)) {
      const arcD = buildEllipseArcPath(w, h, eNode.startAngle ?? 0, eNode.sweepAngle ?? 360, eNode.innerRadius ?? 0)
      const path = ck.Path.MakeFromSVGString(arcD)
      if (path) {
        path.offset(x, y)
        const fillPaint = this.makeFillPaint(fills, w, h, opacity, x, y)
        fillPaint.setAntiAlias(true)
        canvas.drawPath(path, fillPaint)
        fillPaint.delete()
        path.delete()
      }
      return
    }

    const fillPaint = this.makeFillPaint(fills, w, h, opacity, x, y)
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
    const lNode = node as any
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
    const pNode = node as any
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

    const fillPaint = this.makeFillPaint(fills, w, h, opacity, x, y)
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
    const pNode = node as any
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
        const fillPaint = this.makeFillPaint(fills, w, h, opacity, x, y)
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
      const isIcon = !!(pNode.iconId)
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
      const fillPaint = this.makeFillPaint(
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
    const iNode = node as any
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
   * Render text using browser Canvas 2D API (supports all system fonts including CJK),
   * then draw the rasterized result as a CanvasKit image. Results are cached.
   */
  private drawText(
    canvas: Canvas, node: PenNode,
    x: number, y: number, w: number, h: number,
    opacity: number,
  ) {
    const ck = this.ck
    const tNode = node as any
    const content = typeof tNode.content === 'string'
      ? tNode.content
      : Array.isArray(tNode.content)
        ? tNode.content.map((s: any) => s.text ?? '').join('')
        : ''

    if (!content) return

    const fontSize = tNode.fontSize ?? 16
    const fillColor = resolveFillColor(tNode.fill)
    const fontWeight = tNode.fontWeight ?? '400'
    const fontFamily = tNode.fontFamily ?? 'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif'
    const textAlign: string = tNode.textAlign ?? 'left'
    const lineHeightMul = tNode.lineHeight ?? defaultLineHeight(fontSize)
    const lineHeight = lineHeightMul * fontSize
    const textGrowth: string | undefined = tNode.textGrowth

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
    mCtx.font = `${fontWeight} ${fontSize}px ${fontFamily}`

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

    // Cache key
    const cacheKey = `${content}|${fontSize}|${fillColor}|${fontWeight}|${textAlign}|${Math.round(renderW)}|${Math.round(textH)}`

    let img = this.textCache.get(cacheKey)
    if (img === undefined) {
      const scale = 2
      const cw = Math.ceil(renderW * scale)
      const ch = Math.ceil(textH * scale)
      if (cw <= 0 || ch <= 0) { this.textCache.set(cacheKey, null); return }

      const tmp = document.createElement('canvas')
      tmp.width = cw
      tmp.height = ch
      const ctx = tmp.getContext('2d')!
      ctx.scale(scale, scale)
      ctx.font = `${fontWeight} ${fontSize}px ${fontFamily}`
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
    const iNode = node as any
    const src: string | undefined = iNode.src
    const cr = cornerRadiusValue('cornerRadius' in node ? (node as any).cornerRadius : undefined)

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

    // Draw loaded image with optional corner radius clipping
    if (cr > 0) {
      canvas.save()
      const maxR = Math.min(cr, w / 2, h / 2)
      canvas.clipRRect(
        ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), maxR, maxR),
        ck.ClipOp.Intersect, true,
      )
    }
    const paint = new ck.Paint()
    paint.setAntiAlias(true)
    if (opacity < 1) paint.setAlphaf(opacity)
    canvas.drawImageRect(
      cached,
      ck.LTRBRect(0, 0, cached.width(), cached.height()),
      ck.LTRBRect(x, y, x + w, y + h),
      paint,
    )
    paint.delete()
    if (cr > 0) canvas.restore()
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
