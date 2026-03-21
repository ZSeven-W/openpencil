import type { CanvasKit, Canvas } from 'canvaskit-wasm'
import { SkiaNodeRenderer, type RenderNode } from '@zseven-w/pen-renderer'
import { parseColor } from '@zseven-w/pen-renderer'
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
  drawArcHandles as _drawArcHandles,
  type PenPreviewData,
} from './skia-overlays'

export type { RenderNode } from '@zseven-w/pen-renderer'

/**
 * Editor-specific renderer that extends the core SkiaNodeRenderer
 * with selection borders, hover outlines, agent indicators, and other
 * interactive overlays.
 */
export class SkiaRenderer extends SkiaNodeRenderer {
  constructor(ck: CanvasKit) {
    super(ck)
  }

  /**
   * Draw a single render node with optional selection highlight.
   */
  drawNodeWithSelection(canvas: Canvas, rn: RenderNode, selectedIds: Set<string>) {
    super.drawNode(canvas, rn)
    if (selectedIds.has(rn.node.id)) {
      this.drawSelectionBorder(canvas, rn.absX, rn.absY, rn.absW, rn.absH)
    }
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
    } else if (shape.type === 'polygon') {
      const count = 3
      const raw: [number, number][] = []
      for (let i = 0; i < count; i++) {
        const angle = (i * 2 * Math.PI) / count - Math.PI / 2
        raw.push([Math.cos(angle), Math.sin(angle)])
      }
      let pMinX = Infinity, pMaxX = -Infinity, pMinY = Infinity, pMaxY = -Infinity
      for (const [rx, ry] of raw) {
        if (rx < pMinX) pMinX = rx
        if (rx > pMaxX) pMaxX = rx
        if (ry < pMinY) pMinY = ry
        if (ry > pMaxY) pMaxY = ry
      }
      const rw = pMaxX - pMinX
      const rh = pMaxY - pMinY
      const path = new ck.Path()
      for (let i = 0; i < count; i++) {
        const px = x + ((raw[i][0] - pMinX) / rw) * w
        const py = y + ((raw[i][1] - pMinY) / rh) * h
        if (i === 0) path.moveTo(px, py)
        else path.lineTo(px, py)
      }
      path.close()
      canvas.drawPath(path, fillPaint)
      canvas.drawPath(path, strokePaint)
      path.delete()
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

  drawArcHandles(
    canvas: Canvas,
    x: number, y: number, w: number, h: number,
    startAngle: number, sweepAngle: number, innerRadius: number,
    zoom: number,
  ) {
    _drawArcHandles(this.ck, canvas, x, y, w, h, startAngle, sweepAngle, innerRadius, zoom)
  }
}
