import type { CanvasKit, Canvas, Paint, Font, Typeface } from 'canvaskit-wasm';
import type {
  PenNode,
  ContainerProps,
  EllipseNode,
  LineNode,
  PolygonNode,
  PathNode,
  ImageNode,
  IconFontNode,
} from '@zseven-w/pen-types';
import type { PenFill, PenStroke, PenEffect, ShadowEffect, ImageFill } from '@zseven-w/pen-types';
import {
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
  buildEllipseArcPath,
  getPathBoundsFromAnchors,
  isArcEllipse,
  pathDataToAnchors,
} from '@zseven-w/pen-core';
import {
  parseColor,
  cornerRadiusValue,
  cornerRadii,
  resolveFillColor,
  shouldUseTransparentFallbackFill,
  resolveStrokeColor,
  resolveStrokeWidth,
} from './paint-utils.js';
import { sanitizeSvgPath, hasInvalidNumbers, tryManualPathParse } from './path-utils.js';
import { SkiaImageLoader } from './image-loader.js';
import { SkiaTextRenderer } from './text-renderer.js';
import type { SkiaFontManager, FontManagerOptions } from './font-manager.js';
import type { RenderNode, IconLookupFn } from './types.js';

const FALLBACK_ICON_D = 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0';

/**
 * Core node renderer for CanvasKit/Skia. Draws PenNode shapes, fills,
 * strokes, effects, text, and images. No editor overlays or store dependencies.
 */
export class SkiaNodeRenderer {
  protected ck: CanvasKit;
  private defaultTypeface: Typeface | null = null;
  private defaultFont: Font | null = null;

  // Current viewport zoom (set by engine before each render frame)
  zoom = 1;

  // Device pixel ratio
  devicePixelRatio: number | undefined;

  // Sub-renderers
  private textRenderer: SkiaTextRenderer;
  imageLoader: SkiaImageLoader;

  // Injectable icon lookup
  private iconLookup: IconLookupFn | null = null;

  /** Font manager — delegates to text renderer */
  get fontManager(): SkiaFontManager {
    return this.textRenderer.fontManager;
  }

  constructor(ck: CanvasKit, fontOptions?: FontManagerOptions) {
    this.ck = ck;
    this.imageLoader = new SkiaImageLoader(ck);
    this.textRenderer = new SkiaTextRenderer(ck, fontOptions);
  }

  init() {
    this.defaultFont = new this.ck.Font(null, 16);
  }

  /** Set callback to trigger re-render when async images finish loading. */
  setRedrawCallback(cb: () => void) {
    this.imageLoader.setOnLoaded(cb);
  }

  /** Set injectable icon lookup function. */
  setIconLookup(fn: IconLookupFn) {
    this.iconLookup = fn;
  }

  setImageSourceResolver(resolver: (src: string) => { cacheKey: string; loadUrl: string | null }) {
    this.imageLoader.setSourceResolver(resolver);
  }

  dispose() {
    this.defaultFont?.delete();
    this.defaultFont = null;
    this.defaultTypeface?.delete();
    this.defaultTypeface = null;
    this.textRenderer.dispose();
    this.imageLoader.dispose();
  }

  clearTextCache() {
    this.textRenderer.clearTextCache();
  }
  clearParaCache() {
    this.textRenderer.clearParaCache();
  }

  // ---------------------------------------------------------------------------
  // Fill paint
  // ---------------------------------------------------------------------------

  private makeFillPaint(
    fills: PenFill[] | string | undefined,
    w: number,
    h: number,
    opacity: number,
    absX: number,
    absY: number,
  ): {
    paint: Paint;
    imageFillDraw?: {
      fill: ImageFill;
      w: number;
      h: number;
      absX: number;
      absY: number;
      opacity: number;
    };
  } {
    const ck = this.ck;
    const paint = new ck.Paint();
    paint.setStyle(ck.PaintStyle.Fill);
    paint.setAntiAlias(true);

    if (typeof fills === 'string') {
      const c = parseColor(ck, fills);
      c[3] *= opacity;
      paint.setColor(c);
      return { paint };
    }
    if (!fills || fills.length === 0) {
      const c = parseColor(ck, DEFAULT_FILL);
      c[3] *= opacity;
      paint.setColor(c);
      return { paint };
    }

    const first = fills[0];
    if (first.type === 'solid') {
      const c = parseColor(ck, first.color);
      c[3] *= (first.opacity ?? 1) * opacity;
      paint.setColor(c);
    } else if (first.type === 'linear_gradient') {
      const stops = first.stops ?? [];
      const fillOpacity = (first.opacity ?? 1) * opacity;
      if (stops.length >= 2) {
        const rad = (((first.angle ?? 0) - 90) * Math.PI) / 180;
        const cos = Math.cos(rad),
          sin = Math.sin(rad);
        const x1 = absX + w / 2 - (cos * w) / 2,
          y1 = absY + h / 2 - (sin * h) / 2;
        const x2 = absX + w / 2 + (cos * w) / 2,
          y2 = absY + h / 2 + (sin * h) / 2;
        const colors = stops.map((s) => {
          const c = parseColor(ck, s.color);
          c[3] *= fillOpacity;
          return c;
        });
        const positions = stops.map((s) => Math.max(0, Math.min(1, s.offset)));
        const shader = ck.Shader.MakeLinearGradient(
          [x1, y1],
          [x2, y2],
          colors,
          positions,
          ck.TileMode.Clamp,
        );
        if (shader) paint.setShader(shader);
      } else {
        const c = parseColor(ck, stops[0]?.color ?? DEFAULT_FILL);
        c[3] *= fillOpacity;
        paint.setColor(c);
      }
    } else if (first.type === 'radial_gradient') {
      const stops = first.stops ?? [];
      const fillOpacity = (first.opacity ?? 1) * opacity;
      if (stops.length >= 2) {
        const cx = absX + (first.cx ?? 0.5) * w,
          cy = absY + (first.cy ?? 0.5) * h;
        const r = (first.radius ?? 0.5) * Math.max(w, h);
        const colors = stops.map((s) => {
          const c = parseColor(ck, s.color);
          c[3] *= fillOpacity;
          return c;
        });
        const positions = stops.map((s) => Math.max(0, Math.min(1, s.offset)));
        const shader = ck.Shader.MakeRadialGradient(
          [cx, cy],
          r,
          colors,
          positions,
          ck.TileMode.Clamp,
        );
        if (shader) paint.setShader(shader);
      } else {
        const c = parseColor(ck, stops[0]?.color ?? DEFAULT_FILL);
        c[3] *= fillOpacity;
        paint.setColor(c);
      }
    } else if (first.type === 'image') {
      const result = this.applyImageFillToPaint(paint, first, w, h, opacity, absX, absY);
      if (result.needsDrawImageRect && result.fill) {
        return {
          paint,
          imageFillDraw: {
            fill: result.fill,
            w: result.w!,
            h: result.h!,
            absX: result.absX!,
            absY: result.absY!,
            opacity: result.opacity!,
          },
        };
      }
    }

    return { paint };
  }

  private applyImageFillToPaint(
    paint: Paint,
    fill: ImageFill,
    w: number,
    h: number,
    opacity: number,
    absX: number,
    absY: number,
  ): {
    needsDrawImageRect: boolean;
    fill?: ImageFill;
    w?: number;
    h?: number;
    absX?: number;
    absY?: number;
    opacity?: number;
  } {
    const ck = this.ck;
    const fillOpacity = (fill.opacity ?? 1) * opacity;
    const url = fill.url;
    if (!url) {
      const c = parseColor(ck, '#e5e7eb');
      c[3] *= fillOpacity;
      paint.setColor(c);
      return { needsDrawImageRect: false };
    }

    const cached = this.imageLoader.get(url);
    if (cached === undefined) this.imageLoader.request(url);
    if (!cached) {
      const isMissing = this.imageLoader.getStatus(url)?.state === 'missing';
      const c = parseColor(ck, isMissing ? '#f1d7d7' : '#e5e7eb');
      c[3] *= fillOpacity;
      paint.setColor(c);
      return { needsDrawImageRect: false };
    }

    const imgW = cached.width(),
      imgH = cached.height();
    if (imgW <= 0 || imgH <= 0) return { needsDrawImageRect: false };

    const mode = fill.mode ?? 'fill';
    if (mode === 'tile') {
      const dispX = absX + (w - imgW) / 2,
        dispY = absY + (h - imgH) / 2;
      const localMatrix = Float32Array.of(1, 0, -dispX, 0, 1, -dispY, 0, 0, 1);
      const shader = cached.makeShaderOptions(
        ck.TileMode.Repeat,
        ck.TileMode.Repeat,
        ck.FilterMode.Linear,
        ck.MipmapMode.None,
        localMatrix,
      );
      if (shader) {
        paint.setShader(shader);
        if (fillOpacity < 1) paint.setAlphaf(fillOpacity);
        const cf = this.buildImageAdjustmentFilter(fill);
        if (cf) paint.setColorFilter(cf);
      }
      return { needsDrawImageRect: false };
    }

    paint.setColor(Float32Array.of(0, 0, 0, 0));
    return { needsDrawImageRect: true, fill, w, h, absX, absY, opacity: fillOpacity };
  }

  private drawImageFillRect(
    canvas: Canvas,
    fill: ImageFill,
    w: number,
    h: number,
    absX: number,
    absY: number,
    fillOpacity: number,
  ) {
    const ck = this.ck;
    const url = fill.url;
    if (!url) return;
    const cached = this.imageLoader.get(url);
    if (!cached) return;
    const imgW = cached.width(),
      imgH = cached.height();
    if (imgW <= 0 || imgH <= 0) return;

    const mode = fill.mode ?? 'fill';
    const paint = new ck.Paint();
    paint.setAntiAlias(true);
    if (fillOpacity < 1) paint.setAlphaf(fillOpacity);
    const adjFilter = this.buildImageAdjustmentFilter(fill);
    if (adjFilter) paint.setColorFilter(adjFilter);

    if (mode === 'fit') {
      const scale = Math.min(w / imgW, h / imgH);
      const dw = imgW * scale,
        dh = imgH * scale;
      const dx = absX + (w - dw) / 2,
        dy = absY + (h - dh) / 2;
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      );
    } else if (mode === 'stretch') {
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(absX, absY, absX + w, absY + h),
        paint,
      );
    } else {
      const scale = Math.max(w / imgW, h / imgH);
      const dw = imgW * scale,
        dh = imgH * scale;
      const dx = absX + (w - dw) / 2,
        dy = absY + (h - dh) / 2;
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      );
    }
    paint.delete();
  }

  private buildImageAdjustmentFilter(adj: {
    exposure?: number;
    contrast?: number;
    saturation?: number;
    temperature?: number;
    tint?: number;
    highlights?: number;
    shadows?: number;
  }) {
    const ck = this.ck;
    const exp = (adj.exposure ?? 0) / 100,
      con = (adj.contrast ?? 0) / 100,
      sat = (adj.saturation ?? 0) / 100;
    const temp = (adj.temperature ?? 0) / 100,
      tintVal = (adj.tint ?? 0) / 100;
    const hi = (adj.highlights ?? 0) / 100,
      sh = (adj.shadows ?? 0) / 100;
    if (exp === 0 && con === 0 && sat === 0 && temp === 0 && tintVal === 0 && hi === 0 && sh === 0)
      return null;

    const e = 1 + exp * 1.5,
      c = 1 + con,
      cOff = 0.5 * (1 - c);
    const s = 1 + sat;
    const lr = 0.2126,
      lg = 0.7152,
      lb = 0.0722;
    const sr = (1 - s) * lr,
      sg = (1 - s) * lg,
      sb = (1 - s) * lb;
    const f = c * e;
    const offR = cOff + temp * 0.15 + (hi + sh * 0.5) * 0.1;
    const offG = cOff + tintVal * 0.15 + (hi + sh * 0.5) * 0.1;
    const offB = cOff - temp * 0.15 + (hi + sh * 0.5) * 0.1;

    return ck.ColorFilter.MakeMatrix([
      f * (sr + s),
      f * sg,
      f * sb,
      0,
      offR,
      f * sr,
      f * (sg + s),
      f * sb,
      0,
      offG,
      f * sr,
      f * sg,
      f * (sb + s),
      0,
      offB,
      0,
      0,
      0,
      1,
      0,
    ]);
  }

  // ---------------------------------------------------------------------------
  // Stroke paint
  // ---------------------------------------------------------------------------

  private makeStrokePaint(stroke: PenStroke | undefined, opacity: number): Paint | null {
    if (!stroke) return null;
    const strokeColor = resolveStrokeColor(stroke);
    const strokeWidth = resolveStrokeWidth(stroke);
    if (!strokeColor || strokeWidth <= 0) return null;

    const ck = this.ck;
    const paint = new ck.Paint();
    paint.setStyle(ck.PaintStyle.Stroke);
    paint.setAntiAlias(true);
    paint.setStrokeWidth(strokeWidth);
    const c = parseColor(ck, strokeColor);
    c[3] *= opacity;
    paint.setColor(c);

    if (stroke.join === 'round') paint.setStrokeJoin(ck.StrokeJoin.Round);
    else if (stroke.join === 'bevel') paint.setStrokeJoin(ck.StrokeJoin.Bevel);
    if (stroke.cap === 'round') paint.setStrokeCap(ck.StrokeCap.Round);
    else if (stroke.cap === 'square') paint.setStrokeCap(ck.StrokeCap.Square);
    if (stroke.dashPattern && stroke.dashPattern.length >= 2) {
      const effect = ck.PathEffect.MakeDash(stroke.dashPattern, stroke.dashOffset ?? 0);
      if (effect) paint.setPathEffect(effect);
    }

    return paint;
  }

  // ---------------------------------------------------------------------------
  // Shadow
  // ---------------------------------------------------------------------------

  private applyShadowDirect(
    canvas: Canvas,
    effects: PenEffect[] | undefined,
    x: number,
    y: number,
    w: number,
    h: number,
  ): boolean {
    if (!effects) return false;
    const shadow = effects.find((e): e is ShadowEffect => e.type === 'shadow');
    if (!shadow) return false;

    const ck = this.ck;
    const paint = new ck.Paint();
    paint.setStyle(ck.PaintStyle.Fill);
    paint.setAntiAlias(true);
    paint.setColor(parseColor(ck, shadow.color));
    paint.setMaskFilter(ck.MaskFilter.MakeBlur(ck.BlurStyle.Normal, shadow.blur / 2, true));
    canvas.drawRect(
      ck.LTRBRect(
        x + shadow.offsetX - shadow.spread,
        y + shadow.offsetY - shadow.spread,
        x + w + shadow.offsetX + shadow.spread,
        y + h + shadow.offsetY + shadow.spread,
      ),
      paint,
    );
    paint.delete();
    return true;
  }

  // ---------------------------------------------------------------------------
  // Draw a single render node (no selection/overlay logic)
  // ---------------------------------------------------------------------------

  drawNode(canvas: Canvas, rn: RenderNode) {
    const { node, absX, absY, absW, absH, clipRect } = rn;
    const ck = this.ck;
    const opacity = typeof node.opacity === 'number' ? node.opacity : 1;

    if (('visible' in node ? node.visible : undefined) === false) return;

    // Pass zoom to text renderer
    this.textRenderer.zoom = this.zoom;
    this.textRenderer.devicePixelRatio = this.devicePixelRatio;

    // Apply clipping from parent frame
    let clipped = false;
    if (clipRect) {
      canvas.save();
      clipped = true;
      if (clipRect.rx > 0) {
        canvas.clipRRect(
          ck.RRectXY(
            ck.LTRBRect(clipRect.x, clipRect.y, clipRect.x + clipRect.w, clipRect.y + clipRect.h),
            clipRect.rx,
            clipRect.rx,
          ),
          ck.ClipOp.Intersect,
          true,
        );
      } else {
        canvas.clipRect(
          ck.LTRBRect(clipRect.x, clipRect.y, clipRect.x + clipRect.w, clipRect.y + clipRect.h),
          ck.ClipOp.Intersect,
          true,
        );
      }
    }

    // Apply flip
    const flipX = node.flipX === true,
      flipY = node.flipY === true;
    if (flipX || flipY) {
      canvas.save();
      canvas.translate(absX + absW / 2, absY + absH / 2);
      canvas.scale(flipX ? -1 : 1, flipY ? -1 : 1);
      canvas.translate(-(absX + absW / 2), -(absY + absH / 2));
    }

    // Apply rotation
    const rotation = node.rotation ?? 0;
    if (rotation !== 0) {
      canvas.save();
      canvas.rotate(rotation, absX + absW / 2, absY + absH / 2);
    }

    // Apply shadow (text uses glyph-shaped shadow, not rectangle)
    const effects =
      'effects' in node ? (node as PenNode & { effects?: PenEffect[] }).effects : undefined;
    if (node.type !== 'text') {
      this.applyShadowDirect(canvas, effects, absX, absY, absW, absH);
    }

    switch (node.type) {
      case 'frame':
      case 'rectangle':
      case 'group':
        this.drawRect(canvas, node, absX, absY, absW, absH, opacity);
        break;
      case 'ellipse':
        this.drawEllipse(canvas, node, absX, absY, absW, absH, opacity);
        break;
      case 'line':
        this.drawLine(canvas, node, absX, absY, opacity);
        break;
      case 'polygon':
        this.drawPolygon(canvas, node, absX, absY, absW, absH, opacity);
        break;
      case 'path':
        this.drawPath(canvas, node, absX, absY, absW, absH, opacity);
        break;
      case 'icon_font':
        this.drawIconFont(canvas, node, absX, absY, absW, absH, opacity);
        break;
      case 'text':
        this.textRenderer.drawText(canvas, node, absX, absY, absW, absH, opacity, effects);
        break;
      case 'image':
        this.drawImage(canvas, node, absX, absY, absW, absH, opacity);
        break;
    }

    if (rotation !== 0) canvas.restore();
    if (flipX || flipY) canvas.restore();
    if (clipped) canvas.restore();
  }

  // ---------------------------------------------------------------------------
  // Shape drawing
  // ---------------------------------------------------------------------------

  private drawRect(
    canvas: Canvas,
    node: PenNode,
    x: number,
    y: number,
    w: number,
    h: number,
    opacity: number,
  ) {
    const ck = this.ck;
    const container = node as PenNode & ContainerProps;
    const cr = cornerRadii(container.cornerRadius);
    const fills = container.fill;
    const stroke = container.stroke;
    const hasFill = fills && fills.length > 0;
    const isContainer = node.type === 'frame' || node.type === 'group';

    const { paint: fillPaint, imageFillDraw } = this.makeFillPaint(
      hasFill || !shouldUseTransparentFallbackFill(fills, stroke, isContainer)
        ? fills
        : 'transparent',
      w,
      h,
      opacity,
      x,
      y,
    );
    const hasRoundedCorners = cr.some((r) => r > 0);
    if (hasRoundedCorners) {
      const maxR = Math.min(w / 2, h / 2);
      canvas.drawRRect(
        ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), Math.min(cr[0], maxR), Math.min(cr[0], maxR)),
        fillPaint,
      );
    } else {
      canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), fillPaint);
    }
    fillPaint.delete();

    if (imageFillDraw) {
      canvas.save();
      if (hasRoundedCorners) {
        const maxR = Math.min(w / 2, h / 2);
        canvas.clipRRect(
          ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), Math.min(cr[0], maxR), Math.min(cr[0], maxR)),
          ck.ClipOp.Intersect,
          true,
        );
      } else {
        canvas.clipRect(ck.LTRBRect(x, y, x + w, y + h), ck.ClipOp.Intersect, true);
      }
      this.drawImageFillRect(
        canvas,
        imageFillDraw.fill,
        imageFillDraw.w,
        imageFillDraw.h,
        imageFillDraw.absX,
        imageFillDraw.absY,
        imageFillDraw.opacity,
      );
      canvas.restore();
    }

    const strokePaint = this.makeStrokePaint(stroke, opacity);
    if (strokePaint) {
      if (hasRoundedCorners) {
        const maxR = Math.min(w / 2, h / 2);
        canvas.drawRRect(
          ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), Math.min(cr[0], maxR), Math.min(cr[0], maxR)),
          strokePaint,
        );
      } else {
        canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), strokePaint);
      }
      strokePaint.delete();
    }
  }

  private drawEllipse(
    canvas: Canvas,
    node: PenNode,
    x: number,
    y: number,
    w: number,
    h: number,
    opacity: number,
  ) {
    const ck = this.ck;
    const eNode = node as EllipseNode;
    const fills = eNode.fill,
      stroke = eNode.stroke;
    const cr = cornerRadiusValue(eNode.cornerRadius);
    const fillSource = shouldUseTransparentFallbackFill(fills, stroke) ? 'transparent' : fills;

    if (isArcEllipse(eNode.startAngle, eNode.sweepAngle, eNode.innerRadius)) {
      const arcD = buildEllipseArcPath(
        w,
        h,
        eNode.startAngle ?? 0,
        eNode.sweepAngle ?? 360,
        eNode.innerRadius ?? 0,
      );
      const path = ck.Path.MakeFromSVGString(arcD);
      if (path) {
        path.offset(x, y);
        const { paint: fillPaint } = this.makeFillPaint(fillSource, w, h, opacity, x, y);
        if (cr > 0) {
          const effect = ck.PathEffect.MakeCorner(cr);
          if (effect) fillPaint.setPathEffect(effect);
        }
        canvas.drawPath(path, fillPaint);
        fillPaint.delete();
        const strokePaint = this.makeStrokePaint(stroke, opacity);
        if (strokePaint) {
          if (cr > 0) {
            const effect = ck.PathEffect.MakeCorner(cr);
            if (effect) strokePaint.setPathEffect(effect);
          }
          canvas.drawPath(path, strokePaint);
          strokePaint.delete();
        }
        path.delete();
      }
      return;
    }

    const { paint: fillPaint } = this.makeFillPaint(fillSource, w, h, opacity, x, y);
    canvas.drawOval(ck.LTRBRect(x, y, x + w, y + h), fillPaint);
    fillPaint.delete();
    const strokePaint = this.makeStrokePaint(stroke, opacity);
    if (strokePaint) {
      canvas.drawOval(ck.LTRBRect(x, y, x + w, y + h), strokePaint);
      strokePaint.delete();
    }
  }

  private drawLine(canvas: Canvas, node: PenNode, x: number, y: number, opacity: number) {
    const ck = this.ck;
    const lNode = node as LineNode;
    const x2 = lNode.x2 ?? x + 100,
      y2 = lNode.y2 ?? y;
    const strokeColor = resolveStrokeColor(lNode.stroke) ?? DEFAULT_STROKE;
    const strokeWidth = resolveStrokeWidth(lNode.stroke) || DEFAULT_STROKE_WIDTH;
    const paint = new ck.Paint();
    paint.setStyle(ck.PaintStyle.Stroke);
    paint.setAntiAlias(true);
    paint.setStrokeWidth(strokeWidth);
    const c = parseColor(ck, strokeColor);
    c[3] *= opacity;
    paint.setColor(c);
    canvas.drawLine(x, y, x2, y2, paint);
    paint.delete();
  }

  private drawPolygon(
    canvas: Canvas,
    node: PenNode,
    x: number,
    y: number,
    w: number,
    h: number,
    opacity: number,
  ) {
    const ck = this.ck;
    const pNode = node as PolygonNode;
    const count = pNode.polygonCount || 6;
    const fills = pNode.fill,
      stroke = pNode.stroke;
    const cr = cornerRadiusValue(pNode.cornerRadius);
    const fillSource = shouldUseTransparentFallbackFill(fills, stroke) ? 'transparent' : fills;

    const raw: [number, number][] = [];
    for (let i = 0; i < count; i++) {
      const angle = (i * 2 * Math.PI) / count - Math.PI / 2;
      raw.push([Math.cos(angle), Math.sin(angle)]);
    }
    let minX = Infinity,
      maxX = -Infinity,
      minY = Infinity,
      maxY = -Infinity;
    for (const [rx, ry] of raw) {
      if (rx < minX) minX = rx;
      if (rx > maxX) maxX = rx;
      if (ry < minY) minY = ry;
      if (ry > maxY) maxY = ry;
    }
    const rawW = maxX - minX,
      rawH = maxY - minY;

    const path = new ck.Path();
    for (let i = 0; i < count; i++) {
      const px = x + ((raw[i][0] - minX) / rawW) * w,
        py = y + ((raw[i][1] - minY) / rawH) * h;
      if (i === 0) path.moveTo(px, py);
      else path.lineTo(px, py);
    }
    path.close();

    const { paint: fillPaint } = this.makeFillPaint(fillSource, w, h, opacity, x, y);
    if (cr > 0) {
      const effect = ck.PathEffect.MakeCorner(cr);
      if (effect) fillPaint.setPathEffect(effect);
    }
    canvas.drawPath(path, fillPaint);
    fillPaint.delete();
    const strokePaint = this.makeStrokePaint(stroke, opacity);
    if (strokePaint) {
      if (cr > 0) {
        const effect = ck.PathEffect.MakeCorner(cr);
        if (effect) strokePaint.setPathEffect(effect);
      }
      canvas.drawPath(path, strokePaint);
      strokePaint.delete();
    }
    path.delete();
  }

  private drawPath(
    canvas: Canvas,
    node: PenNode,
    x: number,
    y: number,
    w: number,
    h: number,
    opacity: number,
  ) {
    const ck = this.ck;
    const pNode = node as PathNode;
    const rawD = typeof pNode.d === 'string' && pNode.d.trim().length > 0 ? pNode.d : 'M0 0 L0 0';
    const fills = pNode.fill,
      stroke = pNode.stroke;

    let path: ReturnType<typeof ck.Path.MakeFromSVGString> = null;
    if (hasInvalidNumbers(rawD)) {
      path = tryManualPathParse(ck, rawD);
    } else {
      const d = sanitizeSvgPath(rawD);
      path = ck.Path.MakeFromSVGString(d);
      if (!path && d !== rawD) path = ck.Path.MakeFromSVGString(rawD);
      if (!path) path = tryManualPathParse(ck, rawD);
    }
    if (!path) {
      if (w > 0 && h > 0) {
        const { paint: fp } = this.makeFillPaint(fills, w, h, opacity, x, y);
        canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), fp);
        fp.delete();
      }
      return;
    }

    const parsedAnchors = pNode.anchors
      ? {
          anchors: pNode.anchors,
          closed: pNode.closed ?? /[Zz]\s*$/.test(rawD),
        }
      : pathDataToAnchors(rawD);
    const geometryBounds = parsedAnchors
      ? getPathBoundsFromAnchors(parsedAnchors.anchors, parsedAnchors.closed)
      : null;
    const bounds = geometryBounds
      ? Float32Array.of(
          geometryBounds.x,
          geometryBounds.y,
          geometryBounds.x + geometryBounds.width,
          geometryBounds.y + geometryBounds.height,
        )
      : path.getBounds();
    const nativeW = bounds[2] - bounds[0],
      nativeH = bounds[3] - bounds[1];
    if (w > 0 && h > 0 && nativeW > 0.01 && nativeH > 0.01) {
      const isIcon = !!pNode.iconId;
      const sx = isIcon ? Math.min(w / nativeW, h / nativeH) : w / nativeW;
      const sy = isIcon ? sx : h / nativeH;
      path.transform(
        ck.Matrix.multiply(
          ck.Matrix.translated(x - bounds[0] * sx, y - bounds[1] * sy),
          ck.Matrix.scaled(sx, sy),
        ),
      );
    } else if (nativeW > 0.01 || nativeH > 0.01) {
      const sx = nativeW > 0.01 && w > 0 ? w / nativeW : 1,
        sy = nativeH > 0.01 && h > 0 ? h / nativeH : 1;
      path.transform(
        ck.Matrix.multiply(
          ck.Matrix.translated(x - bounds[0] * sx, y - bounds[1] * sy),
          ck.Matrix.scaled(sx, sy),
        ),
      );
    } else {
      path.offset(x, y);
    }

    const hasExplicitFill = fills && fills.length > 0;
    const strokeColor = resolveStrokeColor(stroke),
      strokeWidth = resolveStrokeWidth(stroke);
    const hasVisibleStroke = strokeWidth > 0 && !!strokeColor;

    if (hasExplicitFill || !hasVisibleStroke) {
      const { paint: fillPaint } = this.makeFillPaint(
        hasExplicitFill ? fills : undefined,
        w,
        h,
        opacity,
        x,
        y,
      );
      const closeCount = (rawD.match(/Z/gi) || []).length;
      path.setFillType(closeCount > 1 ? ck.FillType.EvenOdd : ck.FillType.Winding);
      canvas.drawPath(path, fillPaint);
      fillPaint.delete();
    }
    if (hasVisibleStroke) {
      const sp = this.makeStrokePaint(stroke, opacity);
      if (sp) {
        canvas.drawPath(path, sp);
        sp.delete();
      }
    }
    path.delete();
  }

  private drawIconFont(
    canvas: Canvas,
    node: PenNode,
    x: number,
    y: number,
    w: number,
    h: number,
    opacity: number,
  ) {
    const ck = this.ck;
    const iNode = node as IconFontNode;
    const iconName = iNode.iconFontName ?? iNode.name ?? '';
    const iconMatch = this.iconLookup?.(iconName) ?? null;
    const iconD = iconMatch?.d ?? FALLBACK_ICON_D;
    const iconStyle = iconMatch?.style ?? 'stroke';

    const rawFill = iNode.fill;
    const iconFillColor =
      typeof rawFill === 'string'
        ? rawFill
        : Array.isArray(iNode.fill) && iNode.fill.length > 0
          ? resolveFillColor(iNode.fill)
          : '#64748B';

    const sanitizedIconD = sanitizeSvgPath(iconD);
    let path = ck.Path.MakeFromSVGString(sanitizedIconD);
    if (!path && sanitizedIconD !== iconD) path = ck.Path.MakeFromSVGString(iconD);
    if (!path) path = tryManualPathParse(ck, iconD);
    if (!path) return;

    const bounds = path.getBounds();
    const nativeW = bounds[2] - bounds[0],
      nativeH = bounds[3] - bounds[1];
    if (w > 0 && h > 0 && nativeW > 0 && nativeH > 0) {
      const s = Math.min(w / nativeW, h / nativeH);
      path.transform(
        ck.Matrix.multiply(
          ck.Matrix.translated(x - bounds[0] * s, y - bounds[1] * s),
          ck.Matrix.scaled(s, s),
        ),
      );
    } else {
      path.offset(x, y);
    }

    const paint = new ck.Paint();
    paint.setAntiAlias(true);
    const c = parseColor(ck, iconFillColor);
    c[3] *= opacity;
    paint.setColor(c);
    if (iconStyle === 'stroke') {
      paint.setStyle(ck.PaintStyle.Stroke);
      paint.setStrokeWidth(2);
      paint.setStrokeCap(ck.StrokeCap.Round);
      paint.setStrokeJoin(ck.StrokeJoin.Round);
    } else {
      paint.setStyle(ck.PaintStyle.Fill);
      path.setFillType(ck.FillType.EvenOdd);
    }
    canvas.drawPath(path, paint);
    paint.delete();
    path.delete();
  }

  // ---------------------------------------------------------------------------
  // Image drawing
  // ---------------------------------------------------------------------------

  private drawImage(
    canvas: Canvas,
    node: PenNode,
    x: number,
    y: number,
    w: number,
    h: number,
    opacity: number,
  ) {
    const ck = this.ck;
    const iNode = node as ImageNode;
    const src: string | undefined = iNode.src;
    const cr = cornerRadiusValue(iNode.cornerRadius);

    if (!src) {
      this.drawImageFallback(canvas, x, y, w, h, cr, opacity, false);
      return;
    }

    const cached = this.imageLoader.get(src);
    if (cached === undefined) {
      this.imageLoader.request(src);
      this.drawImageFallback(canvas, x, y, w, h, cr, opacity, false);
      return;
    }
    if (!cached) {
      const status = this.imageLoader.getStatus(src);
      this.drawImageFallback(
        canvas,
        x,
        y,
        w,
        h,
        cr,
        opacity,
        status?.state === 'missing' || status?.state === 'error',
      );
      return;
    }

    const imgW = cached.width(),
      imgH = cached.height();

    if (cr > 0) {
      canvas.save();
      const maxR = Math.min(cr, w / 2, h / 2);
      canvas.clipRRect(
        ck.RRectXY(ck.LTRBRect(x, y, x + w, y + h), maxR, maxR),
        ck.ClipOp.Intersect,
        true,
      );
    } else {
      canvas.save();
      canvas.clipRect(ck.LTRBRect(x, y, x + w, y + h), ck.ClipOp.Intersect, true);
    }

    const paint = new ck.Paint();
    paint.setAntiAlias(true);
    if (opacity < 1) paint.setAlphaf(opacity);
    const adjFilter = this.buildImageAdjustmentFilter(iNode);
    if (adjFilter) paint.setColorFilter(adjFilter);

    const fit = iNode.objectFit ?? 'fill';
    if (fit === 'tile') {
      const tileMatrix = Float32Array.of(1, 0, -x, 0, 1, -y, 0, 0, 1);
      const shader = cached.makeShaderOptions(
        ck.TileMode.Repeat,
        ck.TileMode.Repeat,
        ck.FilterMode.Linear,
        ck.MipmapMode.None,
        tileMatrix,
      );
      if (shader) {
        paint.setShader(shader);
        canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), paint);
      }
    } else if (fit === 'fit') {
      const bgPaint = new ck.Paint();
      bgPaint.setStyle(ck.PaintStyle.Fill);
      bgPaint.setColor(parseColor(ck, '#f3f4f6'));
      if (opacity < 1) bgPaint.setAlphaf(opacity * 0.3);
      else bgPaint.setAlphaf(0.3);
      canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), bgPaint);
      bgPaint.delete();
      const scale = Math.min(w / imgW, h / imgH),
        dw = imgW * scale,
        dh = imgH * scale;
      const dx = x + (w - dw) / 2,
        dy = y + (h - dh) / 2;
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      );
    } else {
      const scale = Math.max(w / imgW, h / imgH),
        dw = imgW * scale,
        dh = imgH * scale;
      const dx = x + (w - dw) / 2,
        dy = y + (h - dh) / 2;
      canvas.drawImageRect(
        cached,
        ck.LTRBRect(0, 0, imgW, imgH),
        ck.LTRBRect(dx, dy, dx + dw, dy + dh),
        paint,
      );
    }
    paint.delete();
    canvas.restore();
  }

  private drawImageFallback(
    canvas: Canvas,
    x: number,
    y: number,
    w: number,
    h: number,
    cr: number,
    opacity: number,
    emphasizeMissing: boolean,
  ) {
    const ck = this.ck;
    const rect = ck.LTRBRect(x, y, x + w, y + h);
    const maxR = Math.min(cr, w / 2, h / 2);

    const fillPaint = new ck.Paint();
    fillPaint.setStyle(ck.PaintStyle.Fill);
    fillPaint.setAntiAlias(true);
    const fillColor = parseColor(ck, emphasizeMissing ? '#f8d7da' : '#e5e7eb');
    fillColor[3] *= opacity;
    fillPaint.setColor(fillColor);

    if (cr > 0) {
      canvas.drawRRect(ck.RRectXY(rect, maxR, maxR), fillPaint);
    } else {
      canvas.drawRect(rect, fillPaint);
    }
    fillPaint.delete();

    const strokePaint = new ck.Paint();
    strokePaint.setStyle(ck.PaintStyle.Stroke);
    strokePaint.setAntiAlias(true);
    strokePaint.setStrokeWidth(Math.max(1, Math.min(w, h) * 0.02));
    const strokeColor = parseColor(ck, emphasizeMissing ? '#c2410c' : '#94a3b8');
    strokeColor[3] *= opacity;
    strokePaint.setColor(strokeColor);
    const dash = ck.PathEffect.MakeDash([8, 6], 0);
    if (dash) strokePaint.setPathEffect(dash);

    if (cr > 0) {
      canvas.drawRRect(ck.RRectXY(rect, maxR, maxR), strokePaint);
    } else {
      canvas.drawRect(rect, strokePaint);
    }

    const iconPaint = new ck.Paint();
    iconPaint.setStyle(ck.PaintStyle.Stroke);
    iconPaint.setAntiAlias(true);
    iconPaint.setStrokeWidth(Math.max(1.25, Math.min(w, h) * 0.03));
    iconPaint.setStrokeCap(ck.StrokeCap.Round);
    iconPaint.setStrokeJoin(ck.StrokeJoin.Round);
    iconPaint.setColor(strokeColor);

    const inset = Math.max(10, Math.min(w, h) * 0.18);
    const iconLeft = x + inset;
    const iconTop = y + inset;
    const iconRight = x + w - inset;
    const iconBottom = y + h - inset;
    const iconPath = new ck.Path();
    iconPath.moveTo(iconLeft, iconBottom);
    iconPath.lineTo(x + w * 0.42, y + h * 0.58);
    iconPath.lineTo(x + w * 0.58, y + h * 0.72);
    iconPath.lineTo(iconRight, y + h * 0.42);
    iconPath.moveTo(iconLeft, iconTop);
    iconPath.lineTo(iconRight, iconTop);
    iconPath.lineTo(iconRight, iconBottom);
    iconPath.lineTo(iconLeft, iconBottom);
    iconPath.close();
    canvas.drawPath(iconPath, iconPaint);
    iconPath.delete();

    const dotPaint = new ck.Paint();
    dotPaint.setStyle(ck.PaintStyle.Fill);
    dotPaint.setAntiAlias(true);
    dotPaint.setColor(strokeColor);
    canvas.drawCircle(x + w * 0.35, y + h * 0.38, Math.max(3, Math.min(w, h) * 0.05), dotPaint);

    if (emphasizeMissing) {
      canvas.drawLine(
        x + inset * 0.9,
        y + h - inset * 0.9,
        x + w - inset * 0.9,
        y + inset * 0.9,
        iconPaint,
      );
    }

    strokePaint.delete();
    iconPaint.delete();
    dotPaint.delete();
  }
}
