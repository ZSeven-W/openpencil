import type { FigmaColor } from './figma-types'

/**
 * Convert Figma {r, g, b, a} (0-1 floats) to #RRGGBB or #RRGGBBAA hex string.
 */
export function figmaColorToHex(color: FigmaColor): string {
  const r = Math.round(color.r * 255)
  const g = Math.round(color.g * 255)
  const b = Math.round(color.b * 255)
  const hex = `#${toHex(r)}${toHex(g)}${toHex(b)}`

  if (color.a !== undefined && color.a < 1) {
    const a = Math.round(color.a * 255)
    return `${hex}${toHex(a)}`
  }
  return hex
}

function toHex(n: number): string {
  return n.toString(16).padStart(2, '0')
}

/**
 * Extract opacity from a Figma color's alpha channel (0-1).
 */
export function figmaColorOpacity(color: FigmaColor): number {
  return color.a ?? 1
}
