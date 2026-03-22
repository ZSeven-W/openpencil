import type { FigmaEffect } from './figma-types'
import type { PenEffect } from '@/types/styles'
import { figmaColorToHex } from './figma-color-utils'

/**
 * Convert Figma effects[] (internal format) to PenEffect[].
 */
export function mapFigmaEffects(
  effects: FigmaEffect[] | undefined
): PenEffect[] | undefined {
  if (!effects || effects.length === 0) return undefined
  const mapped: PenEffect[] = []

  for (const effect of effects) {
    if (effect.visible === false) continue
    const pen = mapSingleEffect(effect)
    if (pen) mapped.push(pen)
  }

  return mapped.length > 0 ? mapped : undefined
}

function mapSingleEffect(effect: FigmaEffect): PenEffect | null {
  switch (effect.type) {
    case 'DROP_SHADOW':
    case 'INNER_SHADOW': {
      return {
        type: 'shadow',
        inner: effect.type === 'INNER_SHADOW',
        offsetX: effect.offset?.x ?? 0,
        offsetY: effect.offset?.y ?? 0,
        blur: effect.radius ?? 0,
        spread: effect.spread ?? 0,
        color: effect.color
          ? figmaColorToHex(effect.color)
          : '#00000040',
      }
    }

    case 'FOREGROUND_BLUR': {
      return {
        type: 'blur',
        radius: effect.radius ?? 0,
      }
    }

    case 'BACKGROUND_BLUR': {
      return {
        type: 'background_blur',
        radius: effect.radius ?? 0,
      }
    }

    default:
      return null
  }
}
