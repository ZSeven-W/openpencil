/**
 * CSS token extraction from HTML/CSS text.
 * Pure utility — no framework dependencies.
 */

import type { VariableDefinition } from '../../src/types/variables'

export interface ExtractedTokens {
  colors: Record<string, string>
  fonts: string[]
  spacing: number[]
  borderRadius: number[]
  fontSizes: number[]
}

function emptyTokens(): ExtractedTokens {
  return { colors: {}, fonts: [], spacing: [], borderRadius: [], fontSizes: [] }
}

function mergeTokens(a: ExtractedTokens, b: ExtractedTokens): ExtractedTokens {
  return {
    colors: { ...a.colors, ...b.colors },
    fonts: Array.from(new Set([...a.fonts, ...b.fonts])),
    spacing: Array.from(new Set([...a.spacing, ...b.spacing])).sort((x, y) => x - y),
    borderRadius: Array.from(new Set([...a.borderRadius, ...b.borderRadius])).sort((x, y) => x - y),
    fontSizes: Array.from(new Set([...a.fontSizes, ...b.fontSizes])).sort((x, y) => x - y),
  }
}

// ── CSS custom property extraction ────────────────────────────────────

const CSS_VAR_RE = /--([\w-]+)\s*:\s*([^;]+)/g
const HEX_RE = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i
const RGB_RE = /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/
const HSL_RE = /^hsla?\(\s*(\d+)\s*,\s*([\d.]+)%?\s*,\s*([\d.]+)%?/

function normalizeColor(raw: string): string | null {
  const v = raw.trim()
  if (HEX_RE.test(v)) return v.toLowerCase()
  const rgb = v.match(RGB_RE)
  if (rgb) {
    const hex = [rgb[1], rgb[2], rgb[3]]
      .map((c) => parseInt(c).toString(16).padStart(2, '0'))
      .join('')
    return `#${hex}`
  }
  const hsl = v.match(HSL_RE)
  if (hsl) return hslToHex(parseInt(hsl[1]), parseFloat(hsl[2]), parseFloat(hsl[3]))
  // Named CSS colors — skip, too many edge cases
  return null
}

function hslToHex(h: number, s: number, l: number): string {
  s /= 100
  l /= 100
  const a = s * Math.min(l, 1 - l)
  const f = (n: number) => {
    const k = (n + h / 30) % 12
    const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1)
    return Math.round(255 * color)
      .toString(16)
      .padStart(2, '0')
  }
  return `#${f(0)}${f(8)}${f(4)}`
}

function extractCSSCustomProperties(css: string): Record<string, string> {
  const result: Record<string, string> = {}
  let match: RegExpExecArray | null
  const re = new RegExp(CSS_VAR_RE.source, 'g')
  while ((match = re.exec(css)) !== null) {
    const name = match[1]
    const hex = normalizeColor(match[2])
    if (hex) result[name] = hex
  }
  return result
}

// ── Font extraction ───────────────────────────────────────────────────

const FONT_FAMILY_RE = /font-family\s*:\s*([^;}"]+)/gi

function extractFonts(css: string): string[] {
  const fonts = new Set<string>()
  let match: RegExpExecArray | null
  const re = new RegExp(FONT_FAMILY_RE.source, 'gi')
  while ((match = re.exec(css)) !== null) {
    const stack = match[1].trim().replace(/['"]/g, '')
    if (stack && !stack.startsWith('var(')) fonts.add(stack)
  }
  return Array.from(fonts)
}

// ── Numeric value extraction ──────────────────────────────────────────

const PX_VALUE_RE = /(\d+(?:\.\d+)?)\s*px/g

function extractPxValues(css: string, propertyPattern: RegExp): number[] {
  const values = new Set<number>()
  let propMatch: RegExpExecArray | null
  const re = new RegExp(propertyPattern.source, 'gi')
  while ((propMatch = re.exec(css)) !== null) {
    const decl = propMatch[0]
    let pxMatch: RegExpExecArray | null
    const pxRe = new RegExp(PX_VALUE_RE.source, 'g')
    while ((pxMatch = pxRe.exec(decl)) !== null) {
      const v = parseFloat(pxMatch[1])
      if (v > 0 && v < 200) values.add(v)
    }
  }
  return Array.from(values).sort((a, b) => a - b)
}

const SPACING_RE = /(?:padding|margin|gap)\s*:\s*[^;]+/gi
const RADIUS_RE = /border-radius\s*:\s*[^;]+/gi
const FONT_SIZE_RE = /font-size\s*:\s*[^;]+/gi

// ── Inline color extraction (background, color, border-color) ─────────

const INLINE_COLOR_RE = /(?:background(?:-color)?|(?:^|\s)color|border-color)\s*:\s*([^;}"]+)/gi

function extractInlineColors(css: string): Record<string, string> {
  const result: Record<string, string> = {}
  let match: RegExpExecArray | null
  const re = new RegExp(INLINE_COLOR_RE.source, 'gi')
  let idx = 0
  while ((match = re.exec(css)) !== null) {
    const hex = normalizeColor(match[1])
    if (hex) result[`inline-color-${idx++}`] = hex
  }
  return result
}

// ── Public API ────────────────────────────────────────────────────────

/** Extract tokens from a CSS string */
export function extractTokensFromCSS(css: string): ExtractedTokens {
  return {
    colors: { ...extractCSSCustomProperties(css), ...extractInlineColors(css) },
    fonts: extractFonts(css),
    spacing: extractPxValues(css, SPACING_RE),
    borderRadius: extractPxValues(css, RADIUS_RE),
    fontSizes: extractPxValues(css, FONT_SIZE_RE),
  }
}

/** Extract tokens from HTML (parses inline <style> blocks and style="" attributes) */
export function extractTokensFromHTML(html: string): ExtractedTokens {
  let combined = emptyTokens()

  // Extract <style> blocks
  const styleBlockRe = /<style[^>]*>([\s\S]*?)<\/style>/gi
  let blockMatch: RegExpExecArray | null
  while ((blockMatch = styleBlockRe.exec(html)) !== null) {
    combined = mergeTokens(combined, extractTokensFromCSS(blockMatch[1]))
  }

  // Extract style="" attributes
  const styleAttrRe = /style\s*=\s*"([^"]+)"/gi
  let attrMatch: RegExpExecArray | null
  while ((attrMatch = styleAttrRe.exec(html)) !== null) {
    combined = mergeTokens(combined, extractTokensFromCSS(attrMatch[1]))
  }

  return combined
}

// ── Map tokens to VibeKit variable definitions ────────────────────────

/** Compute relative luminance of a hex color (0 = black, 1 = white) */
function luminance(hex: string): number {
  const r = parseInt(hex.slice(1, 3), 16) / 255
  const g = parseInt(hex.slice(3, 5), 16) / 255
  const b = parseInt(hex.slice(5, 7), 16) / 255
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

/**
 * Map extracted tokens to VibeKit-compatible variable definitions.
 * Uses heuristics: darkest frequent color → text, lightest → bg, etc.
 */
export function mapTokensToVibeKit(
  tokens: ExtractedTokens,
  _sourceUrl: string,
): Partial<Record<string, VariableDefinition>> {
  const vars: Partial<Record<string, VariableDefinition>> = {}

  // Sort colors by luminance
  const colorEntries = Object.entries(tokens.colors)
  const sorted = colorEntries
    .map(([name, hex]) => ({ name, hex, lum: luminance(hex) }))
    .sort((a, b) => a.lum - b.lum)

  if (sorted.length > 0) {
    // Darkest → text, lightest → bg
    vars['color-text'] = { type: 'color', value: sorted[0].hex }
    vars['color-bg'] = { type: 'color', value: sorted[sorted.length - 1].hex }

    // Look for semantic CSS variable names first
    for (const { name, hex } of sorted) {
      const n = name.toLowerCase()
      if (n.includes('primary') && !vars['color-primary'])
        vars['color-primary'] = { type: 'color', value: hex }
      if (n.includes('secondary') && !vars['color-secondary'])
        vars['color-secondary'] = { type: 'color', value: hex }
      if (n.includes('accent') && !vars['color-accent'])
        vars['color-accent'] = { type: 'color', value: hex }
      if ((n.includes('surface') || n.includes('card')) && !vars['color-surface'])
        vars['color-surface'] = { type: 'color', value: hex }
      if ((n.includes('muted') || n.includes('subtle')) && !vars['color-text-muted'])
        vars['color-text-muted'] = { type: 'color', value: hex }
      if (n.includes('border') && !vars['color-border'])
        vars['color-border'] = { type: 'color', value: hex }
    }

    // Fill in primary from mid-luminance saturated color if not found via name
    if (!vars['color-primary'] && sorted.length >= 3) {
      const mid = sorted[Math.floor(sorted.length / 2)]
      vars['color-primary'] = { type: 'color', value: mid.hex }
    }
  }

  // Fonts
  if (tokens.fonts.length > 0) {
    vars['font-heading'] = { type: 'string', value: tokens.fonts[0] }
    vars['font-body'] = { type: 'string', value: tokens.fonts[tokens.fonts.length > 1 ? 1 : 0] }
  }

  // Spacing scale
  const spacingNames = ['space-xs', 'space-sm', 'space-md', 'space-lg', 'space-xl', 'space-2xl']
  const spacingVals = tokens.spacing.slice(0, spacingNames.length)
  spacingVals.forEach((v, i) => {
    vars[spacingNames[i]] = { type: 'number', value: v }
  })

  // Border radius
  if (tokens.borderRadius.length > 0) {
    vars['radius-sm'] = { type: 'number', value: tokens.borderRadius[0] }
    if (tokens.borderRadius.length > 1)
      vars['radius-md'] = { type: 'number', value: tokens.borderRadius[Math.floor(tokens.borderRadius.length / 2)] }
    if (tokens.borderRadius.length > 2)
      vars['radius-lg'] = { type: 'number', value: tokens.borderRadius[tokens.borderRadius.length - 1] }
  }

  // Font sizes
  const sizeNames = ['text-xs', 'text-sm', 'text-base', 'text-lg', 'text-xl', 'text-2xl']
  const sizeVals = tokens.fontSizes.slice(0, sizeNames.length)
  sizeVals.forEach((v, i) => {
    vars[sizeNames[i]] = { type: 'number', value: v }
  })

  return vars
}
