import type { PenNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Pure utility functions extracted from design-generator.ts
// ---------------------------------------------------------------------------

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

export function toSizeNumber(
  value: number | string | undefined,
  fallback: number,
): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (typeof value === 'string') {
    const wrapped = value.match(/\((\d+(?:\.\d+)?)\)/)
    if (wrapped) return Number(wrapped[1])
    const parsed = Number(value)
    if (Number.isFinite(parsed)) return parsed
  }
  return fallback
}

export function toGapNumber(value: number | string | undefined): number {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    const parsed = Number(value)
    if (Number.isFinite(parsed)) return parsed
  }
  return 0
}

export function toStrokeThicknessNumber(
  stroke: { thickness?: number | [number, number, number, number] } | undefined,
  fallback: number,
): number {
  if (!stroke) return fallback
  const t = stroke.thickness
  if (typeof t === 'number' && Number.isFinite(t)) return t
  if (Array.isArray(t) && t.length > 0 && Number.isFinite(t[0])) return t[0]
  return fallback
}

export function toCornerRadiusNumber(
  value: number | [number, number, number, number] | undefined,
  fallback: number,
): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (Array.isArray(value) && value.length > 0 && Number.isFinite(value[0]))
    return value[0]
  return fallback
}

export function parsePaddingValues(
  padding:
    | number
    | [number, number]
    | [number, number, number, number]
    | string
    | undefined,
): { top: number; right: number; bottom: number; left: number } {
  if (typeof padding === 'number') {
    return { top: padding, right: padding, bottom: padding, left: padding }
  }
  if (typeof padding === 'string') {
    const parsed = Number(padding)
    if (Number.isFinite(parsed)) {
      return { top: parsed, right: parsed, bottom: parsed, left: parsed }
    }
    return { top: 0, right: 0, bottom: 0, left: 0 }
  }
  if (Array.isArray(padding) && padding.length === 2) {
    return {
      top: padding[0],
      right: padding[1],
      bottom: padding[0],
      left: padding[1],
    }
  }
  if (Array.isArray(padding) && padding.length === 4) {
    return {
      top: padding[0],
      right: padding[1],
      bottom: padding[2],
      left: padding[3],
    }
  }
  return { top: 0, right: 0, bottom: 0, left: 0 }
}

// ---------------------------------------------------------------------------
// Text measurement utilities
// ---------------------------------------------------------------------------

export function estimateWrappingLineWidth(
  text: string,
  fontSize: number,
): number {
  let width = 0
  for (const char of text) {
    if (
      /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF\uFE30-\uFE4F]/.test(
        char,
      )
    ) {
      width += fontSize * 1.12
    } else if (char === ' ') {
      width += fontSize * 0.3
    } else if (/[A-Z]/.test(char)) {
      width += fontSize * 0.72
    } else {
      width += fontSize * 0.58
    }
  }
  return width
}

export function estimateSingleLineTextWidth(
  text: string,
  fontSize: number,
): number {
  let width = 0
  for (const char of text) {
    if (
      /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF\uFE30-\uFE4F]/.test(
        char,
      )
    ) {
      width += fontSize * 1.35
    } else if (char === ' ') {
      width += fontSize * 0.3
    } else {
      width += fontSize * 0.6
    }
  }
  return width
}

/**
 * Estimate auto-height for text with wrapping.
 * `canvasWidth` is used as fallback when parentContentWidth is unavailable.
 */
export function estimateAutoHeight(
  text: string,
  fontSize: number,
  lineHeight: number,
  parentContentWidth?: number,
  canvasWidth = 1200,
): number {
  const hasCjk = /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F]/.test(text)
  const minLh = hasCjk
    ? fontSize >= 28
      ? 1.28
      : 1.5
    : fontSize >= 28
      ? 1.15
      : 1.35
  const effectiveLh = Math.max(lineHeight, minLh)

  const availW =
    parentContentWidth && parentContentWidth > 0
      ? Math.max(120, parentContentWidth)
      : canvasWidth <= 520
        ? Math.max(220, canvasWidth * 0.74)
        : Math.max(260, canvasWidth * 0.5)

  const logicalLines = text.split(/\r?\n/)
  const wrappedLineCount = logicalLines.reduce((sum, line) => {
    const lineWidth = estimateWrappingLineWidth(line, fontSize)
    return sum + Math.max(1, Math.ceil(lineWidth / availW))
  }, 0)
  const safetyMargin = fontSize >= 28 ? 1.15 : 1.1
  return Math.round(
    Math.max(1, wrappedLineCount) * fontSize * effectiveLh * safetyMargin,
  )
}

export function estimateNodeIntrinsicHeight(
  node: PenNode,
  parentContentWidth?: number,
  canvasWidth = 1200,
): number {
  const explicitHeight = toSizeNumber(
    ('height' in node ? node.height : undefined) as
      | number
      | string
      | undefined,
    0,
  )

  let textHeight = 0
  if (node.type === 'text') {
    const fs = node.fontSize ?? 16
    const lh = node.lineHeight ?? (fs >= 28 ? 1.2 : 1.5)
    if (
      typeof node.content === 'string' &&
      node.content.trim() &&
      node.textGrowth === 'fixed-width' &&
      parentContentWidth &&
      parentContentWidth > 0
    ) {
      textHeight = estimateAutoHeight(
        node.content.trim(),
        fs,
        lh,
        parentContentWidth,
        canvasWidth,
      )
    } else {
      textHeight = Math.max(20, Math.round(fs * lh))
    }
  }

  if (
    !('children' in node) ||
    !Array.isArray(node.children) ||
    node.children.length === 0
  ) {
    if (node.type === 'text' && textHeight > 0) {
      return Math.max(explicitHeight, textHeight)
    }
    return explicitHeight || textHeight || 80
  }

  const padding = parsePaddingValues(
    'padding' in node ? node.padding : undefined,
  )
  const gap = toGapNumber('gap' in node ? node.gap : undefined)
  const layout = 'layout' in node ? node.layout : undefined
  const children = node.children

  const nodeW = toSizeNumber(
    ('width' in node ? node.width : undefined) as number | string | undefined,
    0,
  )
  const childContentW = nodeW > 0 ? nodeW - padding.left - padding.right : 0

  if (layout === 'vertical') {
    let total = padding.top + padding.bottom
    for (const child of children) {
      total += estimateNodeIntrinsicHeight(
        child,
        childContentW || undefined,
        canvasWidth,
      )
    }
    if (children.length > 1) {
      total += gap * (children.length - 1)
    }
    return Math.max(explicitHeight, total)
  }

  if (layout === 'horizontal') {
    const childCount = children.length
    const totalGap = childCount > 1 ? gap * (childCount - 1) : 0
    const perChildW =
      childContentW > 0 && childCount > 0
        ? (childContentW - totalGap) / childCount
        : 0
    let maxChild = 0
    for (const child of children) {
      const childW = toSizeNumber(
        ('width' in child ? child.width : undefined) as
          | number
          | string
          | undefined,
        0,
      )
      const effectiveW =
        childW > 0 ? childW : perChildW > 0 ? perChildW : undefined
      maxChild = Math.max(
        maxChild,
        estimateNodeIntrinsicHeight(child, effectiveW, canvasWidth),
      )
    }
    const total = padding.top + padding.bottom + maxChild
    return Math.max(explicitHeight, total)
  }

  let boundsBottom = 0
  for (const child of children) {
    const childY = typeof child.y === 'number' ? child.y : 0
    const childBottom =
      childY +
      estimateNodeIntrinsicHeight(child, childContentW || undefined, canvasWidth)
    boundsBottom = Math.max(boundsBottom, childBottom)
  }

  const contentHeight = boundsBottom + padding.bottom
  return Math.max(explicitHeight, contentHeight)
}

// ---------------------------------------------------------------------------
// Color & fill utilities
// ---------------------------------------------------------------------------

export function extractPrimaryColor(fill: unknown): string | null {
  if (!Array.isArray(fill) || fill.length === 0) return null
  const first = fill[0]
  if (!first || typeof first !== 'object') return null
  const solid = first as { type?: string; color?: string }
  if (solid.type !== 'solid') return null
  return solid.color ?? null
}

export function isFillDark(fill: unknown): boolean {
  if (!Array.isArray(fill) || fill.length === 0) return true
  const first = fill[0] as { type?: string; color?: string }
  if (first.type !== 'solid' || !first.color) return true
  const c = first.color
  if (c.startsWith('rgba')) {
    const parts = c
      .replace(/rgba?\(|\)/g, '')
      .split(',')
      .map(Number)
    if (parts.length >= 3)
      return (parts[0] * 299 + parts[1] * 587 + parts[2] * 114) / 1000 < 128
  }
  const hex = c.replace('#', '')
  if (hex.length < 6) return true
  const r = parseInt(hex.substring(0, 2), 16)
  const g = parseInt(hex.substring(2, 4), 16)
  const b = parseInt(hex.substring(4, 6), 16)
  return (r * 299 + g * 587 + b * 114) / 1000 < 128
}

export function getPlaceholderColors(fill: unknown): {
  fillColor: string
  strokeColor: string
  textColor: string
} {
  if (isFillDark(fill)) {
    return {
      fillColor: '#111627',
      strokeColor: '#1E2440',
      textColor: '#2A3050',
    }
  }
  return { fillColor: '#F1F5F9', strokeColor: '#D1D5DB', textColor: '#C0C4CC' }
}

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

export function getTextContentForNode(node: PenNode): string {
  if (node.type !== 'text') return ''
  return typeof node.content === 'string'
    ? node.content
    : Array.isArray(node.content)
      ? node.content.map((s: { text: string }) => s.text).join('')
      : ''
}

export function hasCjkText(text: string): boolean {
  return /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF]/.test(text)
}

// ---------------------------------------------------------------------------
// Phone placeholder SVG
// ---------------------------------------------------------------------------

export function createPhonePlaceholderDataUri(
  width: number,
  height: number,
  dark = true,
): string {
  const w = Math.max(140, Math.round(width))
  const h = Math.max(240, Math.round(height))
  const pad = Math.max(8, Math.round(Math.min(w, h) * 0.05))
  const innerW = w - pad * 2
  const innerH = h - pad * 2
  const outerR = Math.round(Math.min(w, h) * 0.12)
  const innerR = Math.max(outerR - pad, 8)

  const bgColor = dark ? '#111627' : '#F1F5F9'
  const strokeColor = dark ? '#1E2440' : '#D1D5DB'

  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">` +
    `<rect width="${w}" height="${h}" rx="${outerR}" fill="${bgColor}"/>` +
    `<rect x="${pad}" y="${pad}" width="${innerW}" height="${innerH}" rx="${innerR}" fill="none" stroke="${strokeColor}" stroke-width="1"/>` +
    `</svg>`
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}
