import type { CanvasKit } from 'canvaskit-wasm'
import type { PenFill, PenStroke } from '@/types/styles'
import { DEFAULT_FILL, DEFAULT_STROKE_WIDTH } from '../canvas-constants'

// ---------------------------------------------------------------------------
// Color parsing — ck.Color4f takes 0-1 floats for all channels (r, g, b, a)
// ---------------------------------------------------------------------------

export function parseColor(ck: CanvasKit, color: string): Float32Array {
  if (color.startsWith('#')) {
    const hex = color.slice(1)
    if (hex.length === 8) {
      const r = parseInt(hex.slice(0, 2), 16) / 255
      const g = parseInt(hex.slice(2, 4), 16) / 255
      const b = parseInt(hex.slice(4, 6), 16) / 255
      const a = parseInt(hex.slice(6, 8), 16) / 255
      return ck.Color4f(r, g, b, a)
    }
    if (hex.length === 6) {
      const r = parseInt(hex.slice(0, 2), 16) / 255
      const g = parseInt(hex.slice(2, 4), 16) / 255
      const b = parseInt(hex.slice(4, 6), 16) / 255
      return ck.Color4f(r, g, b, 1)
    }
    if (hex.length === 3) {
      const r = parseInt(hex[0] + hex[0], 16) / 255
      const g = parseInt(hex[1] + hex[1], 16) / 255
      const b = parseInt(hex[2] + hex[2], 16) / 255
      return ck.Color4f(r, g, b, 1)
    }
  }
  if (color === 'transparent') return ck.Color4f(0, 0, 0, 0)
  if (color === 'white') return ck.Color4f(1, 1, 1, 1)
  if (color === 'black') return ck.Color4f(0, 0, 0, 1)
  // rgba() parsing
  const rgbaMatch = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/)
  if (rgbaMatch) {
    return ck.Color4f(
      parseInt(rgbaMatch[1]) / 255,
      parseInt(rgbaMatch[2]) / 255,
      parseInt(rgbaMatch[3]) / 255,
      rgbaMatch[4] !== undefined ? parseFloat(rgbaMatch[4]) : 1,
    )
  }
  return ck.Color4f(0.82, 0.835, 0.858, 1) // fallback #d1d5db
}

// ---------------------------------------------------------------------------
// Corner radius helpers
// ---------------------------------------------------------------------------

export function cornerRadiusValue(cr: number | [number, number, number, number] | undefined): number {
  if (cr === undefined) return 0
  if (typeof cr === 'number') return cr
  return cr[0]
}

export function cornerRadii(cr: number | [number, number, number, number] | undefined): [number, number, number, number] {
  if (cr === undefined) return [0, 0, 0, 0]
  if (typeof cr === 'number') return [cr, cr, cr, cr]
  return cr
}

// ---------------------------------------------------------------------------
// Fill / stroke helpers
// ---------------------------------------------------------------------------

export function resolveFillColor(fills?: PenFill[] | string): string {
  if (typeof fills === 'string') return fills
  if (!fills || fills.length === 0) return DEFAULT_FILL
  const first = fills[0]
  if (first.type === 'solid') return first.color
  if (first.type === 'linear_gradient' || first.type === 'radial_gradient') {
    return first.stops[0]?.color ?? DEFAULT_FILL
  }
  return DEFAULT_FILL
}

export function resolveStrokeColor(stroke?: PenStroke): string | undefined {
  if (!stroke) return undefined
  if (typeof stroke.fill === 'string') return stroke.fill
  if (stroke.fill && stroke.fill.length > 0) return resolveFillColor(stroke.fill)
  return undefined
}

export function resolveStrokeWidth(stroke?: PenStroke): number {
  if (!stroke) return 0
  if (typeof stroke.thickness === 'number') return stroke.thickness
  if (typeof stroke.thickness === 'object' && !Array.isArray(stroke.thickness)) return 0
  return stroke.thickness?.[0] ?? DEFAULT_STROKE_WIDTH
}

// ---------------------------------------------------------------------------
// Text wrapping utilities
// ---------------------------------------------------------------------------

/** CJK character range check (for character-level line breaking). */
function isCJK(ch: string): boolean {
  const c = ch.charCodeAt(0)
  return (c >= 0x4E00 && c <= 0x9FFF) || (c >= 0x3400 && c <= 0x4DBF) ||
    (c >= 0x3000 && c <= 0x303F) || (c >= 0xFF00 && c <= 0xFFEF) ||
    (c >= 0x2E80 && c <= 0x2FDF)
}

/** Word-wrap a single line of text, appending wrapped lines to `out`. */
export function wrapLine(ctx: CanvasRenderingContext2D, text: string, maxW: number, out: string[]) {
  if (ctx.measureText(text).width <= maxW) { out.push(text); return }

  let current = ''
  let i = 0
  while (i < text.length) {
    const ch = text[i]
    if (isCJK(ch)) {
      const test = current + ch
      if (ctx.measureText(test).width > maxW && current) {
        out.push(current)
        current = ch
      } else {
        current = test
      }
      i++
    } else if (ch === ' ') {
      const test = current + ch
      if (ctx.measureText(test).width > maxW && current) {
        out.push(current)
        current = ''
      } else {
        current = test
      }
      i++
    } else {
      let word = ''
      while (i < text.length && text[i] !== ' ' && !isCJK(text[i])) {
        word += text[i]; i++
      }
      const test = current + word
      if (ctx.measureText(test).width > maxW && current) {
        out.push(current)
        current = word
      } else {
        current = test
      }
    }
  }
  if (current) out.push(current)
}

/** CSS generic font families that must NOT be quoted */
const GENERIC_FAMILIES = new Set([
  'serif', 'sans-serif', 'monospace', 'cursive', 'fantasy', 'system-ui',
  'ui-serif', 'ui-sans-serif', 'ui-monospace', 'ui-rounded',
  '-apple-system', 'blinkmacsystemfont',
])

/**
 * Quote font family names for CSS `font` shorthand.
 * Names with spaces MUST be quoted, otherwise Canvas 2D parses each word
 * as a separate family (e.g. "Bodoni 72 Smallcaps" → "Bodoni", "72", "Smallcaps").
 */
export function cssFontFamily(family: string): string {
  return family.split(',').map(f => {
    const trimmed = f.trim()
    if (!trimmed) return trimmed
    // Already quoted
    if ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))) return trimmed
    // Generic families must not be quoted
    if (GENERIC_FAMILIES.has(trimmed.toLowerCase())) return trimmed
    // Quote everything else (safe even for single-word names)
    return `"${trimmed}"`
  }).join(', ')
}
