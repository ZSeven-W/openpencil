import type { PenNodeType } from '@/types/pen'

export interface AnimatablePropertyDescriptor<T = unknown> {
  key: string
  type: 'number' | 'color'
  default: T
  interpolate: (from: T, to: T, t: number) => T
  nodeTypes?: PenNodeType[]
}

const propertyDescriptors = new Map<string, AnimatablePropertyDescriptor>()

export function registerPropertyDescriptor<T>(
  desc: AnimatablePropertyDescriptor<T>,
): void {
  propertyDescriptors.set(desc.key, desc as AnimatablePropertyDescriptor)
}

export function getPropertyDescriptor(
  key: string,
): AnimatablePropertyDescriptor | undefined {
  return propertyDescriptors.get(key)
}

export function getAllPropertyDescriptors(): AnimatablePropertyDescriptor[] {
  return Array.from(propertyDescriptors.values())
}

// --- Interpolation helpers ---

export function lerp(from: number, to: number, t: number): number {
  return from + (to - from) * t
}

export function srgbLerp(from: string, to: string, t: number): string {
  const fromRgb = parseHex(from)
  const toRgb = parseHex(to)
  if (!fromRgb || !toRgb) return t < 0.5 ? from : to
  const r = Math.round(lerp(fromRgb[0], toRgb[0], t))
  const g = Math.round(lerp(fromRgb[1], toRgb[1], t))
  const b = Math.round(lerp(fromRgb[2], toRgb[2], t))
  return formatHex(r, g, b)
}

function parseHex(hex: string): [number, number, number] | null {
  const m = hex.match(/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i)
  if (m) {
    return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)]
  }
  const m3 = hex.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/i)
  if (m3) {
    return [
      parseInt(m3[1] + m3[1], 16),
      parseInt(m3[2] + m3[2], 16),
      parseInt(m3[3] + m3[3], 16),
    ]
  }
  return null
}

function formatHex(r: number, g: number, b: number): string {
  const clamp = (v: number) => Math.max(0, Math.min(255, v))
  return `#${clamp(r).toString(16).padStart(2, '0')}${clamp(g).toString(16).padStart(2, '0')}${clamp(b).toString(16).padStart(2, '0')}`
}

// --- Register all 22 animatable properties ---

// Transform properties (no nodeTypes restriction)
registerPropertyDescriptor({
  key: 'x',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'y',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'scaleX',
  type: 'number',
  default: 1,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'scaleY',
  type: 'number',
  default: 1,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'rotation',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'opacity',
  type: 'number',
  default: 1,
  interpolate: lerp,
})

// Visual properties
registerPropertyDescriptor({
  key: 'fill.color',
  type: 'color',
  default: '#000000',
  interpolate: srgbLerp,
})

registerPropertyDescriptor({
  key: 'stroke.color',
  type: 'color',
  default: '#000000',
  interpolate: srgbLerp,
})

registerPropertyDescriptor({
  key: 'strokeWidth',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'cornerRadius',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'blur',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

// Shadow properties
registerPropertyDescriptor({
  key: 'shadow.offsetX',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'shadow.offsetY',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'shadow.blur',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'shadow.color',
  type: 'color',
  default: '#000000',
  interpolate: srgbLerp,
})

// Typography properties (text nodes only)
registerPropertyDescriptor({
  key: 'fontSize',
  type: 'number',
  default: 16,
  interpolate: lerp,
  nodeTypes: ['text'],
})

registerPropertyDescriptor({
  key: 'letterSpacing',
  type: 'number',
  default: 0,
  interpolate: lerp,
  nodeTypes: ['text'],
})

registerPropertyDescriptor({
  key: 'lineHeight',
  type: 'number',
  default: 1.2,
  interpolate: lerp,
  nodeTypes: ['text'],
})

registerPropertyDescriptor({
  key: 'text.fill.color',
  type: 'color',
  default: '#000000',
  interpolate: srgbLerp,
  nodeTypes: ['text'],
})

// Video-related (number types for scrubbing)
registerPropertyDescriptor({
  key: 'sourceStart',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'sourceEnd',
  type: 'number',
  default: 0,
  interpolate: lerp,
})

registerPropertyDescriptor({
  key: 'playbackRate',
  type: 'number',
  default: 1,
  interpolate: lerp,
})
