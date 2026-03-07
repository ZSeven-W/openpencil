import type { PenNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Sizing parser (shared by layout engine and text height estimation)
// ---------------------------------------------------------------------------

/** Parse a sizing value. Handles number, "fit_content", "fill_container" and parenthesized forms. */
export function parseSizing(value: unknown): number | 'fit' | 'fill' {
  if (typeof value === 'number') return value
  if (typeof value !== 'string') return 0
  if (value.startsWith('fill_container')) return 'fill'
  if (value.startsWith('fit_content')) return 'fit'
  const n = parseFloat(value)
  return isNaN(n) ? 0 : n
}

// ---------------------------------------------------------------------------
// Default line height — single source of truth for all modules
// ---------------------------------------------------------------------------

/**
 * Canonical default lineHeight when a text node has no explicit value.
 * Display/heading text (>=28px) gets tighter spacing; body text gets looser.
 * All modules (factory, layout engine, text estimation, AI generation)
 * MUST use this function instead of hardcoded fallbacks.
 */
export function defaultLineHeight(fontSize: number): number {
  return fontSize >= 28 ? 1.2 : 1.5
}

// ---------------------------------------------------------------------------
// CJK detection
// ---------------------------------------------------------------------------

export function isCjkCodePoint(code: number): boolean {
  return (code >= 0x4E00 && code <= 0x9FFF) // CJK Unified Ideographs
    || (code >= 0x3400 && code <= 0x4DBF) // CJK Extension A
    || (code >= 0x3040 && code <= 0x30FF) // Hiragana + Katakana
    || (code >= 0xAC00 && code <= 0xD7AF) // Hangul
    || (code >= 0x3000 && code <= 0x303F) // CJK symbols/punctuation
    || (code >= 0xFF00 && code <= 0xFFEF) // Full-width forms
}

export function hasCjkText(text: string): boolean {
  for (const ch of text) {
    const code = ch.codePointAt(0) ?? 0
    if (isCjkCodePoint(code)) return true
  }
  return false
}

// ---------------------------------------------------------------------------
// Glyph / line width estimation
// ---------------------------------------------------------------------------

export function estimateGlyphWidth(ch: string, fontSize: number): number {
  if (ch === '\n' || ch === '\r') return 0
  if (ch === '\t') return fontSize * 1.2
  if (ch === ' ') return fontSize * 0.33

  const code = ch.codePointAt(0) ?? 0
  if (isCjkCodePoint(code)) return fontSize * 1.12
  if (/[A-Z]/.test(ch)) return fontSize * 0.62
  if (/[a-z]/.test(ch)) return fontSize * 0.56
  if (/[0-9]/.test(ch)) return fontSize * 0.56
  return fontSize * 0.58
}

export function estimateLineWidth(
  text: string,
  fontSize: number,
  letterSpacing = 0,
): number {
  let width = 0
  let visibleChars = 0
  for (const ch of text) {
    width += estimateGlyphWidth(ch, fontSize)
    if (ch !== '\n' && ch !== '\r') visibleChars += 1
  }
  if (visibleChars > 1 && letterSpacing !== 0) {
    width += (visibleChars - 1) * letterSpacing
  }
  return Math.max(0, width)
}

function widthSafetyFactor(text: string): number {
  // Latin fonts vary a lot by weight/family; use a larger safety margin to
  // avoid underestimating width and causing accidental wraps.
  return hasCjkText(text) ? 1.06 : 1.14
}

export function estimateTextWidth(text: string, fontSize: number, letterSpacing = 0): number {
  const lines = text.split(/\r?\n/)
  const maxLine = lines.reduce((max, line) => {
    const lineWidth = estimateLineWidth(line, fontSize, letterSpacing)
    const safeLineWidth = lineWidth * widthSafetyFactor(line)
    return Math.max(max, safeLineWidth)
  }, 0)
  return maxLine
}

// ---------------------------------------------------------------------------
// Text content helpers
// ---------------------------------------------------------------------------

export function resolveTextContent(node: PenNode): string {
  if (node.type !== 'text') return ''
  return typeof node.content === 'string'
    ? node.content
    : node.content.map((s) => s.text).join('')
}

export function countExplicitTextLines(text: string): number {
  if (!text) return 1
  return Math.max(1, text.split(/\r?\n/).length)
}

// ---------------------------------------------------------------------------
// Optical vertical correction for centered single-line text
// ---------------------------------------------------------------------------

/**
 * Optical vertical correction for centered single-line text.
 * Font line boxes are mathematically centered but glyph ink tends to look
 * slightly top-heavy, especially for CJK, so we nudge down proportionally.
 * The offset scales with fontSize (no fixed cap) so large text stays centered.
 */
export function getTextOpticalCenterYOffset(node: PenNode): number {
  if (node.type !== 'text') return 0
  const text = resolveTextContent(node).trim()
  if (!text) return 0
  if (countExplicitTextLines(text) > 1) return 0

  const fontSize = node.fontSize ?? 16
  const lineHeight = node.lineHeight ?? defaultLineHeight(fontSize)
  const hasCjk = hasCjkText(text)

  // Base ratio: CJK glyphs sit higher in the em box than Latin
  const ratio = hasCjk ? 0.12 : 0.07
  // When lineHeight is compact (≤1.35), the visual offset is more pronounced
  const compactLineBoost = lineHeight <= 1.35 ? 1 : 0.65
  const offset = fontSize * ratio * compactLineBoost
  // Proportional cap: never exceed 8% of fontSize, minimum 1px
  return Math.max(1, Math.min(Math.round(fontSize * 0.08), Math.round(offset)))
}

// ---------------------------------------------------------------------------
// Text height estimation (multi-line wrapping aware)
// ---------------------------------------------------------------------------

/** Estimate text height including multi-line wrapping when available width is known. */
export function estimateTextHeight(node: PenNode, availableWidth?: number): number {
  // Access text-specific properties via Record to avoid union type issues
  const n = node as unknown as Record<string, unknown>
  const fontSize = (typeof n.fontSize === 'number' ? n.fontSize : 16)
  const lineHeight = (typeof n.lineHeight === 'number' ? n.lineHeight : defaultLineHeight(fontSize))
  const singleLineH = fontSize * lineHeight

  // Get text content
  const rawContent = n.content
  const content = typeof rawContent === 'string'
    ? rawContent
    : Array.isArray(rawContent)
      ? rawContent.map((s: { text: string }) => s.text).join('')
      : ''
  if (!content) return singleLineH

  // Determine the effective text width for wrapping estimation
  let textWidth = 0
  if ('width' in node) {
    const w = parseSizing(node.width)
    if (typeof w === 'number' && w > 0) textWidth = w
    else if (w === 'fill' && availableWidth && availableWidth > 0) textWidth = availableWidth
  }

  // If no width constraint is known, return single-line height
  if (textWidth <= 0) return singleLineH

  // Estimate wrapped lines per paragraph line, then sum.
  // This preserves explicit newlines and avoids under-estimating CJK widths.
  const letterSpacing = (typeof n.letterSpacing === 'number' ? n.letterSpacing : 0)
  const rawLines = content.split(/\r?\n/)
  const wrappedLineCount = rawLines.reduce((sum, line) => {
    const lineWidth = estimateLineWidth(line, fontSize, letterSpacing) * widthSafetyFactor(line)
    return sum + Math.max(1, Math.ceil(lineWidth / textWidth))
  }, 0)

  return Math.round(Math.max(1, wrappedLineCount) * singleLineH)
}
