import * as fabric from 'fabric'
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

function shouldSplitByGrapheme(text: string): boolean {
  const hasCjk = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/.test(text)
  const hasLongCjkRun = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]{4,}/.test(text)
  return hasCjk && hasLongCjkRun
}

function isFixedWidthText(node: PenNode): boolean {
  if (node.type !== 'text') return false
  return node.textGrowth === 'fixed-width' || node.textGrowth === 'fixed-width-height'
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
      const fixedWidthText = isFixedWidthText(node)
      const fontSize = node.fontSize ?? 16
      const splitByGrapheme = shouldSplitByGrapheme(content)
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
      if (obj instanceof fabric.Textbox) {
        obj.set({ splitByGrapheme } as Partial<fabric.Textbox>)
        if (fixedWidthText && w > 0) obj.set({ width: w })
      }
      break
    }
    case 'image': {
      const w = sizeToNumber(node.width, 200)
      const h = sizeToNumber(node.height, 200)
      const r = cornerRadiusValue(node.cornerRadius)
      // Update scale to reflect target size over natural dimensions
      const nw = obj.width || w
      const nh = obj.height || h
      obj.set({
        scaleX: w / nw,
        scaleY: h / nh,
      })
      // Apply or remove rounded-rect clipPath for corner radius.
      // Disable objectCaching so clipPath updates render immediately
      // without relying on Fabric's cache invalidation.
      if (r > 0) {
        const clipR = r * nw / w
        obj.set({
          clipPath: new fabric.Rect({
            width: nw,
            height: nh,
            rx: clipR,
            ry: clipR,
            originX: 'center',
            originY: 'center',
          }),
          objectCaching: false,
          dirty: true,
        })
      } else {
        obj.set({
          clipPath: undefined,
          objectCaching: true,
          dirty: true,
        })
      }
      break
    }
    case 'polygon':
    case 'path': {
      // Update path data in-place when `d` changes — avoids object recreation
      // and preserves selection, position, and Fabric object identity.
      if (obj instanceof fabric.Path && node.type === 'path') {
        const nextD = typeof node.d === 'string' ? node.d.trim() : ''
        const trackedD = typeof (obj as any).__sourceD === 'string' ? (obj as any).__sourceD.trim() : ''
        if (nextD && nextD !== trackedD) {
          // Parse into a temporary Path to get the new internal representation
          const tmp = new fabric.Path(nextD)
          ;(obj as any).path = (tmp as any).path
          obj.width = tmp.width
          obj.height = tmp.height
          ;(obj as any).pathOffset = (tmp as any).pathOffset
          ;(obj as any).__sourceD = nextD
          ;(obj as any).__nativeWidth = tmp.width
          ;(obj as any).__nativeHeight = tmp.height
        }
      }

      const w = sizeToNumber('width' in node ? node.width : undefined, 100)
      const h = sizeToNumber('height' in node ? node.height : undefined, 100)
      const hasExplicitFill = node.type === 'path' && 'fill' in node && node.fill && node.fill.length > 0
      const strokeColor = resolveStrokeColor('stroke' in node ? node.stroke : undefined)
      const strokeWidth = resolveStrokeWidth('stroke' in node ? node.stroke : undefined)
      const hasVisibleStroke = node.type === 'path' && strokeWidth > 0 && !!strokeColor
      // For path nodes: stroke-only icons must not get a default fill
      const fill = node.type === 'path' && !hasExplicitFill && hasVisibleStroke
        ? 'transparent'
        : resolveFill('fill' in node ? node.fill : undefined, w, h)
      obj.set({
        fill,
        stroke: hasVisibleStroke ? strokeColor : undefined,
        strokeWidth: hasVisibleStroke ? strokeWidth : 0,
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
          // Keep native width/height to avoid pathOffset drift that can visually
          // offset icons inside logo containers.
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
