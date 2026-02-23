/**
 * Normalize a Pencil.dev .pen document into OpenPencil's internal format.
 *
 * Handles format normalization ONLY — does NOT resolve $variable references:
 * - fill type: "color" → "solid"
 * - fill shorthand string "#hex" → [{ type: "solid", color }]
 * - gradient type: "gradient" → "linear_gradient" / "radial_gradient"
 * - gradient stops { color, position } → { offset, color }
 * - sizing "fit_content(N)" / "fill_container(N)" → fallback number
 * - padding array normalization
 *
 * Variable resolution is handled separately by `resolve-variables.ts` at
 * canvas render time, preserving $variable bindings in the document.
 */

import type { PenDocument, PenNode } from '@/types/pen'
import type { PenFill, PenStroke, GradientStop } from '@/types/styles'

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function normalizePenDocument(doc: PenDocument): PenDocument {
  return {
    ...doc,
    children: doc.children.map((n) => normalizeNode(n)),
  }
}

// ---------------------------------------------------------------------------
// Node normalizer (recursive)
// ---------------------------------------------------------------------------

function normalizeNode(node: PenNode): PenNode {
  const out: Record<string, unknown> = { ...node }

  // fill
  if ('fill' in out && out.fill !== undefined) {
    out.fill = normalizeFills(out.fill)
  }

  // stroke
  if ('stroke' in out && out.stroke != null) {
    out.stroke = normalizeStroke(out.stroke as Record<string, unknown>)
  }

  // effects — pass through (no format changes needed)

  // sizing
  if ('width' in out) out.width = normalizeSizing(out.width)
  if ('height' in out) out.height = normalizeSizing(out.height)

  // gap — pass through ($variable strings preserved)

  // padding — normalize array format only (not variable resolution)
  if ('padding' in out) out.padding = normalizePadding(out.padding)

  // opacity — pass through ($variable strings preserved)

  // children
  if ('children' in out && Array.isArray(out.children)) {
    out.children = (out.children as PenNode[]).map((c) => normalizeNode(c))
  }

  return out as unknown as PenNode
}

// ---------------------------------------------------------------------------
// Fill normalization
// ---------------------------------------------------------------------------

function normalizeFills(raw: unknown): PenFill[] {
  if (!raw) return []

  // String shorthand: "#hex" or "$variable" → solid fill
  if (typeof raw === 'string') {
    return [{ type: 'solid', color: raw }]
  }

  // Array of fills
  if (Array.isArray(raw)) {
    return raw.map((f) => normalizeSingleFill(f)).filter(Boolean) as PenFill[]
  }

  // Single fill object
  if (typeof raw === 'object') {
    const f = normalizeSingleFill(raw as Record<string, unknown>)
    return f ? [f] : []
  }

  return []
}

function normalizeSingleFill(
  raw: Record<string, unknown>,
): PenFill | null {
  if (!raw || typeof raw !== 'object') return null
  const t = raw.type as string | undefined

  // Pencil "color" → OpenPencil "solid"
  if (t === 'color' || t === 'solid') {
    return {
      type: 'solid',
      color: typeof raw.color === 'string' ? raw.color : '#000000',
    }
  }

  // Pencil "gradient" → split by gradientType
  if (t === 'gradient') {
    const gt = (raw.gradientType as string) ?? 'linear'
    const stops = normalizeGradientStops(raw.colors as unknown[])

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
        ? normalizeGradientStops(raw.stops as unknown[])
        : 'colors' in raw
          ? normalizeGradientStops(raw.colors as unknown[])
          : []
    return { ...(raw as unknown as PenFill), stops } as PenFill
  }

  // Image fill — pass through
  if (t === 'image') return raw as unknown as PenFill

  // Fallback: if there's a color field, treat as solid
  if ('color' in raw) {
    return {
      type: 'solid',
      color: typeof raw.color === 'string' ? raw.color : '#000000',
    }
  }

  return null
}

function normalizeGradientStops(
  raw: unknown[] | undefined,
): GradientStop[] {
  if (!Array.isArray(raw)) return []
  return raw.map((s: unknown) => {
    const stop = s as Record<string, unknown>
    return {
      offset:
        typeof stop.offset === 'number' && Number.isFinite(stop.offset)
          ? stop.offset
          : typeof stop.position === 'number' && Number.isFinite(stop.position)
            ? stop.position
            : 0,
      color: typeof stop.color === 'string' ? stop.color : '#000000',
    }
  })
}

// ---------------------------------------------------------------------------
// Stroke normalization
// ---------------------------------------------------------------------------

function normalizeStroke(
  raw: Record<string, unknown>,
): PenStroke | undefined {
  if (!raw) return undefined
  const out = { ...raw }

  // Normalize fill inside stroke
  if ('fill' in out) {
    out.fill = normalizeFills(out.fill)
  }

  // Pencil may use "color" directly on stroke
  if ('color' in out && typeof out.color === 'string') {
    out.fill = [{ type: 'solid', color: out.color as string }]
    delete out.color
  }

  // Thickness: leave $variable strings as-is, normalise plain number strings
  if (typeof out.thickness === 'string') {
    const str = out.thickness as string
    if (!str.startsWith('$')) {
      const num = parseFloat(str)
      out.thickness = isNaN(num) ? 1 : num
    }
  }

  return out as unknown as PenStroke
}

// ---------------------------------------------------------------------------
// Sizing normalization
// ---------------------------------------------------------------------------

function normalizeSizing(value: unknown): number | string {
  if (typeof value === 'number') return value
  if (typeof value !== 'string') return 0

  // $variable — pass through
  if (value.startsWith('$')) return value

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
): number | [number, number] | [number, number, number, number] | string | undefined {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    // $variable — pass through
    if (value.startsWith('$')) return value
    const num = parseFloat(value)
    return isNaN(num) ? 0 : num
  }
  if (Array.isArray(value)) {
    return value.map((v) => {
      if (typeof v === 'number') return v
      if (typeof v === 'string') {
        const num = parseFloat(v)
        return isNaN(num) ? 0 : num
      }
      return 0
    }) as [number, number] | [number, number, number, number]
  }
  return undefined
}
