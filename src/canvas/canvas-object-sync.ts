import type { PenNode } from '@/types/pen'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import {
  resolveFill,
  resolveFillColor,
  resolveShadow,
  resolveStrokeColor,
  resolveStrokeWidth,
} from './canvas-object-factory'

function sizeToNumber(
  val: number | string | undefined,
  fallback: number,
): number {
  if (typeof val === 'number') return val
  if (typeof val === 'string') {
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

export function syncFabricObject(
  obj: FabricObjectWithPenId,
  node: PenNode,
) {
  const visible = ('visible' in node ? node.visible : undefined) !== false
  const locked = ('locked' in node ? node.locked : undefined) === true
  const effects = 'effects' in node ? node.effects : undefined
  const shadow = resolveShadow(effects)

  obj.set({
    left: node.x ?? obj.left,
    top: node.y ?? obj.top,
    angle: node.rotation ?? 0,
    opacity: typeof node.opacity === 'number' ? node.opacity : 1,
    visible,
    selectable: !locked,
    evented: !locked,
  })
  obj.shadow = shadow ?? null

  switch (node.type) {
    case 'frame': {
      // Frames without explicit fill are transparent containers
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      const hasFill = node.fill && node.fill.length > 0
      obj.set({
        width: w,
        height: h,
        fill: hasFill ? resolveFill(node.fill, w, h) : 'transparent',
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      })
      if ('rx' in obj) {
        const r = cornerRadiusValue(node.cornerRadius)
        obj.set({ rx: r, ry: r })
      }
      break
    }
    case 'rectangle':
    case 'group': {
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      obj.set({
        width: w,
        height: h,
        fill: resolveFill(node.fill, w, h),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      })
      if ('rx' in obj) {
        const r = cornerRadiusValue(node.cornerRadius)
        obj.set({ rx: r, ry: r })
      }
      break
    }
    case 'ellipse': {
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      obj.set({
        rx: w / 2,
        ry: h / 2,
        fill: resolveFill(node.fill, w, h),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      })
      break
    }
    case 'line': {
      obj.set({
        x1: node.x ?? 0,
        y1: node.y ?? 0,
        x2: node.x2 ?? 100,
        y2: node.y2 ?? 0,
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      })
      break
    }
    case 'text': {
      const content =
        typeof node.content === 'string'
          ? node.content
          : node.content.map((s) => s.text).join('')
      const w = sizeToNumber(node.width, 0)
      const fontSize = node.fontSize ?? 16
      obj.set({
        text: content,
        fontFamily: node.fontFamily ?? 'Inter, sans-serif',
        fontSize,
        fontWeight: (node.fontWeight as string) ?? 'normal',
        fontStyle: node.fontStyle ?? 'normal',
        fill: resolveFillColor(node.fill),
        textAlign: node.textAlign ?? 'left',
        lineHeight: node.lineHeight ?? 1.2,
        charSpacing: node.letterSpacing
          ? (node.letterSpacing / fontSize) * 1000
          : 0,
      })
      if (w > 0) obj.set({ width: w })
      break
    }
    case 'polygon':
    case 'path': {
      const w = sizeToNumber('width' in node ? node.width : undefined, 100)
      const h = sizeToNumber('height' in node ? node.height : undefined, 100)
      const hasExplicitFill = node.type === 'path' && 'fill' in node && node.fill && node.fill.length > 0
      const hasStroke = 'stroke' in node && !!node.stroke
      // For path nodes: stroke-only icons must not get a default fill
      const fill = node.type === 'path' && !hasExplicitFill && hasStroke
        ? 'transparent'
        : resolveFill('fill' in node ? node.fill : undefined, w, h)
      obj.set({
        fill,
        stroke: resolveStrokeColor('stroke' in node ? node.stroke : undefined),
        strokeWidth: resolveStrokeWidth('stroke' in node ? node.stroke : undefined),
        ...(node.type === 'path' ? { strokeUniform: true, fillRule: 'evenodd' } : {}),
      })
      // Use cached native dimensions (from path/points data) to compute correct
      // scale, even if obj.width was previously corrupted by scale baking.
      const nw = (obj as any).__nativeWidth || obj.width
      const nh = (obj as any).__nativeHeight || obj.height
      if (w > 0 && h > 0 && nw && nh) {
        if (node.type === 'path') {
          // Uniform scale — preserve aspect ratio so icons don't get squished
          const uniformScale = Math.min(w / nw, h / nh)
          obj.set({ width: nw, height: nh, scaleX: uniformScale, scaleY: uniformScale })
        } else {
          obj.set({ width: nw, height: nh, scaleX: w / nw, scaleY: h / nh })
        }
      }
      break
    }
  }

  obj.setCoords()
}
