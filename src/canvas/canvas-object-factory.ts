import * as fabric from 'fabric'
import type { PenNode } from '@/types/pen'
import type {
  PenFill,
  PenStroke,
  PenEffect,
  LinearGradientFill,
  RadialGradientFill,
  ShadowEffect,
} from '@/types/styles'
import {
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
  SELECTION_BLUE,
} from './canvas-constants'
import { applyRotationControls } from './canvas-controls'

function angleToCoords(
  angleDeg: number,
  width: number,
  height: number,
): { x1: number; y1: number; x2: number; y2: number } {
  const rad = ((angleDeg - 90) * Math.PI) / 180
  const cos = Math.cos(rad)
  const sin = Math.sin(rad)
  return {
    x1: width / 2 - (cos * width) / 2,
    y1: height / 2 - (sin * height) / 2,
    x2: width / 2 + (cos * width) / 2,
    y2: height / 2 + (sin * height) / 2,
  }
}

function sanitizeColorStops(
  stops: { offset: number; color: string }[],
): { offset: number; color: string }[] {
  return stops.map((s) => ({
    offset: Number.isFinite(s.offset)
      ? Math.max(0, Math.min(1, s.offset))
      : 0,
    color: s.color,
  }))
}

function createLinearGradient(
  fill: LinearGradientFill,
  width: number,
  height: number,
): fabric.Gradient<'linear'> {
  const coords = angleToCoords(fill.angle ?? 0, width, height)
  return new fabric.Gradient({
    type: 'linear',
    coords,
    colorStops: sanitizeColorStops(fill.stops),
  })
}

function createRadialGradient(
  fill: RadialGradientFill,
  width: number,
  height: number,
): fabric.Gradient<'radial'> {
  const cx = (fill.cx ?? 0.5) * width
  const cy = (fill.cy ?? 0.5) * height
  const r = (fill.radius ?? 0.5) * Math.max(width, height)
  return new fabric.Gradient({
    type: 'radial',
    coords: { x1: cx, y1: cy, r1: 0, x2: cx, y2: cy, r2: r },
    colorStops: sanitizeColorStops(fill.stops),
  })
}

export function resolveFill(
  fills: PenFill[] | undefined,
  width: number,
  height: number,
): string | fabric.Gradient<'linear'> | fabric.Gradient<'radial'> {
  if (!fills || fills.length === 0) return DEFAULT_FILL
  const first = fills[0]
  if (first.type === 'solid') return first.color
  if (first.type === 'linear_gradient') {
    if (!first.stops || first.stops.length === 0) return DEFAULT_FILL
    return createLinearGradient(first, width, height)
  }
  if (first.type === 'radial_gradient') {
    if (!first.stops || first.stops.length === 0) return DEFAULT_FILL
    return createRadialGradient(first, width, height)
  }
  return DEFAULT_FILL
}

export function resolveFillColor(fills?: PenFill[]): string {
  if (!fills || fills.length === 0) return DEFAULT_FILL
  const first = fills[0]
  if (first.type === 'solid') return first.color
  if (
    first.type === 'linear_gradient' ||
    first.type === 'radial_gradient'
  ) {
    return first.stops[0]?.color ?? DEFAULT_FILL
  }
  return DEFAULT_FILL
}

export function resolveShadow(
  effects?: PenEffect[],
): fabric.Shadow | undefined {
  if (!effects) return undefined
  const shadow = effects.find(
    (e): e is ShadowEffect => e.type === 'shadow',
  )
  if (!shadow) return undefined
  return new fabric.Shadow({
    color: shadow.color,
    blur: shadow.blur,
    offsetX: shadow.offsetX,
    offsetY: shadow.offsetY,
  })
}

export function resolveStrokeColor(stroke?: PenStroke): string | undefined {
  if (!stroke) return undefined
  if (stroke.fill && stroke.fill.length > 0) {
    return resolveFillColor(stroke.fill)
  }
  return DEFAULT_STROKE
}

export function resolveStrokeWidth(stroke?: PenStroke): number {
  if (!stroke) return 0
  if (typeof stroke.thickness === 'number') return stroke.thickness
  return stroke.thickness[0] ?? DEFAULT_STROKE_WIDTH
}

function resolveTextContent(
  content: string | { text: string }[],
): string {
  if (typeof content === 'string') return content
  return content.map((s) => s.text).join('')
}

function shouldSplitByGrapheme(text: string): boolean {
  // Mixed CJK content (especially with a long CJK run) wraps poorly with word-based splitting.
  // Enable grapheme splitting so Textbox can break between CJK characters naturally.
  const hasCjk = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/.test(text)
  const hasLongCjkRun = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]{4,}/.test(text)
  return hasCjk && hasLongCjkRun
}

function isFixedWidthText(node: PenNode): boolean {
  if (node.type !== 'text') return false
  return node.textGrowth === 'fixed-width' || node.textGrowth === 'fixed-width-height'
}

function sizeToNumber(
  val: number | string | undefined,
  fallback: number,
): number {
  if (typeof val === 'number') return val
  if (typeof val === 'string') {
    // Handle "fit_content(N)" / "fill_container(N)"
    const m = val.match(/\((\d+(?:\.\d+)?)\)/)
    if (m) return parseFloat(m[1])
    const n = parseFloat(val)
    if (!isNaN(n)) return n
  }
  return fallback
}

function cornerRadiusValue(
  cr: number | [number, number, number, number] | undefined,
): number {
  if (cr === undefined) return 0
  if (typeof cr === 'number') return cr
  return cr[0]
}

export interface FabricObjectWithPenId extends fabric.FabricObject {
  penNodeId?: string
}

export function createFabricObject(
  node: PenNode,
): FabricObjectWithPenId | null {
  let obj: FabricObjectWithPenId | null = null

  const baseProps = {
    left: node.x ?? 0,
    top: node.y ?? 0,
    originX: 'left' as const,
    originY: 'top' as const,
    angle: node.rotation ?? 0,
    opacity: typeof node.opacity === 'number' ? node.opacity : 1,
  }

  // Resolve effects (shadow)
  const effects = 'effects' in node ? node.effects : undefined
  const shadow = resolveShadow(effects)

  // Resolve visibility and lock
  const visible = ('visible' in node ? node.visible : undefined) !== false
  const locked = ('locked' in node ? node.locked : undefined) === true

  switch (node.type) {
    case 'frame': {
      // Frames without explicit fill are transparent containers
      const r = cornerRadiusValue(node.cornerRadius)
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      const hasFill = node.fill && node.fill.length > 0
      obj = new fabric.Rect({
        ...baseProps,
        width: w,
        height: h,
        rx: r,
        ry: r,
        fill: hasFill ? resolveFill(node.fill, w, h) : 'transparent',
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      }) as FabricObjectWithPenId
      break
    }
    case 'rectangle': {
      const r = cornerRadiusValue(node.cornerRadius)
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      obj = new fabric.Rect({
        ...baseProps,
        width: w,
        height: h,
        rx: r,
        ry: r,
        fill: resolveFill(node.fill, w, h),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      }) as FabricObjectWithPenId
      break
    }
    case 'ellipse': {
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      obj = new fabric.Ellipse({
        ...baseProps,
        rx: w / 2,
        ry: h / 2,
        fill: resolveFill(node.fill, w, h),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      }) as FabricObjectWithPenId
      break
    }
    case 'line': {
      obj = new fabric.Line(
        [
          node.x ?? 0,
          node.y ?? 0,
          node.x2 ?? (node.x ?? 0) + 100,
          node.y2 ?? (node.y ?? 0),
        ],
        {
          ...baseProps,
          stroke: resolveStrokeColor(node.stroke) ?? DEFAULT_STROKE,
          strokeWidth: resolveStrokeWidth(node.stroke) || DEFAULT_STROKE_WIDTH,
          fill: '',
        },
      ) as FabricObjectWithPenId
      break
    }
    case 'polygon': {
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      const count = node.polygonCount || 6
      const points = Array.from({ length: count }, (_, i) => {
        const angle = (i * 2 * Math.PI) / count - Math.PI / 2
        return {
          x: (w / 2) * Math.cos(angle) + w / 2,
          y: (h / 2) * Math.sin(angle) + h / 2,
        }
      })
      obj = new fabric.Polygon(points, {
        ...baseProps,
        fill: resolveFill(node.fill, w, h),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      }) as FabricObjectWithPenId
      // Cache native dimensions before scaling (Polygon width/height is derived from points)
      ;(obj as any).__nativeWidth = obj.width
      ;(obj as any).__nativeHeight = obj.height
      if (w > 0 && h > 0 && obj.width && obj.height) {
        obj.set({ scaleX: w / obj.width, scaleY: h / obj.height })
      }
      break
    }
    case 'path': {
      const pw = sizeToNumber(node.width, 0)
      const ph = sizeToNumber(node.height, 0)
      const safePathData =
        typeof node.d === 'string' && node.d.trim().length > 0
          ? node.d
          : 'M0 0 L0 0'
      const hasExplicitFill = node.fill && node.fill.length > 0
      const strokeColor = resolveStrokeColor(node.stroke)
      const strokeWidth = resolveStrokeWidth(node.stroke)
      const hasVisibleStroke = strokeWidth > 0 && !!strokeColor
      // Stroke-only icons (e.g. Lucide-style) must not get a default fill.
      // Use 'transparent' (not 'none' — Fabric.js ignores 'none' and falls back to black).
      const pathFill = hasExplicitFill
        ? resolveFill(node.fill, pw || 100, ph || 100)
        : hasVisibleStroke
          ? 'transparent'
          : DEFAULT_FILL
      obj = new fabric.Path(safePathData, {
        ...baseProps,
        fill: pathFill,
        stroke: hasVisibleStroke ? strokeColor : undefined,
        strokeWidth: hasVisibleStroke ? strokeWidth : 0,
        strokeUniform: true,
        fillRule: 'evenodd', // Compound paths: inner sub-paths become transparent cutouts
      }) as FabricObjectWithPenId
      ;(obj as any).__sourceD = safePathData
      // Cache native dimensions before scaling (Path width/height is derived from d)
      ;(obj as any).__nativeWidth = obj.width
      ;(obj as any).__nativeHeight = obj.height
      if (pw > 0 && ph > 0 && obj.width && obj.height) {
        // Uniform scale — preserve aspect ratio so icons don't get squished
        const uniformScale = Math.min(pw / obj.width, ph / obj.height)
        // Keep native path width/height. Overriding width/height can shift pathOffset
        // and make icons appear visually off-center in logos.
        obj.set({ scaleX: uniformScale, scaleY: uniformScale })
      }
      break
    }
    case 'text': {
      const textContent = resolveTextContent(node.content)
      const w = sizeToNumber(node.width, 0)
      const splitByGrapheme = shouldSplitByGrapheme(textContent)
      const textProps = {
        ...baseProps,
        fontFamily: node.fontFamily ?? 'Inter, sans-serif',
        fontSize: node.fontSize ?? 16,
        fontWeight: (node.fontWeight as string) ?? 'normal',
        fontStyle: node.fontStyle ?? 'normal',
        fill: resolveFillColor(node.fill),
        textAlign: node.textAlign ?? 'left',
        underline: node.underline ?? false,
        linethrough: node.strikethrough ?? false,
        lineHeight: node.lineHeight ?? 1.2,
        charSpacing: node.letterSpacing
          ? (node.letterSpacing / (node.fontSize || 16)) * 1000
          : 0,
      }
      // Use Textbox for fixed-width / fixed-size modes (word wrapping).
      // Use IText for auto-width mode (no wrapping, expands horizontally).
      const useTextbox = isFixedWidthText(node)
      if (useTextbox) {
        obj = new fabric.Textbox(textContent, {
          ...textProps,
          width: w > 0 ? w : 200,
          splitByGrapheme,
        }) as FabricObjectWithPenId
      } else {
        obj = new fabric.IText(textContent, textProps) as FabricObjectWithPenId
      }
      break
    }
    case 'image': {
      const w = sizeToNumber(node.width, 200)
      const h = sizeToNumber(node.height, 200)
      const r = cornerRadiusValue(node.cornerRadius)
      const imgEl = new Image()
      imgEl.src = node.src

      // Build a rounded-rect clipPath for corner radius
      const makeImageClip = (cw: number, ch: number, cr: number) => {
        if (cr <= 0) return undefined
        return new fabric.Rect({
          width: cw,
          height: ch,
          rx: cr,
          ry: cr,
          originX: 'center',
          originY: 'center',
        })
      }

      if (imgEl.complete) {
        const nw = imgEl.naturalWidth || w
        const nh = imgEl.naturalHeight || h
        const clip = makeImageClip(nw, nh, r * nw / w)
        obj = new fabric.FabricImage(imgEl, {
          ...baseProps,
          width: nw,
          height: nh,
          scaleX: w / nw,
          scaleY: h / nh,
          clipPath: clip ?? undefined,
          objectCaching: !clip,
        }) as unknown as FabricObjectWithPenId
      } else {
        // Placeholder while image loads
        const placeholder = new fabric.Rect({
          ...baseProps,
          width: w,
          height: h,
          rx: r,
          ry: r,
          fill: '#e5e7eb',
          strokeWidth: 0,
        }) as FabricObjectWithPenId
        placeholder.penNodeId = node.id
        imgEl.onload = () => {
          const canvas = placeholder.canvas
          if (!canvas) return
          const nw = imgEl.naturalWidth
          const nh = imgEl.naturalHeight
          const fabricImg = new fabric.FabricImage(imgEl, {
            ...baseProps,
            left: placeholder.left,
            top: placeholder.top,
            width: nw,
            height: nh,
            scaleX: w / nw,
            scaleY: h / nh,
          }) as unknown as FabricObjectWithPenId
          fabricImg.penNodeId = node.id
          fabricImg.set({
            borderColor: SELECTION_BLUE,
            borderScaleFactor: 2,
            cornerColor: SELECTION_BLUE,
            cornerStrokeColor: '#ffffff',
            cornerStyle: 'rect',
            cornerSize: 8,
            transparentCorners: false,
            borderOpacityWhenMoving: 1,
            padding: 0,
            hoverCursor: 'default',
          })
          fabricImg.setControlVisible('mtr', false)
          applyRotationControls(fabricImg)
          if (shadow) fabricImg.shadow = shadow
          fabricImg.visible = visible
          fabricImg.selectable = !locked
          fabricImg.evented = !locked
          // Apply rounded-rect clip for corner radius
          const clip = makeImageClip(nw, nh, r * nw / w)
          if (clip) {
            fabricImg.clipPath = clip
            fabricImg.objectCaching = false
          }
          // Preserve clipPath from placeholder so clipped-frame children stay clipped
          if (!clip && placeholder.clipPath) {
            fabricImg.clipPath = placeholder.clipPath
            fabricImg.dirty = true
          }
          // Preserve z-order: insert at placeholder's index instead of
          // appending to end (which would put the image on top of everything)
          const idx = canvas.getObjects().indexOf(placeholder)
          canvas.remove(placeholder)
          if (idx >= 0) {
            canvas.insertAt(idx, fabricImg)
          } else {
            canvas.add(fabricImg)
          }
          canvas.requestRenderAll()
        }
        obj = placeholder
      }
      break
    }
    case 'group': {
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      obj = new fabric.Rect({
        ...baseProps,
        width: w,
        height: h,
        fill: resolveFill(node.fill, w, h),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
        selectable: true,
      }) as FabricObjectWithPenId
      break
    }
    case 'ref': {
      // RefNodes need to be resolved before rendering
      return null
    }
  }

  if (obj) {
    obj.penNodeId = node.id
    // Selection styling (from HEAD)
    obj.set({
      borderColor: SELECTION_BLUE,
      borderScaleFactor: 2,
      cornerColor: SELECTION_BLUE,
      cornerStrokeColor: '#ffffff',
      cornerStyle: 'rect',
      cornerSize: 8,
      transparentCorners: false,
      borderOpacityWhenMoving: 1,
      padding: 0,
      hoverCursor: 'default',
    })
    obj.setControlVisible('mtr', false)
    applyRotationControls(obj)
    // Shadow, visibility, lock (from theirs)
    if (shadow) obj.shadow = shadow
    obj.visible = visible
    obj.selectable = !locked
    obj.evented = !locked
  }
  return obj
}
