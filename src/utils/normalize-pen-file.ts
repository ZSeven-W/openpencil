/**
 * Normalize a Pencil.dev .pen document into OpenPencil's internal format.
 *
 * Handles:
 * - fill type: "color" → "solid"
 * - fill shorthand string "#hex" → [{ type: "solid", color }]
 * - gradient type: "gradient" → "linear_gradient" / "radial_gradient"
 * - gradient stops { color, position } → { offset, color }
 * - $variable references → resolved values (first/default theme)
 * - sizing "fit_content(N)" / "fill_container(N)" → fallback number
 */

import type { PenDocument, PenNode } from '@/types/pen'
import type { PenFill, PenStroke, PenEffect, GradientStop } from '@/types/styles'
import type { VariableDefinition } from '@/types/variables'

type Vars = Record<string, VariableDefinition>

// Module-level default theme map, set per normalizePenDocument call
let _defaultTheme: Record<string, string> = {}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function normalizePenDocument(doc: PenDocument): PenDocument {
  const vars: Vars = doc.variables ?? {}
  // Build default theme: first entry of each theme collection
  _defaultTheme = {}
  if (doc.themes) {
    for (const [key, values] of Object.entries(doc.themes)) {
      if (values.length > 0) _defaultTheme[key] = values[0]
    }
  }
  return {
    ...doc,
    children: doc.children.map((n) => normalizeNode(n, vars)),
  }
}

// ---------------------------------------------------------------------------
// Node normalizer (recursive)
// ---------------------------------------------------------------------------

function normalizeNode(node: PenNode, vars: Vars): PenNode {
  const out: Record<string, unknown> = { ...node }

  // fill
  if ('fill' in out && out.fill !== undefined) {
    out.fill = normalizeFills(out.fill, vars)
  }

  // stroke
  if ('stroke' in out && out.stroke != null) {
    out.stroke = normalizeStroke(out.stroke as Record<string, unknown>, vars)
  }

  // effects
  if ('effects' in out && Array.isArray(out.effects)) {
    out.effects = normalizeEffects(out.effects as Record<string, unknown>[], vars)
  }

  // sizing
  if ('width' in out) out.width = normalizeSizing(out.width)
  if ('height' in out) out.height = normalizeSizing(out.height)

  // gap / padding may be variable refs
  if ('gap' in out) out.gap = resolveNumeric(out.gap, vars)
  if ('padding' in out) out.padding = normalizePadding(out.padding, vars)

  // opacity
  if ('opacity' in out) out.opacity = resolveNumeric(out.opacity, vars) ?? 1

  // text content — resolve variable in content if it's a $ref
  if (out.type === 'text' && typeof out.content === 'string' && (out.content as string).startsWith('$')) {
    const resolved = resolveVar(out.content as string, vars)
    if (typeof resolved === 'string') out.content = resolved
  }

  // children
  if ('children' in out && Array.isArray(out.children)) {
    out.children = (out.children as PenNode[]).map((c) => normalizeNode(c, vars))
  }

  return out as unknown as PenNode
}

// ---------------------------------------------------------------------------
// Fill normalization
// ---------------------------------------------------------------------------

function normalizeFills(raw: unknown, vars: Vars): PenFill[] {
  if (!raw) return []

  // String shorthand: "#hex" or "$variable"
  if (typeof raw === 'string') {
    const color = resolveColor(raw, vars)
    return color ? [{ type: 'solid', color }] : []
  }

  // Array of fills
  if (Array.isArray(raw)) {
    return raw.map((f) => normalizeSingleFill(f, vars)).filter(Boolean) as PenFill[]
  }

  // Single fill object
  if (typeof raw === 'object') {
    const f = normalizeSingleFill(raw as Record<string, unknown>, vars)
    return f ? [f] : []
  }

  return []
}

function normalizeSingleFill(
  raw: Record<string, unknown>,
  vars: Vars,
): PenFill | null {
  if (!raw || typeof raw !== 'object') return null
  const t = raw.type as string | undefined

  // Pencil "color" → OpenPencil "solid"
  if (t === 'color' || t === 'solid') {
    return {
      type: 'solid',
      color: resolveColor(raw.color, vars) ?? '#000000',
    }
  }

  // Pencil "gradient" → split by gradientType
  if (t === 'gradient') {
    const gt = (raw.gradientType as string) ?? 'linear'
    const stops = normalizeGradientStops(raw.colors as unknown[], vars)

    if (gt === 'radial') {
      const center = raw.center as Record<string, unknown> | undefined
      return {
        type: 'radial_gradient',
        cx: typeof center?.x === 'number' ? center.x : 0.5,
        cy: typeof center?.y === 'number' ? center.y : 0.5,
        radius: 0.5,
        stops,
      }
    }
    // linear or angular
    return {
      type: 'linear_gradient',
      angle: typeof raw.rotation === 'number' ? raw.rotation : 0,
      stops,
    }
  }

  // Already our format
  if (t === 'linear_gradient' || t === 'radial_gradient') {
    const stops =
      'stops' in raw
        ? normalizeGradientStops(raw.stops as unknown[], vars)
        : 'colors' in raw
          ? normalizeGradientStops(raw.colors as unknown[], vars)
          : []
    return { ...(raw as unknown as PenFill), stops } as PenFill
  }

  // Image fill — pass through
  if (t === 'image') return raw as unknown as PenFill

  // Fallback: if there's a color field, treat as solid
  if ('color' in raw) {
    return {
      type: 'solid',
      color: resolveColor(raw.color, vars) ?? '#000000',
    }
  }

  return null
}

function normalizeGradientStops(
  raw: unknown[] | undefined,
  vars: Vars,
): GradientStop[] {
  if (!Array.isArray(raw)) return []
  return raw.map((s: unknown) => {
    const stop = s as Record<string, unknown>
    return {
      offset:
        typeof stop.offset === 'number'
          ? stop.offset
          : typeof stop.position === 'number'
            ? stop.position
            : 0,
      color: resolveColor(stop.color, vars) ?? '#000000',
    }
  })
}

// ---------------------------------------------------------------------------
// Stroke normalization
// ---------------------------------------------------------------------------

function normalizeStroke(
  raw: Record<string, unknown>,
  vars: Vars,
): PenStroke | undefined {
  if (!raw) return undefined
  const out = { ...raw }

  // Normalize fill inside stroke
  if ('fill' in out) {
    out.fill = normalizeFills(out.fill, vars)
  }

  // Pencil may use "color" directly on stroke
  if ('color' in out && typeof out.color === 'string') {
    out.fill = [{ type: 'solid', color: resolveColor(out.color, vars) ?? '#000000' }]
    delete out.color
  }

  // Normalize thickness variable ref
  if (typeof out.thickness === 'string') {
    out.thickness = resolveNumeric(out.thickness, vars) ?? 1
  }

  return out as unknown as PenStroke
}

// ---------------------------------------------------------------------------
// Effects normalization
// ---------------------------------------------------------------------------

function normalizeEffects(
  raw: Record<string, unknown>[],
  vars: Vars,
): PenEffect[] {
  return raw.map((e) => {
    const out = { ...e }
    if (typeof out.color === 'string') {
      out.color = resolveColor(out.color, vars) ?? '#000000'
    }
    if (typeof out.blur === 'string') out.blur = resolveNumeric(out.blur, vars) ?? 0
    if (typeof out.offsetX === 'string') out.offsetX = resolveNumeric(out.offsetX, vars) ?? 0
    if (typeof out.offsetY === 'string') out.offsetY = resolveNumeric(out.offsetY, vars) ?? 0
    if (typeof out.spread === 'string') out.spread = resolveNumeric(out.spread, vars) ?? 0
    return out as unknown as PenEffect
  })
}

// ---------------------------------------------------------------------------
// Sizing normalization
// ---------------------------------------------------------------------------

function normalizeSizing(value: unknown): number | string {
  if (typeof value === 'number') return value
  if (typeof value !== 'string') return 0

  // fill_container must always resolve dynamically from parent dimensions
  if (value.startsWith('fill_container')) return 'fill_container'

  // fit_content with a hint value: use the hint (more accurate than our estimation)
  if (value.startsWith('fit_content')) {
    const match = value.match(/\((\d+(?:\.\d+)?)\)/)
    if (match) return parseFloat(match[1])
    return 'fit_content'
  }

  // Try as a plain number string
  const num = parseFloat(value)
  return isNaN(num) ? 0 : num
}

function normalizePadding(
  value: unknown,
  vars: Vars,
): number | [number, number] | [number, number, number, number] | undefined {
  if (typeof value === 'number') return value
  if (typeof value === 'string') return (resolveNumeric(value, vars) as number) ?? 0
  if (Array.isArray(value)) {
    return value.map((v) =>
      typeof v === 'number' ? v : (resolveNumeric(v, vars) as number) ?? 0,
    ) as [number, number] | [number, number, number, number]
  }
  return undefined
}

// ---------------------------------------------------------------------------
// Variable resolution
// ---------------------------------------------------------------------------

function resolveVar(ref: string, vars: Vars): unknown {
  if (!ref.startsWith('$')) return ref
  const name = ref.slice(1)
  const def = vars[name]
  if (!def) return ref

  const val = def.value
  if (Array.isArray(val)) {
    // Try to find value matching the default theme (first entry per collection)
    if (Object.keys(_defaultTheme).length > 0) {
      const matching = val.find((v) => {
        if (!v.theme) return false
        return Object.entries(_defaultTheme).every(
          ([key, expected]) => v.theme?.[key] === expected,
        )
      })
      if (matching) return matching.value
    }
    // Fallback to first value
    return val[0]?.value ?? ref
  }
  return val
}

function resolveColor(raw: unknown, vars: Vars): string | null {
  if (typeof raw !== 'string') return null
  if (raw.startsWith('$')) {
    const resolved = resolveVar(raw, vars)
    return typeof resolved === 'string' ? resolved : '#000000'
  }
  return raw
}

function resolveNumeric(raw: unknown, vars: Vars): number | undefined {
  if (typeof raw === 'number') return raw
  if (typeof raw === 'string') {
    if (raw.startsWith('$')) {
      const resolved = resolveVar(raw, vars)
      return typeof resolved === 'number' ? resolved : undefined
    }
    const num = parseFloat(raw)
    return isNaN(num) ? undefined : num
  }
  return undefined
}
