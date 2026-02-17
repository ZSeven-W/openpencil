import * as fabric from 'fabric'
import type { PenNode } from '@/types/pen'
import type { PenFill, PenStroke } from '@/types/styles'
import {
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
} from './canvas-constants'

export function resolveFillColor(fills?: PenFill[]): string {
  if (!fills || fills.length === 0) return DEFAULT_FILL
  const first = fills[0]
  if (first.type === 'solid') return first.color
  // Gradients/images: fallback to first color or default
  if (
    first.type === 'linear_gradient' ||
    first.type === 'radial_gradient'
  ) {
    return first.stops[0]?.color ?? DEFAULT_FILL
  }
  return DEFAULT_FILL
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

  switch (node.type) {
    case 'rectangle':
    case 'frame': {
      const r = cornerRadiusValue(node.cornerRadius)
      obj = new fabric.Rect({
        ...baseProps,
        width: sizeToNumber(node.width, 100),
        height: sizeToNumber(node.height, 100),
        rx: r,
        ry: r,
        fill: resolveFillColor(node.fill),
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
        fill: resolveFillColor(node.fill),
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
        fill: resolveFillColor(node.fill),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      }) as FabricObjectWithPenId
      break
    }
    case 'path': {
      obj = new fabric.Path(node.d, {
        ...baseProps,
        fill: resolveFillColor(node.fill),
        stroke: resolveStrokeColor(node.stroke),
        strokeWidth: resolveStrokeWidth(node.stroke),
      }) as FabricObjectWithPenId
      break
    }
    case 'text': {
      obj = new fabric.IText(resolveTextContent(node.content), {
        ...baseProps,
        width: sizeToNumber(node.width, undefined!),
        fontFamily: node.fontFamily ?? 'Inter, sans-serif',
        fontSize: node.fontSize ?? 16,
        fontWeight: (node.fontWeight as string) ?? 'normal',
        fontStyle: node.fontStyle ?? 'normal',
        fill: resolveFillColor(node.fill),
        textAlign: node.textAlign ?? 'left',
        underline: node.underline ?? false,
        linethrough: node.strikethrough ?? false,
      }) as FabricObjectWithPenId
      break
    }
    case 'group': {
      // Groups are rendered as their children; the group itself is a container
      obj = new fabric.Rect({
        ...baseProps,
        width: sizeToNumber(node.width, 100),
        height: sizeToNumber(node.height, 100),
        fill: resolveFillColor(node.fill),
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
  }
  return obj
}
