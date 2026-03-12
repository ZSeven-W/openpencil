import * as fabric from 'fabric'
import type { PenNode } from '@/types/pen'
import { buildEllipseArcPath, isArcEllipse } from '@/utils/arc-path'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { lookupIconByName } from '@/services/ai/icon-resolver'
import {
  resolveFill,
  resolveFillColor,
  resolveShadow,
  resolveStrokeColor,
  resolveStrokeWidth,
  computeImageTransform,
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
        const r = Math.min(cornerRadiusValue(node.cornerRadius), h / 2)
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
        const r = Math.min(cornerRadiusValue(node.cornerRadius), h / 2)
        obj.set({ rx: r, ry: r })
      }
      break
    }
    case 'ellipse': {
      const w = sizeToNumber(node.width, 100)
      const h = sizeToNumber(node.height, 100)
      if (isArcEllipse(node.startAngle, node.sweepAngle, node.innerRadius)) {
        // Arc ellipse rendered as Fabric.Path — update path data
        const arcD = buildEllipseArcPath(w, h, node.startAngle ?? 0, node.sweepAngle ?? 360, node.innerRadius ?? 0)
        if (obj instanceof fabric.Path) {
          const trackedD = typeof (obj as any).__sourceD === 'string' ? (obj as any).__sourceD.trim() : ''
          if (arcD !== trackedD) {
            const tmp = new fabric.Path(arcD)
            ;(obj as any).path = (tmp as any).path
            ;(obj as any).__sourceD = arcD
            ;(obj as any).__nativeWidth = w
            ;(obj as any).__nativeHeight = h
          }
        }
        // Override Fabric's auto-computed bounding box — the arc path is
        // drawn within a 0,0 → w,h coordinate space.
        ;(obj as any).pathOffset = new fabric.Point(w / 2, h / 2)
        obj.set({
          width: w,
          height: h,
          scaleX: 1,
          scaleY: 1,
          fill: resolveFill(node.fill, w, h),
          stroke: resolveStrokeColor(node.stroke),
          strokeWidth: resolveStrokeWidth(node.stroke),
        })
      } else {
        obj.set({
          rx: w / 2,
          ry: h / 2,
          fill: resolveFill(node.fill, w, h),
          stroke: resolveStrokeColor(node.stroke),
          strokeWidth: resolveStrokeWidth(node.stroke),
        })
      }
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
      const r = Math.min(cornerRadiusValue(node.cornerRadius), h / 2)
      const fitMode = node.objectFit ?? 'fill'

      // Detect mode changes that require object recreation (e.g. tile ↔ non-tile)
      const prevMode = (obj as any).__objectFit ?? 'fill'
      if (prevMode !== fitMode) {
        ;(obj as any).__needsRecreation = true
        return
      }

      // Tile mode: update pattern fill on the Rect
      if (fitMode === 'tile') {
        obj.set({ width: w, height: h, rx: r, ry: r, dirty: true })
        break
      }

      // Fill/Fit/Crop: use computeImageTransform with native (source) dimensions
      const nw = (obj as any).__nativeWidth || obj.width || w
      const nh = (obj as any).__nativeHeight || obj.height || h
      const transform = computeImageTransform(nw, nh, w, h, fitMode, r)
      obj.set({
        cropX: transform.cropX,
        cropY: transform.cropY,
        width: transform.cropWidth,
        height: transform.cropHeight,
        scaleX: transform.scaleX,
        scaleY: transform.scaleY,
      })
      // clipPath only for corner radius (fill/crop overflow handled by cropX/cropY)
      if (transform.clipPath) {
        obj.set({
          clipPath: transform.clipPath,
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
    case 'icon_font': {
      const iw = sizeToNumber('width' in node ? node.width : undefined, 20)
      const ih = sizeToNumber('height' in node ? node.height : undefined, 20)

      // Re-resolve icon if name changed
      const iconName = (node as any).iconFontName ?? node.name ?? ''
      const prevIconName = (obj as any).__iconFontName ?? ''
      if (iconName !== prevIconName && obj instanceof fabric.Path) {
        const iconMatch = lookupIconByName(iconName)
        const iconD = iconMatch?.d ?? 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0'
        const iconStyle = iconMatch?.style ?? 'stroke'
        const tmp = new fabric.Path(iconD)
        ;(obj as any).path = (tmp as any).path
        obj.width = tmp.width
        obj.height = tmp.height
        ;(obj as any).pathOffset = (tmp as any).pathOffset
        ;(obj as any).__nativeWidth = tmp.width
        ;(obj as any).__nativeHeight = tmp.height
        ;(obj as any).__iconFontName = iconName
        ;(obj as any).__iconStyle = iconStyle
      }

      // Resolve fill color
      const rawFill = (node as any).fill
      const iconFillColor = typeof rawFill === 'string'
        ? rawFill
        : Array.isArray(rawFill) && rawFill.length > 0
          ? resolveFillColor(rawFill)
          : '#64748B'
      const iconStyle = (obj as any).__iconStyle ?? 'stroke'
      obj.set({
        fill: iconStyle === 'stroke' ? 'transparent' : iconFillColor,
        stroke: iconStyle === 'stroke' ? iconFillColor : undefined,
        strokeWidth: iconStyle === 'stroke' ? 2 : 0,
      })

      // Uniform scale for icons
      const inw = (obj as any).__nativeWidth || obj.width
      const inh = (obj as any).__nativeHeight || obj.height
      if (iw > 0 && ih > 0 && inw && inh) {
        const uniformScale = Math.min(iw / inw, ih / inh)
        obj.set({ width: inw, height: inh, scaleX: uniformScale, scaleY: uniformScale })
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
        if (node.type === 'path' && (node as any).iconId) {
          // Icon path: uniform scale to preserve aspect ratio
          const uniformScale = Math.min(w / nw, h / nh)
          obj.set({ width: nw, height: nh, scaleX: uniformScale, scaleY: uniformScale })
        } else {
          // Regular path / polygon: non-uniform scale to fill exact bounds
          obj.set({ width: nw, height: nh, scaleX: w / nw, scaleY: h / nh })
        }
      }
      break
    }
  }

  obj.setCoords()
}
