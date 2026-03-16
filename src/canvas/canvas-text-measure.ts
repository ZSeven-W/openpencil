import type { PenNode } from '@/types/pen'
import { cssFontFamily } from './skia/skia-paint-utils'

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
  if (fontSize >= 40) return 1.0   // Display text: tight leading (matches Pencil 0.9-1.0)
  if (fontSize >= 28) return 1.15  // Heading text: moderate (matches Pencil 1.0-1.2)
  if (fontSize >= 20) return 1.2   // Subheading
  return 1.5                       // Body text: comfortable reading
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

/**
 * Font weight multiplier — bold/semibold text is wider than regular text.
 * Values based on typical proportional font width scaling.
 */
function fontWeightFactor(fontWeight?: string | number): number {
  const w = typeof fontWeight === 'string' ? parseInt(fontWeight, 10) : (fontWeight ?? 400)
  if (isNaN(w) || w <= 400) return 1.0
  if (w <= 500) return 1.03
  if (w <= 600) return 1.06
  if (w <= 700) return 1.09
  return 1.12
}

export function estimateGlyphWidth(ch: string, fontSize: number, fontWeight?: string | number): number {
  if (ch === '\n' || ch === '\r') return 0
  if (ch === '\t') return fontSize * 1.2
  if (ch === ' ') return fontSize * 0.33

  const wf = fontWeightFactor(fontWeight)
  const code = ch.codePointAt(0) ?? 0
  if (isCjkCodePoint(code)) return fontSize * 1.12 * wf
  if (/[A-Z]/.test(ch)) return fontSize * 0.62 * wf
  if (/[a-z]/.test(ch)) return fontSize * 0.56 * wf
  if (/[0-9]/.test(ch)) return fontSize * 0.56 * wf
  return fontSize * 0.58 * wf
}

export function estimateLineWidth(
  text: string,
  fontSize: number,
  letterSpacing = 0,
  fontWeight?: string | number,
): number {
  let width = 0
  let visibleChars = 0
  for (const ch of text) {
    width += estimateGlyphWidth(ch, fontSize, fontWeight)
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

export function estimateTextWidth(text: string, fontSize: number, letterSpacing = 0, fontWeight?: string | number): number {
  const lines = text.split(/\r?\n/)
  const maxLine = lines.reduce((max, line) => {
    const lineWidth = estimateLineWidth(line, fontSize, letterSpacing, fontWeight)
    const safeLineWidth = lineWidth * widthSafetyFactor(line)
    return Math.max(max, safeLineWidth)
  }, 0)
  return maxLine
}

/**
 * Estimate text width WITHOUT safety factor.
 * Used for layout centering where the safety margin causes text to appear
 * off-center (the overestimated width shifts the text box left when centered).
 * For wrapping/sizing decisions, use estimateTextWidth() which includes the safety factor.
 */
export function estimateTextWidthPrecise(text: string, fontSize: number, letterSpacing = 0, fontWeight?: string | number): number {
  const lines = text.split(/\r?\n/)
  return lines.reduce((max, line) => {
    return Math.max(max, estimateLineWidth(line, fontSize, letterSpacing, fontWeight))
  }, 0)
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
 * Within the Fabric text bounding box (fontSize * 1.13), glyph ink sits
 * slightly above the mathematical center due to ascent/descent asymmetry.
 * We nudge down proportionally to compensate.
 */
export function getTextOpticalCenterYOffset(node: PenNode): number {
  if (node.type !== 'text') return 0
  const text = resolveTextContent(node).trim()
  if (!text) return 0
  if (countExplicitTextLines(text) > 1) return 0

  const fontSize = node.fontSize ?? 16
  const hasCjk = hasCjkText(text)

  // CJK glyphs sit higher in the em box than Latin glyphs
  const ratio = hasCjk ? 0.06 : 0.03
  const offset = fontSize * ratio
  return Math.max(0, Math.min(Math.round(fontSize * 0.05), Math.round(offset)))
}

// ---------------------------------------------------------------------------
// Canvas 2D measurement context (lazy singleton, browser-only)
// ---------------------------------------------------------------------------

let _textMeasureCtx: CanvasRenderingContext2D | null = null
function getTextMeasureCtx(): CanvasRenderingContext2D | null {
  if (typeof document === 'undefined') return null
  if (!_textMeasureCtx) {
    const c = document.createElement('canvas')
    _textMeasureCtx = c.getContext('2d')
  }
  return _textMeasureCtx
}

/**
 * Count wrapped lines using Canvas 2D measureText for accurate word-wrap
 * prediction. Falls back to character-width estimation if Canvas 2D is
 * unavailable (e.g. SSR).
 */
function countWrappedLinesCanvas2D(
  rawLines: string[],
  wrapWidth: number,
  fontSize: number,
  fontWeight: string | number | undefined,
  fontFamily: string,
  letterSpacing: number,
): number {
  const ctx = getTextMeasureCtx()
  if (!ctx) {
    // Fallback: character-width estimation
    return rawLines.reduce((sum, line) => {
      const lineWidth = estimateLineWidth(line, fontSize, letterSpacing, fontWeight) * widthSafetyFactor(line)
      return sum + Math.max(1, Math.ceil(lineWidth / wrapWidth))
    }, 0)
  }

  const fw = typeof fontWeight === 'number' ? String(fontWeight) : (fontWeight ?? '400')
  ctx.font = `${fw} ${fontSize}px ${cssFontFamily(fontFamily)}`

  let total = 0
  for (const rawLine of rawLines) {
    if (!rawLine) { total += 1; continue }
    // Word-wrap using Canvas 2D measureText — same logic as the renderer's wrapLine
    if (ctx.measureText(rawLine).width <= wrapWidth) { total += 1; continue }
    let lineCount = 0
    let current = ''
    let i = 0
    while (i < rawLine.length) {
      const ch = rawLine[i]
      if (isCjkCodePoint(ch.codePointAt(0) ?? 0)) {
        const test = current + ch
        if (ctx.measureText(test).width > wrapWidth && current) {
          lineCount++
          current = ch
        } else {
          current = test
        }
        i++
      } else if (ch === ' ') {
        const test = current + ch
        if (ctx.measureText(test).width > wrapWidth && current) {
          lineCount++
          current = ''
        } else {
          current = test
        }
        i++
      } else {
        // Collect word
        let word = ''
        while (i < rawLine.length && rawLine[i] !== ' ' && !isCjkCodePoint(rawLine[i].codePointAt(0) ?? 0)) {
          word += rawLine[i]
          i++
        }
        const test = current + word
        if (ctx.measureText(test).width > wrapWidth && current) {
          lineCount++
          current = word
        } else {
          current = test
        }
      }
    }
    if (current) lineCount++
    total += Math.max(1, lineCount)
  }
  return total
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
  // Fabric.js uses _fontSizeMult = 1.13 for the glyph height of a single line.
  // lineHeight spacing applies *between* lines, not below the last line.
  const FABRIC_FONT_MULT = 1.13
  const glyphH = fontSize * FABRIC_FONT_MULT
  const lineStep = fontSize * lineHeight

  // Get text content
  const rawContent = n.content
  const content = typeof rawContent === 'string'
    ? rawContent
    : Array.isArray(rawContent)
      ? rawContent.map((s: { text: string }) => s.text).join('')
      : ''
  if (!content) return glyphH

  // Determine the effective text width for wrapping estimation
  let textWidth = 0
  if ('width' in node) {
    const w = parseSizing(node.width)
    if (typeof w === 'number' && w > 0) textWidth = w
    else if (w === 'fill' && availableWidth && availableWidth > 0) textWidth = availableWidth
  }

  // If no width constraint is known, still count explicit newlines
  if (textWidth <= 0) {
    const explicitLines = content.split(/\r?\n/).length
    const n2 = Math.max(1, explicitLines)
    return Math.round(n2 <= 1 ? glyphH : (n2 - 1) * lineStep + glyphH)
  }

  // Use Canvas 2D measureText for accurate wrapping prediction (matches renderer).
  // Falls back to character-width estimation in non-browser environments.
  const fontWeight = n.fontWeight as string | number | undefined
  const fontFamily = (typeof n.fontFamily === 'string' ? n.fontFamily : '') || 'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif'
  const letterSpacing = (typeof n.letterSpacing === 'number' ? n.letterSpacing : 0)
  const rawLines = content.split(/\r?\n/)
  // Add tolerance matching the renderer's wrapLine (w + fontSize * 0.2)
  const wrapWidth = textWidth + fontSize * 0.2
  const wrappedLineCount = countWrappedLinesCanvas2D(
    rawLines, wrapWidth, fontSize, fontWeight, fontFamily, letterSpacing,
  )

  const totalLines = Math.max(1, wrappedLineCount)
  return Math.round(totalLines <= 1 ? glyphH : (totalLines - 1) * lineStep + glyphH)
}
