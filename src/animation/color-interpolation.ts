import type { HexColor } from '@/types/animation'

export function parseHex(hex: string): [number, number, number] | null {
  const m = hex.match(/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i)
  if (m) return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)]
  const m3 = hex.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/i)
  if (m3)
    return [
      parseInt(m3[1] + m3[1], 16),
      parseInt(m3[2] + m3[2], 16),
      parseInt(m3[3] + m3[3], 16),
    ]
  return null
}

export function formatHex(r: number, g: number, b: number): HexColor {
  const c = (v: number) => Math.max(0, Math.min(255, Math.round(v)))
  return `#${c(r).toString(16).padStart(2, '0')}${c(g).toString(16).padStart(2, '0')}${c(b).toString(16).padStart(2, '0')}` as HexColor
}

export function srgbLerp(from: string, to: string, t: number): string {
  const a = parseHex(from)
  const b = parseHex(to)
  if (!a || !b) return t < 0.5 ? from : to
  return formatHex(
    a[0] + (b[0] - a[0]) * t,
    a[1] + (b[1] - a[1]) * t,
    a[2] + (b[2] - a[2]) * t,
  )
}
