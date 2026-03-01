import type { FigmaNodeChange } from './figma-types'
import type { PenStroke } from '@/types/styles'
import { mapFigmaFills } from './figma-fill-mapper'

/**
 * Convert Figma strokePaints + strokeWeight to PenStroke.
 */
export function mapFigmaStroke(node: FigmaNodeChange): PenStroke | undefined {
  if (!node.strokePaints || node.strokePaints.length === 0) return undefined
  const visibleStrokes = node.strokePaints.filter((s) => s.visible !== false)
  if (visibleStrokes.length === 0) return undefined

  const thickness = node.borderStrokeWeightsIndependent
    ? [
        node.borderTopWeight ?? 0,
        node.borderRightWeight ?? 0,
        node.borderBottomWeight ?? 0,
        node.borderLeftWeight ?? 0,
      ] as [number, number, number, number]
    : (node.strokeWeight ?? 1)

  const fill = mapFigmaFills(visibleStrokes)

  return {
    thickness,
    align: mapStrokeAlign(node.strokeAlign),
    join: mapStrokeJoin(node.strokeJoin),
    cap: mapStrokeCap(node.strokeCap),
    dashPattern: node.dashPattern?.length ? node.dashPattern : undefined,
    fill,
  }
}

function mapStrokeAlign(
  align?: string
): 'inside' | 'center' | 'outside' | undefined {
  switch (align) {
    case 'INSIDE': return 'inside'
    case 'OUTSIDE': return 'outside'
    case 'CENTER': return 'center'
    default: return undefined
  }
}

function mapStrokeJoin(
  join?: string
): 'miter' | 'bevel' | 'round' | undefined {
  switch (join) {
    case 'MITER': return 'miter'
    case 'BEVEL': return 'bevel'
    case 'ROUND': return 'round'
    default: return undefined
  }
}

function mapStrokeCap(
  cap?: string
): 'none' | 'round' | 'square' | undefined {
  switch (cap) {
    case 'NONE': return 'none'
    case 'ROUND': return 'round'
    case 'SQUARE': return 'square'
    default: return undefined
  }
}
