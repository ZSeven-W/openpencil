import type { FigmaNodeChange } from './figma-types'
import type { TextNode } from '@/types/pen'
import type { StyledTextSegment } from '@/types/styles'
import { figmaColorToHex } from './figma-color-utils'

/**
 * Map Figma .fig internal text properties to PenNode TextNode partial.
 */
export function mapFigmaTextProps(
  node: FigmaNodeChange
): Pick<
  TextNode,
  | 'content'
  | 'fontFamily'
  | 'fontSize'
  | 'fontWeight'
  | 'fontStyle'
  | 'letterSpacing'
  | 'lineHeight'
  | 'textAlign'
  | 'textAlignVertical'
  | 'textGrowth'
  | 'underline'
  | 'strikethrough'
> {
  const result: ReturnType<typeof mapFigmaTextProps> = {
    content: applyTextCase(buildContent(node), node.textCase),
    fontFamily: node.fontName?.family,
    fontSize: node.fontSize,
    fontWeight: parseFontWeight(node.fontName?.style),
    fontStyle: node.fontName?.style?.toLowerCase().includes('italic') ? 'italic' : undefined,
    letterSpacing: mapLetterSpacing(node),
    lineHeight: mapLineHeight(node),
    textAlign: mapTextAlign(node.textAlignHorizontal),
    textAlignVertical: mapTextAlignVertical(node.textAlignVertical),
    textGrowth: mapTextGrowth(node.textAutoResize),
  }

  if (node.textDecoration === 'UNDERLINE') result.underline = true
  if (node.textDecoration === 'STRIKETHROUGH') result.strikethrough = true

  return result
}

function applyTextCase(
  content: string | StyledTextSegment[],
  textCase?: string,
): string | StyledTextSegment[] {
  if (!textCase || textCase === 'ORIGINAL') return content

  const transform = (text: string): string => {
    switch (textCase) {
      case 'UPPER': return text.toUpperCase()
      case 'LOWER': return text.toLowerCase()
      case 'TITLE': return text.replace(/\b\w/g, c => c.toUpperCase())
      default: return text
    }
  }

  if (typeof content === 'string') {
    return transform(content)
  }

  return content.map(seg => ({ ...seg, text: transform(seg.text) }))
}

function buildContent(node: FigmaNodeChange): string | StyledTextSegment[] {
  const textData = node.textData
  if (!textData?.characters) return ''

  const text = textData.characters
  const styleIds = textData.characterStyleIDs
  const table = textData.styleOverrideTable

  if (!styleIds || !table || styleIds.length === 0 || table.length === 0) {
    return text
  }

  // Build segments from character style IDs
  const segments: StyledTextSegment[] = []
  let currentStyleId = styleIds[0] ?? 0
  let segStart = 0

  for (let i = 1; i <= text.length; i++) {
    const styleId = i < styleIds.length ? styleIds[i] : -1
    if (styleId !== currentStyleId || i === text.length) {
      const endIdx = i === text.length ? text.length : i
      const segText = text.slice(segStart, endIdx)
      if (segText) {
        const segment = buildSegment(segText, currentStyleId, table)
        segments.push(segment)
      }
      currentStyleId = styleId
      segStart = i
    }
  }

  // If all segments have no style overrides, return plain string
  if (segments.every((s) => !s.fontFamily && !s.fontSize && !s.fontWeight && !s.fill)) {
    return text
  }

  return segments
}

function buildSegment(
  text: string,
  styleId: number,
  table: FigmaNodeChange[]
): StyledTextSegment {
  if (styleId === 0) return { text }

  // styleOverrideTable is 0-indexed but style IDs start from 1 in some cases
  const override = table[styleId] ?? table[styleId - 1]
  if (!override) return { text }

  const segment: StyledTextSegment = { text }
  if (override.fontName?.family) segment.fontFamily = override.fontName.family
  if (override.fontSize) segment.fontSize = override.fontSize
  const weight = parseFontWeight(override.fontName?.style)
  if (weight) segment.fontWeight = weight
  if (override.fontName?.style?.toLowerCase().includes('italic')) {
    segment.fontStyle = 'italic'
  }
  if (override.textDecoration === 'UNDERLINE') segment.underline = true
  if (override.textDecoration === 'STRIKETHROUGH') segment.strikethrough = true

  // Text fill color
  if (override.fillPaints?.[0]?.color) {
    segment.fill = figmaColorToHex(override.fillPaints[0].color)
  }

  return segment
}

function parseFontWeight(style?: string): number | undefined {
  if (!style) return undefined
  const lower = style.toLowerCase()
  if (lower.includes('thin') || lower.includes('hairline')) return 100
  if (lower.includes('extralight') || lower.includes('ultralight')) return 200
  if (lower.includes('light')) return 300
  if (lower.includes('regular') || lower.includes('normal')) return 400
  if (lower.includes('medium')) return 500
  if (lower.includes('semibold') || lower.includes('demibold')) return 600
  if (lower.includes('extrabold') || lower.includes('ultrabold')) return 800
  if (lower.includes('bold')) return 700
  if (lower.includes('black') || lower.includes('heavy')) return 900
  return undefined
}

function mapLineHeight(node: FigmaNodeChange): number | undefined {
  if (!node.lineHeight) return undefined
  if (node.lineHeight.units === 'PIXELS' && node.lineHeight.value) {
    return node.lineHeight.value
  }
  return undefined
}

function mapLetterSpacing(node: FigmaNodeChange): number | undefined {
  if (!node.letterSpacing) return undefined
  if (node.letterSpacing.units === 'PIXELS' && node.letterSpacing.value) {
    return node.letterSpacing.value
  }
  return undefined
}

function mapTextAlign(
  align?: string
): TextNode['textAlign'] {
  switch (align) {
    case 'LEFT': return 'left'
    case 'CENTER': return 'center'
    case 'RIGHT': return 'right'
    case 'JUSTIFIED': return 'justify'
    default: return undefined
  }
}

function mapTextAlignVertical(
  align?: string
): TextNode['textAlignVertical'] {
  switch (align) {
    case 'TOP': return 'top'
    case 'CENTER': return 'middle'
    case 'BOTTOM': return 'bottom'
    default: return undefined
  }
}

function mapTextGrowth(
  resize?: string
): TextNode['textGrowth'] {
  switch (resize) {
    case 'WIDTH_AND_HEIGHT': return 'auto'
    case 'HEIGHT': return 'fixed-width'
    case 'NONE': return 'fixed-width-height'
    default: return undefined
  }
}
