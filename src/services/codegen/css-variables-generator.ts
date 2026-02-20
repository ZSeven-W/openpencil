/**
 * Generates CSS custom properties from PenDocument variables.
 *
 * Produces `:root { ... }` blocks with one block per theme variant.
 */

import type { PenDocument } from '@/types/pen'
import type { VariableDefinition, ThemedValue } from '@/types/variables'

/** Sanitise a variable name into a valid CSS custom property name. */
export function variableNameToCSS(name: string): string {
  const sanitised = name
    .replace(/^\$/, '')
    .replace(/\s+/g, '-')
    .replace(/[^a-zA-Z0-9_-]/g, '')
    .toLowerCase()
  return `--${sanitised}`
}

/** Whether a numeric variable should be output without a unit (e.g. opacity). */
function isUnitless(name: string): boolean {
  const lower = name.toLowerCase()
  return (
    lower.includes('opacity') ||
    lower.includes('weight') ||
    lower.includes('scale') ||
    lower.includes('ratio') ||
    lower.includes('z-index') ||
    lower.includes('line-height')
  )
}

/** Format a variable value as a CSS value string. */
function formatValue(
  value: string | number | boolean,
  name: string,
  type: VariableDefinition['type'],
): string | null {
  if (type === 'boolean') return null
  if (type === 'color') return String(value)
  if (type === 'number') {
    if (typeof value !== 'number') return String(value)
    return isUnitless(name) ? String(value) : `${value}px`
  }
  // string
  return String(value)
}

/** Resolve a single themed value for a given theme context. */
function resolveForTheme(
  def: VariableDefinition,
  theme: Record<string, string>,
): string | number | boolean | undefined {
  const val = def.value
  if (!Array.isArray(val)) return val
  const match = (val as ThemedValue[]).find((v) => {
    if (!v.theme) return false
    return Object.entries(theme).every(
      ([key, expected]) => v.theme?.[key] === expected,
    )
  })
  return match?.value ?? (val as ThemedValue[])[0]?.value
}

/**
 * Generate CSS custom properties from a PenDocument's variables and themes.
 *
 * Returns a string containing `:root { ... }` blocks.
 */
export function generateCSSVariables(doc: PenDocument): string {
  const variables = doc.variables
  if (!variables || Object.keys(variables).length === 0) {
    return '/* No design variables defined */\n'
  }

  const themes = doc.themes ?? {}
  const themeAxes = Object.entries(themes)

  // Build default theme (first value per axis)
  const defaultTheme: Record<string, string> = {}
  for (const [key, values] of themeAxes) {
    if (values.length > 0) defaultTheme[key] = values[0]
  }

  const hasThemes = themeAxes.length > 0 && themeAxes.some(([, v]) => v.length > 1)

  // Generate default :root block
  const lines: string[] = []
  lines.push(':root {')

  const varEntries = Object.entries(variables).sort(([a], [b]) => a.localeCompare(b))
  for (const [name, def] of varEntries) {
    const value = Array.isArray(def.value)
      ? resolveForTheme(def, defaultTheme)
      : def.value
    if (value === undefined) continue
    const css = formatValue(value, name, def.type)
    if (css === null) continue
    lines.push(`  ${variableNameToCSS(name)}: ${css};`)
  }

  lines.push('}')

  // Generate per-theme variant blocks
  if (hasThemes) {
    // For simplicity, iterate over each axis independently
    // (e.g. mode: light/dark generates :root[data-theme="dark"] { ... })
    for (const [axis, values] of themeAxes) {
      // Skip the default (first) value
      for (let i = 1; i < values.length; i++) {
        const themeValue = values[i]
        const themeContext = { ...defaultTheme, [axis]: themeValue }

        const block: string[] = []
        for (const [name, def] of varEntries) {
          if (!Array.isArray(def.value)) continue
          const resolvedForThis = resolveForTheme(def, themeContext)
          const resolvedForDefault = resolveForTheme(def, defaultTheme)
          // Only include if different from default
          if (resolvedForThis === resolvedForDefault) continue
          if (resolvedForThis === undefined) continue
          const css = formatValue(resolvedForThis, name, def.type)
          if (css === null) continue
          block.push(`  ${variableNameToCSS(name)}: ${css};`)
        }

        if (block.length > 0) {
          lines.push('')
          lines.push(`:root[data-theme="${themeValue}"] {`)
          lines.push(...block)
          lines.push('}')
        }
      }
    }
  }

  return lines.join('\n') + '\n'
}
