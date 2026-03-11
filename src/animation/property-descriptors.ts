import type { PenNodeType } from '@/types/pen'
import { parseHex, formatHex, srgbLerp } from './color-interpolation'
export { parseHex, formatHex, srgbLerp }

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
