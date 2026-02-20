/**
 * Variable resolution utilities.
 *
 * Resolves `$variableName` references against a VariableDefinition map,
 * optionally matching themed values to an active theme context.
 */

import type { PenNode } from '@/types/pen'
import type { PenFill, PenStroke, PenEffect } from '@/types/styles'
import type { VariableDefinition, ThemedValue } from '@/types/variables'

type Vars = Record<string, VariableDefinition>
type Theme = Record<string, string>

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Check whether a value is a `$variable` reference string. */
export function isVariableRef(value: unknown): value is string {
  return typeof value === 'string' && value.startsWith('$')
}

/** Build the default theme map (first value per axis) from PenDocument.themes. */
export function getDefaultTheme(
  themes: Record<string, string[]> | undefined,
): Theme {
  const result: Theme = {}
  if (!themes) return result
  for (const [key, values] of Object.entries(themes)) {
    if (values.length > 0) result[key] = values[0]
  }
  return result
}

// ---------------------------------------------------------------------------
// Core resolution
// ---------------------------------------------------------------------------

/** Pick the concrete value from a `ThemedValue[]` for the given theme. */
function resolveThemedValue(
  values: ThemedValue[],
  activeTheme?: Theme,
): string | number | boolean | undefined {
  if (activeTheme && Object.keys(activeTheme).length > 0) {
    const match = values.find((v) => {
      if (!v.theme) return false
      return Object.entries(activeTheme).every(
        ([key, expected]) => v.theme?.[key] === expected,
      )
    })
    if (match) return match.value
  }
  return values[0]?.value
}

/**
 * Resolve a single `$variableName` reference to its concrete value.
 * Returns `undefined` if the variable does not exist or has an incompatible type.
 */
export function resolveVariableRef(
  ref: string,
  variables: Vars,
  activeTheme?: Theme,
): string | number | boolean | undefined {
  if (!ref.startsWith('$')) return undefined
  const name = ref.slice(1)
  const def = variables[name]
  if (!def) return undefined

  const val = def.value
  if (Array.isArray(val)) {
    const resolved = resolveThemedValue(val, activeTheme)
    // Circular guard: if resolved value is also a $ref, stop
    if (typeof resolved === 'string' && resolved.startsWith('$')) return undefined
    return resolved
  }
  // Circular guard
  if (typeof val === 'string' && val.startsWith('$')) return undefined
  return val
}

/**
 * Resolve a color string that may be a `$variable` reference.
 * Returns the original string if it's not a ref, or the resolved color.
 */
export function resolveColorRef(
  color: string | undefined,
  variables: Vars,
  activeTheme?: Theme,
): string | undefined {
  if (color === undefined) return undefined
  if (!isVariableRef(color)) return color
  const resolved = resolveVariableRef(color, variables, activeTheme)
  return typeof resolved === 'string' ? resolved : undefined
}

/**
 * Resolve a numeric value that may be a `$variable` reference.
 * Returns the original number if it's not a ref.
 */
export function resolveNumericRef(
  value: unknown,
  variables: Vars,
  activeTheme?: Theme,
): number | undefined {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    if (isVariableRef(value)) {
      const resolved = resolveVariableRef(value, variables, activeTheme)
      return typeof resolved === 'number' ? resolved : undefined
    }
    const num = parseFloat(value)
    return isNaN(num) ? undefined : num
  }
  return undefined
}

// ---------------------------------------------------------------------------
// Fill / stroke / effect resolution
// ---------------------------------------------------------------------------

function resolveFillsForCanvas(
  fills: PenFill[] | undefined,
  vars: Vars,
  theme?: Theme,
): PenFill[] | undefined {
  if (!fills) return fills
  return fills.map((fill) => {
    if (fill.type === 'solid') {
      const color = resolveColorRef(fill.color, vars, theme)
      return color !== fill.color ? { ...fill, color: color ?? '#000000' } : fill
    }
    if (fill.type === 'linear_gradient' || fill.type === 'radial_gradient') {
      const newStops = fill.stops.map((stop) => {
        const color = resolveColorRef(stop.color, vars, theme)
        return color !== stop.color ? { ...stop, color: color ?? '#000000' } : stop
      })
      return newStops !== fill.stops ? { ...fill, stops: newStops } : fill
    }
    return fill
  })
}

function resolveStrokeForCanvas(
  stroke: PenStroke | undefined,
  vars: Vars,
  theme?: Theme,
): PenStroke | undefined {
  if (!stroke) return stroke
  let changed = false
  const out: Record<string, unknown> = { ...stroke }

  // Resolve thickness
  if (typeof stroke.thickness === 'string' && isVariableRef(stroke.thickness)) {
    out.thickness = resolveNumericRef(stroke.thickness, vars, theme) ?? 1
    changed = true
  }

  // Resolve stroke fill colors
  if (stroke.fill) {
    const resolved = resolveFillsForCanvas(stroke.fill, vars, theme)
    if (resolved !== stroke.fill) {
      out.fill = resolved
      changed = true
    }
  }

  return changed ? (out as unknown as PenStroke) : stroke
}

function resolveEffectsForCanvas(
  effects: PenEffect[] | undefined,
  vars: Vars,
  theme?: Theme,
): PenEffect[] | undefined {
  if (!effects) return effects
  return effects.map((effect) => {
    if (effect.type !== 'shadow') return effect
    let changed = false
    const out: Record<string, unknown> = { ...effect }

    if (typeof effect.color === 'string' && isVariableRef(effect.color)) {
      out.color = resolveColorRef(effect.color, vars, theme) ?? '#000000'
      changed = true
    }
    for (const key of ['blur', 'offsetX', 'offsetY', 'spread'] as const) {
      const val = effect[key]
      if (typeof val === 'string' && isVariableRef(val)) {
        out[key] = resolveNumericRef(val, vars, theme) ?? 0
        changed = true
      }
    }

    return changed ? (out as unknown as PenEffect) : effect
  })
}

// ---------------------------------------------------------------------------
// Full node resolution for canvas rendering
// ---------------------------------------------------------------------------

/**
 * Resolve all `$variable` references in a PenNode, returning a new node
 * with concrete values suitable for Fabric.js rendering.
 *
 * Returns the same object reference when no variables are present.
 */
export function resolveNodeForCanvas(
  node: PenNode,
  variables: Vars,
  activeTheme?: Theme,
): PenNode {
  if (!variables || Object.keys(variables).length === 0) return node

  let changed = false
  const out: Record<string, unknown> = { ...node }

  // Opacity
  if (typeof node.opacity === 'string' && isVariableRef(node.opacity)) {
    out.opacity = resolveNumericRef(node.opacity, variables, activeTheme) ?? 1
    changed = true
  }

  // Gap
  if ('gap' in node && typeof (node as unknown as Record<string, unknown>).gap === 'string') {
    const gap = (node as unknown as Record<string, unknown>).gap as string
    if (isVariableRef(gap)) {
      out.gap = resolveNumericRef(gap, variables, activeTheme) ?? 0
      changed = true
    }
  }

  // Padding
  if ('padding' in node) {
    const padding = (node as unknown as Record<string, unknown>).padding
    if (typeof padding === 'string' && isVariableRef(padding)) {
      out.padding = resolveNumericRef(padding, variables, activeTheme) ?? 0
      changed = true
    }
  }

  // Fill
  if ('fill' in node && (node as unknown as Record<string, unknown>).fill) {
    const fills = (node as unknown as Record<string, unknown>).fill as PenFill[]
    const resolved = resolveFillsForCanvas(fills, variables, activeTheme)
    if (resolved !== fills) {
      out.fill = resolved
      changed = true
    }
  }

  // Stroke
  if ('stroke' in node && (node as unknown as Record<string, unknown>).stroke) {
    const stroke = (node as unknown as Record<string, unknown>).stroke as PenStroke
    const resolved = resolveStrokeForCanvas(stroke, variables, activeTheme)
    if (resolved !== stroke) {
      out.stroke = resolved
      changed = true
    }
  }

  // Effects
  if ('effects' in node && (node as unknown as Record<string, unknown>).effects) {
    const effects = (node as unknown as Record<string, unknown>).effects as PenEffect[]
    const resolved = resolveEffectsForCanvas(effects, variables, activeTheme)
    if (resolved !== effects) {
      out.effects = resolved
      changed = true
    }
  }

  // Text content
  if (node.type === 'text' && typeof node.content === 'string' && isVariableRef(node.content)) {
    const resolved = resolveVariableRef(node.content, variables, activeTheme)
    if (typeof resolved === 'string') {
      out.content = resolved
      changed = true
    }
  }

  return changed ? (out as unknown as PenNode) : node
}
