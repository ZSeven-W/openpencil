import type { FigmaPaint, FigmaMatrix } from './figma-types'
import type { PenFill } from '@/types/styles'
import { figmaColorToHex } from './figma-color-utils'

/**
 * Convert Figma fillPaints (internal format) to PenFill[].
 */
export function mapFigmaFills(paints: FigmaPaint[] | undefined): PenFill[] | undefined {
  if (!paints || paints.length === 0) return undefined
  const fills: PenFill[] = []

  for (const paint of paints) {
    if (paint.visible === false) continue
    const mapped = mapSingleFill(paint)
    if (mapped) fills.push(mapped)
  }

  return fills.length > 0 ? fills : undefined
}

function mapSingleFill(paint: FigmaPaint): PenFill | null {
  switch (paint.type) {
    case 'SOLID': {
      if (!paint.color) return null
      return {
        type: 'solid',
        color: figmaColorToHex(paint.color),
        opacity: paint.opacity,
      }
    }

    case 'GRADIENT_LINEAR': {
      if (!paint.stops) return null
      const angle = paint.transform
        ? gradientAngleFromTransform(paint.transform)
        : 0
      return {
        type: 'linear_gradient',
        angle,
        stops: paint.stops.map((s) => ({
          offset: s.position,
          color: figmaColorToHex(s.color),
        })),
        opacity: paint.opacity,
      }
    }

    case 'GRADIENT_RADIAL':
    case 'GRADIENT_ANGULAR':
    case 'GRADIENT_DIAMOND': {
      if (!paint.stops) return null
      return {
        type: 'radial_gradient',
        cx: 0.5,
        cy: 0.5,
        radius: 0.5,
        stops: paint.stops.map((s) => ({
          offset: s.position,
          color: figmaColorToHex(s.color),
        })),
        opacity: paint.opacity,
      }
    }

    case 'IMAGE': {
      // Image fills reference blobs; we'll resolve them later
      const blobIndex = paint.image?.dataBlob
      return {
        type: 'image',
        url: blobIndex !== undefined ? `__blob:${blobIndex}` : '',
        mode: mapScaleMode(paint.imageScaleMode),
        opacity: paint.opacity,
      }
    }

    default:
      return null
  }
}

function gradientAngleFromTransform(m: FigmaMatrix): number {
  return Math.round(Math.atan2(m.m10, m.m00) * (180 / Math.PI))
}

function mapScaleMode(mode?: string): 'stretch' | 'fill' | 'fit' {
  switch (mode) {
    case 'FIT': return 'fit'
    case 'STRETCH': return 'stretch'
    default: return 'fill'
  }
}
