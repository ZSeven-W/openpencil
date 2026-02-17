import type { PenNode } from '@/types/pen'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import {
  resolveFillColor,
  resolveStrokeColor,
  resolveStrokeWidth,
} from './canvas-object-factory'

function sizeToNumber(
  val: number | string | undefined,
  fallback: number,
): number {
  if (typeof val === 'number') return val
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
  obj.set({
    left: node.x ?? obj.left,
    top: node.y ?? obj.top,
    angle: node.rotation ?? 0,
    opacity: typeof node.opacity === 'number' ? node.opacity : 1,
  })

  switch (node.type) {
    case 'rectangle':
    case 'frame':
    case 'group': {
      obj.set({
        width: sizeToNumber(node.width, 100),
        height: sizeToNumber(node.height, 100),
        fill: resolveFillColor(node.fill),
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
        fill: resolveFillColor(node.fill),
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
      obj.set({
        text: content,
        fontFamily: node.fontFamily ?? 'Inter, sans-serif',
        fontSize: node.fontSize ?? 16,
        fontWeight: (node.fontWeight as string) ?? 'normal',
        fontStyle: node.fontStyle ?? 'normal',
        fill: resolveFillColor(node.fill),
        textAlign: node.textAlign ?? 'left',
      })
      break
    }
    case 'polygon':
    case 'path': {
      obj.set({
        fill: resolveFillColor(node.fill),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      })
      break
    }
  }

  obj.setCoords()
}
