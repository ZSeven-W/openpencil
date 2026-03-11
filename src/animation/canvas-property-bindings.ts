import type { FabricObject } from 'fabric'
import { Shadow } from 'fabric'

export interface CanvasPropertyBinding<T = unknown> {
  key: string
  apply: (obj: FabricObject, value: T) => void
  capture: (obj: FabricObject) => T
  requiresCacheInvalidation: boolean
}

const canvasPropertyBindings = new Map<string, CanvasPropertyBinding>()

export function registerCanvasBinding<T>(
  binding: CanvasPropertyBinding<T>,
): void {
  canvasPropertyBindings.set(binding.key, binding as CanvasPropertyBinding)
}

export function getCanvasBinding(
  key: string,
): CanvasPropertyBinding | undefined {
  return canvasPropertyBindings.get(key)
}

export function getAllCanvasBindings(): CanvasPropertyBinding[] {
  return Array.from(canvasPropertyBindings.values())
}

// --- Helper: ensure shadow exists ---

function ensureShadow(obj: FabricObject): Shadow {
  if (!obj.shadow || !(obj.shadow instanceof Shadow)) {
    obj.shadow = new Shadow({ color: '#000000', blur: 0, offsetX: 0, offsetY: 0 })
  }
  return obj.shadow as Shadow
}

// --- Register all 22 canvas property bindings ---

// Transform properties (no cache invalidation needed)

registerCanvasBinding<number>({
  key: 'x',
  apply: (obj, value) => {
    obj.left = value
  },
  capture: (obj) => obj.left ?? 0,
  requiresCacheInvalidation: false,
})

registerCanvasBinding<number>({
  key: 'y',
  apply: (obj, value) => {
    obj.top = value
  },
  capture: (obj) => obj.top ?? 0,
  requiresCacheInvalidation: false,
})

registerCanvasBinding<number>({
  key: 'scaleX',
  apply: (obj, value) => {
    obj.scaleX = value
  },
  capture: (obj) => obj.scaleX ?? 1,
  requiresCacheInvalidation: false,
})

registerCanvasBinding<number>({
  key: 'scaleY',
  apply: (obj, value) => {
    obj.scaleY = value
  },
  capture: (obj) => obj.scaleY ?? 1,
  requiresCacheInvalidation: false,
})

registerCanvasBinding<number>({
  key: 'rotation',
  apply: (obj, value) => {
    obj.angle = value
  },
  capture: (obj) => obj.angle ?? 0,
  requiresCacheInvalidation: false,
})

registerCanvasBinding<number>({
  key: 'opacity',
  apply: (obj, value) => {
    obj.opacity = value
  },
  capture: (obj) => obj.opacity ?? 1,
  requiresCacheInvalidation: false,
})

// Visual properties (require cache invalidation)

registerCanvasBinding<string>({
  key: 'fill.color',
  apply: (obj, value) => {
    obj.fill = value
  },
  capture: (obj) => {
    if (typeof obj.fill === 'string') return obj.fill
    return '#000000'
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<string>({
  key: 'stroke.color',
  apply: (obj, value) => {
    obj.stroke = value
  },
  capture: (obj) => {
    if (typeof obj.stroke === 'string') return obj.stroke
    return '#000000'
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<number>({
  key: 'strokeWidth',
  apply: (obj, value) => {
    obj.strokeWidth = value
  },
  capture: (obj) => obj.strokeWidth ?? 0,
  requiresCacheInvalidation: true,
})

registerCanvasBinding<number>({
  key: 'cornerRadius',
  apply: (obj, value) => {
    const target = obj as FabricObject & { rx?: number; ry?: number }
    target.rx = value
    target.ry = value
  },
  capture: (obj) => {
    const target = obj as FabricObject & { rx?: number }
    return target.rx ?? 0
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<number>({
  key: 'blur',
  apply: (obj, value) => {
    const shadow = ensureShadow(obj)
    shadow.offsetX = 0
    shadow.offsetY = 0
    shadow.blur = value
  },
  capture: (obj) => {
    if (obj.shadow && obj.shadow instanceof Shadow) {
      if (obj.shadow.offsetX === 0 && obj.shadow.offsetY === 0) {
        return obj.shadow.blur
      }
    }
    return 0
  },
  requiresCacheInvalidation: true,
})

// Shadow properties

registerCanvasBinding<number>({
  key: 'shadow.offsetX',
  apply: (obj, value) => {
    const shadow = ensureShadow(obj)
    shadow.offsetX = value
  },
  capture: (obj) => {
    if (obj.shadow && obj.shadow instanceof Shadow) return obj.shadow.offsetX
    return 0
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<number>({
  key: 'shadow.offsetY',
  apply: (obj, value) => {
    const shadow = ensureShadow(obj)
    shadow.offsetY = value
  },
  capture: (obj) => {
    if (obj.shadow && obj.shadow instanceof Shadow) return obj.shadow.offsetY
    return 0
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<number>({
  key: 'shadow.blur',
  apply: (obj, value) => {
    const shadow = ensureShadow(obj)
    shadow.blur = value
  },
  capture: (obj) => {
    if (obj.shadow && obj.shadow instanceof Shadow) return obj.shadow.blur
    return 0
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<string>({
  key: 'shadow.color',
  apply: (obj, value) => {
    const shadow = ensureShadow(obj)
    shadow.color = value
  },
  capture: (obj) => {
    if (obj.shadow && obj.shadow instanceof Shadow) return obj.shadow.color
    return '#000000'
  },
  requiresCacheInvalidation: true,
})

// Typography properties (text nodes)

registerCanvasBinding<number>({
  key: 'fontSize',
  apply: (obj, value) => {
    const textObj = obj as FabricObject & { fontSize?: number }
    textObj.fontSize = value
  },
  capture: (obj) => {
    const textObj = obj as FabricObject & { fontSize?: number }
    return textObj.fontSize ?? 16
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<number>({
  key: 'letterSpacing',
  apply: (obj, value) => {
    // Fabric.js uses charSpacing (in 1/1000 em units)
    const textObj = obj as FabricObject & { charSpacing?: number }
    textObj.charSpacing = value
  },
  capture: (obj) => {
    const textObj = obj as FabricObject & { charSpacing?: number }
    return textObj.charSpacing ?? 0
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<number>({
  key: 'lineHeight',
  apply: (obj, value) => {
    const textObj = obj as FabricObject & { lineHeight?: number }
    textObj.lineHeight = value
  },
  capture: (obj) => {
    const textObj = obj as FabricObject & { lineHeight?: number }
    return textObj.lineHeight ?? 1.2
  },
  requiresCacheInvalidation: true,
})

registerCanvasBinding<string>({
  key: 'text.fill.color',
  apply: (obj, value) => {
    obj.fill = value
  },
  capture: (obj) => {
    if (typeof obj.fill === 'string') return obj.fill
    return '#000000'
  },
  requiresCacheInvalidation: true,
})

// Video-related bindings (no-op on canvas — handled by video sync layer)

registerCanvasBinding<number>({
  key: 'sourceStart',
  apply: () => {},
  capture: () => 0,
  requiresCacheInvalidation: false,
})

registerCanvasBinding<number>({
  key: 'sourceEnd',
  apply: () => {},
  capture: () => 0,
  requiresCacheInvalidation: false,
})

registerCanvasBinding<number>({
  key: 'playbackRate',
  apply: () => {},
  capture: () => 1,
  requiresCacheInvalidation: false,
})
