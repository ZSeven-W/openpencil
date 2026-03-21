import type { PenDocument, PenNode, ContainerProps, TextNode, ImageNode, LineNode, PathNode, PolygonNode } from '@zseven-w/pen-types'
import { getActivePageChildren } from '@zseven-w/pen-core'
import type { PenFill, PenStroke, PenEffect, ShadowEffect } from '@zseven-w/pen-types'
import { isVariableRef } from '@zseven-w/pen-core'
import { variableNameToCSS } from './css-variables-generator.js'

/**
 * Converts PenDocument nodes to SwiftUI code.
 * $variable references are output as var(--name) comments for manual mapping.
 */

/** Convert a `$variable` ref to a placeholder comment, or return the raw value. */
function varOrLiteral(value: string): string {
  if (isVariableRef(value)) {
    return `var(${variableNameToCSS(value.slice(1))})`
  }
  return value
}

function indent(depth: number): string {
  return '    '.repeat(depth)
}

/** Parse a hex color string to SwiftUI Color initializer. */
function hexToSwiftUIColor(hex: string): string {
  if (hex.startsWith('$')) {
    return `Color("${varOrLiteral(hex)}") /* variable */`
  }
  const clean = hex.replace('#', '')
  if (clean.length === 6) {
    const r = parseInt(clean.substring(0, 2), 16) / 255
    const g = parseInt(clean.substring(2, 4), 16) / 255
    const b = parseInt(clean.substring(4, 6), 16) / 255
    return `Color(red: ${r.toFixed(3)}, green: ${g.toFixed(3)}, blue: ${b.toFixed(3)})`
  }
  if (clean.length === 8) {
    const r = parseInt(clean.substring(0, 2), 16) / 255
    const g = parseInt(clean.substring(2, 4), 16) / 255
    const b = parseInt(clean.substring(4, 6), 16) / 255
    const a = parseInt(clean.substring(6, 8), 16) / 255
    return `Color(red: ${r.toFixed(3)}, green: ${g.toFixed(3)}, blue: ${b.toFixed(3)}).opacity(${a.toFixed(3)})`
  }
  return `Color("${hex}")`
}

function fillToSwiftUI(fills: PenFill[] | undefined): string | null {
  if (!fills || fills.length === 0) return null
  const fill = fills[0]
  if (fill.type === 'solid') {
    return hexToSwiftUIColor(fill.color)
  }
  if (fill.type === 'linear_gradient') {
    if (!fill.stops?.length) return null
    const angle = fill.angle ?? 180
    const startPoint = angleToUnitPoint(angle, 'start')
    const endPoint = angleToUnitPoint(angle, 'end')
    const stops = fill.stops
      .map((s) => `.init(color: ${hexToSwiftUIColor(s.color)}, location: ${s.offset.toFixed(2)})`)
      .join(', ')
    return `LinearGradient(stops: [${stops}], startPoint: ${startPoint}, endPoint: ${endPoint})`
  }
  if (fill.type === 'radial_gradient') {
    if (!fill.stops?.length) return null
    const stops = fill.stops
      .map((s) => `.init(color: ${hexToSwiftUIColor(s.color)}, location: ${s.offset.toFixed(2)})`)
      .join(', ')
    return `RadialGradient(stops: [${stops}], center: .center, startRadius: 0, endRadius: 100)`
  }
  return null
}

/** Convert an angle in degrees to SwiftUI UnitPoint for gradient start/end. */
function angleToUnitPoint(angle: number, point: 'start' | 'end'): string {
  const normalized = ((angle % 360) + 360) % 360
  if (point === 'start') {
    if (normalized === 0) return '.bottom'
    if (normalized === 90) return '.leading'
    if (normalized === 180) return '.top'
    if (normalized === 270) return '.trailing'
    return `.top`
  }
  // end
  if (normalized === 0) return '.top'
  if (normalized === 90) return '.trailing'
  if (normalized === 180) return '.bottom'
  if (normalized === 270) return '.leading'
  return `.bottom`
}

function strokeToSwiftUI(
  stroke: PenStroke | undefined,
  cornerRadius: number | [number, number, number, number] | undefined,
): string[] {
  if (!stroke) return []
  const modifiers: string[] = []
  const thickness = typeof stroke.thickness === 'number'
    ? stroke.thickness
    : typeof stroke.thickness === 'string'
      ? stroke.thickness
      : stroke.thickness[0]
  const thicknessStr = typeof thickness === 'string' && isVariableRef(thickness)
    ? `/* ${varOrLiteral(thickness)} */ 1`
    : String(thickness)

  let strokeColor = 'Color.gray'
  if (stroke.fill && stroke.fill.length > 0) {
    const sf = stroke.fill[0]
    if (sf.type === 'solid') {
      strokeColor = hexToSwiftUIColor(sf.color)
    }
  }

  const cr = typeof cornerRadius === 'number' ? cornerRadius : 0
  if (cr > 0) {
    modifiers.push(`.overlay(RoundedRectangle(cornerRadius: ${cr}).stroke(${strokeColor}, lineWidth: ${thicknessStr}))`)
  } else {
    modifiers.push(`.overlay(Rectangle().stroke(${strokeColor}, lineWidth: ${thicknessStr}))`)
  }
  return modifiers
}

function effectsToSwiftUI(effects: PenEffect[] | undefined): string[] {
  if (!effects || effects.length === 0) return []
  const modifiers: string[] = []
  for (const effect of effects) {
    if (effect.type === 'shadow') {
      const s = effect as ShadowEffect
      modifiers.push(`.shadow(color: ${hexToSwiftUIColor(s.color)}, radius: ${s.blur}, x: ${s.offsetX}, y: ${s.offsetY})`)
    } else if (effect.type === 'blur' || effect.type === 'background_blur') {
      modifiers.push(`.blur(radius: ${effect.radius})`)
    }
  }
  return modifiers
}

function paddingToSwiftUI(
  padding: number | [number, number] | [number, number, number, number] | string | undefined,
): string[] {
  if (padding === undefined) return []
  if (typeof padding === 'string' && isVariableRef(padding)) {
    return [`.padding(/* ${varOrLiteral(padding)} */ 0)`]
  }
  if (typeof padding === 'number') {
    return padding > 0 ? [`.padding(${padding})`] : []
  }
  if (Array.isArray(padding)) {
    if (padding.length === 2) {
      const modifiers: string[] = []
      if (padding[0] > 0) modifiers.push(`.padding(.vertical, ${padding[0]})`)
      if (padding[1] > 0) modifiers.push(`.padding(.horizontal, ${padding[1]})`)
      return modifiers
    }
    if (padding.length === 4) {
      const [top, trailing, bottom, leading] = padding
      const modifiers: string[] = []
      if (top > 0) modifiers.push(`.padding(.top, ${top})`)
      if (trailing > 0) modifiers.push(`.padding(.trailing, ${trailing})`)
      if (bottom > 0) modifiers.push(`.padding(.bottom, ${bottom})`)
      if (leading > 0) modifiers.push(`.padding(.leading, ${leading})`)
      return modifiers
    }
  }
  return []
}

function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content
  return node.content.map((s) => s.text).join('')
}

function escapeSwiftString(text: string): string {
  return text
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
}

function fontWeightToSwiftUI(weight: number | string | undefined): string | null {
  if (weight === undefined) return null
  const w = typeof weight === 'number' ? weight : parseInt(weight, 10)
  if (isNaN(w)) return null
  if (w <= 100) return '.ultraLight'
  if (w <= 200) return '.thin'
  if (w <= 300) return '.light'
  if (w <= 400) return '.regular'
  if (w <= 500) return '.medium'
  if (w <= 600) return '.semibold'
  if (w <= 700) return '.bold'
  if (w <= 800) return '.heavy'
  return '.black'
}

function textAlignToSwiftUI(align: string | undefined): string | null {
  if (!align) return null
  const map: Record<string, string> = {
    left: '.leading',
    center: '.center',
    right: '.trailing',
  }
  return map[align] ?? null
}

function alignToSwiftUI(
  alignItems: string | undefined,
  layout: string | undefined,
): string | null {
  if (!alignItems || !layout || layout === 'none') return null
  if (layout === 'vertical') {
    const map: Record<string, string> = {
      start: '.leading',
      center: '.center',
      end: '.trailing',
    }
    return map[alignItems] ?? null
  }
  // horizontal layout: alignment is vertical
  const map: Record<string, string> = {
    start: '.top',
    center: '.center',
    end: '.bottom',
  }
  return map[alignItems] ?? null
}

/** Render a node and its modifiers, returning an array of lines. */
function generateNodeSwiftUI(node: PenNode, depth: number): string {
  const pad = indent(depth)

  switch (node.type) {
    case 'frame':
    case 'rectangle':
    case 'group': {
      return generateContainerSwiftUI(node, depth)
    }

    case 'ellipse': {
      const modifiers: string[] = []
      const fillStr = fillToSwiftUI(node.fill)
      if (fillStr) {
        modifiers.push(`.fill(${fillStr})`)
      }
      if (typeof node.width === 'number' || typeof node.height === 'number') {
        const w = typeof node.width === 'number' ? node.width : (typeof node.height === 'number' ? node.height : 100)
        const h = typeof node.height === 'number' ? node.height : w
        modifiers.push(`.frame(width: ${w}, height: ${h})`)
      }
      modifiers.push(...strokeToSwiftUI(node.stroke, undefined))
      modifiers.push(...effectsToSwiftUI(node.effects))
      modifiers.push(...commonModifiers(node))

      return renderWithModifiers(pad, 'Ellipse()', modifiers)
    }

    case 'text': {
      return generateTextSwiftUI(node, depth)
    }

    case 'line': {
      return generateLineSwiftUI(node, depth)
    }

    case 'polygon':
    case 'path': {
      return generatePathSwiftUI(node, depth)
    }

    case 'image': {
      return generateImageSwiftUI(node, depth)
    }

    case 'icon_font': {
      const size = typeof node.width === 'number' ? node.width : 24
      const color = node.fill?.[0]?.type === 'solid' ? node.fill[0].color : null
      const iconName = (node.iconFontName || 'circle').replace(/-/g, '.')
      const colorMod = color ? `\n${pad}    .foregroundColor(Color(hex: "${color}"))` : ''
      return `${pad}Image("${iconName}")\n${pad}    .resizable()\n${pad}    .frame(width: ${size}, height: ${size})${colorMod}`
    }

    case 'ref':
      return `${pad}// Ref: ${node.ref}`

    default:
      return `${pad}// Unsupported node type`
  }
}

function commonModifiers(node: PenNode): string[] {
  const modifiers: string[] = []

  if (node.opacity !== undefined && node.opacity !== 1) {
    if (typeof node.opacity === 'string' && isVariableRef(node.opacity)) {
      modifiers.push(`.opacity(/* ${varOrLiteral(node.opacity)} */ 1.0)`)
    } else if (typeof node.opacity === 'number') {
      modifiers.push(`.opacity(${node.opacity})`)
    }
  }

  if (node.rotation) {
    modifiers.push(`.rotationEffect(.degrees(${node.rotation}))`)
  }

  if (node.x !== undefined || node.y !== undefined) {
    const x = node.x ?? 0
    const y = node.y ?? 0
    modifiers.push(`.offset(x: ${x}, y: ${y})`)
  }

  return modifiers
}

function renderWithModifiers(
  pad: string,
  element: string,
  modifiers: string[],
): string {
  if (modifiers.length === 0) {
    return `${pad}${element}`
  }
  const lines = [`${pad}${element}`]
  for (const mod of modifiers) {
    lines.push(`${pad}    ${mod}`)
  }
  return lines.join('\n')
}

function generateContainerSwiftUI(
  node: PenNode & ContainerProps,
  depth: number,
): string {
  const pad = indent(depth)
  const children = node.children ?? []
  const hasLayout = node.layout === 'vertical' || node.layout === 'horizontal'
  const cr = typeof node.cornerRadius === 'number' ? node.cornerRadius : 0

  // Determine stack type
  let stackType: string
  let stackArgs = ''
  if (node.layout === 'vertical') {
    const alignment = alignToSwiftUI(node.alignItems, node.layout)
    const spacingStr = gapToSwiftUI(node.gap)
    const args: string[] = []
    if (alignment) args.push(`alignment: ${alignment}`)
    if (spacingStr) args.push(`spacing: ${spacingStr}`)
    stackType = 'VStack'
    if (args.length > 0) stackArgs = `(${args.join(', ')})`
  } else if (node.layout === 'horizontal') {
    const alignment = alignToSwiftUI(node.alignItems, node.layout)
    const spacingStr = gapToSwiftUI(node.gap)
    const args: string[] = []
    if (alignment) args.push(`alignment: ${alignment}`)
    if (spacingStr) args.push(`spacing: ${spacingStr}`)
    stackType = 'HStack'
    if (args.length > 0) stackArgs = `(${args.join(', ')})`
  } else {
    stackType = 'ZStack'
  }

  // Build modifiers
  const modifiers: string[] = []

  modifiers.push(...paddingToSwiftUI(node.padding))

  if (typeof node.width === 'number' || typeof node.height === 'number') {
    const args: string[] = []
    if (typeof node.width === 'number') args.push(`width: ${node.width}`)
    if (typeof node.height === 'number') args.push(`height: ${node.height}`)
    modifiers.push(`.frame(${args.join(', ')})`)
  }

  const fillStr = fillToSwiftUI(node.fill)
  if (fillStr) {
    if (cr > 0) {
      modifiers.push(`.background(${fillStr})`)
      modifiers.push(`.clipShape(RoundedRectangle(cornerRadius: ${cr}))`)
    } else {
      modifiers.push(`.background(${fillStr})`)
    }
  } else if (cr > 0) {
    modifiers.push(`.clipShape(RoundedRectangle(cornerRadius: ${cr}))`)
  }

  modifiers.push(...strokeToSwiftUI(node.stroke, node.cornerRadius))
  modifiers.push(...effectsToSwiftUI(node.effects))

  if (node.clipContent) {
    modifiers.push('.clipped()')
  }

  modifiers.push(...commonModifiers(node))

  // No children: render as a shape
  if (children.length === 0 && !hasLayout) {
    if (fillStr && cr > 0) {
      const shapeModifiers: string[] = []
      shapeModifiers.push(`.fill(${fillStr})`)
      if (typeof node.width === 'number' || typeof node.height === 'number') {
        const args: string[] = []
        if (typeof node.width === 'number') args.push(`width: ${node.width}`)
        if (typeof node.height === 'number') args.push(`height: ${node.height}`)
        shapeModifiers.push(`.frame(${args.join(', ')})`)
      }
      shapeModifiers.push(...strokeToSwiftUI(node.stroke, node.cornerRadius))
      shapeModifiers.push(...effectsToSwiftUI(node.effects))
      shapeModifiers.push(...commonModifiers(node))
      return renderWithModifiers(pad, `RoundedRectangle(cornerRadius: ${cr})`, shapeModifiers)
    }
    if (fillStr) {
      const shapeModifiers: string[] = []
      shapeModifiers.push(`.fill(${fillStr})`)
      if (typeof node.width === 'number' || typeof node.height === 'number') {
        const args: string[] = []
        if (typeof node.width === 'number') args.push(`width: ${node.width}`)
        if (typeof node.height === 'number') args.push(`height: ${node.height}`)
        shapeModifiers.push(`.frame(${args.join(', ')})`)
      }
      shapeModifiers.push(...strokeToSwiftUI(node.stroke, node.cornerRadius))
      shapeModifiers.push(...effectsToSwiftUI(node.effects))
      shapeModifiers.push(...commonModifiers(node))
      return renderWithModifiers(pad, 'Rectangle()', shapeModifiers)
    }
    // Empty container with just size/modifiers
    const emptyLines = [`${pad}${stackType}${stackArgs} {}`]
    for (const mod of modifiers) {
      emptyLines.push(`${pad}    ${mod}`)
    }
    return emptyLines.join('\n')
  }

  // With children
  const comment = node.name ? `${pad}// ${node.name}\n` : ''
  const childLines = children
    .map((c) => generateNodeSwiftUI(c, depth + 1))
    .join('\n')

  const lines = [`${comment}${pad}${stackType}${stackArgs} {`]
  lines.push(childLines)
  lines.push(`${pad}}`)
  for (const mod of modifiers) {
    lines.push(`${pad}    ${mod}`)
  }
  return lines.join('\n')
}

function gapToSwiftUI(gap: number | string | undefined): string | null {
  if (gap === undefined) return null
  if (typeof gap === 'string' && isVariableRef(gap)) {
    return `/* ${varOrLiteral(gap)} */ 0`
  }
  if (typeof gap === 'number' && gap > 0) {
    return String(gap)
  }
  return null
}

function generateTextSwiftUI(node: TextNode, depth: number): string {
  const pad = indent(depth)
  const text = escapeSwiftString(getTextContent(node))
  const modifiers: string[] = []

  // Font
  const weight = fontWeightToSwiftUI(node.fontWeight)
  if (node.fontSize && weight) {
    modifiers.push(`.font(.system(size: ${node.fontSize}, weight: ${weight}))`)
  } else if (node.fontSize) {
    modifiers.push(`.font(.system(size: ${node.fontSize}))`)
  }

  // Font style
  if (node.fontStyle === 'italic') {
    modifiers.push('.italic()')
  }

  // Text color
  if (node.fill && node.fill.length > 0) {
    const fill = node.fill[0]
    if (fill.type === 'solid') {
      modifiers.push(`.foregroundColor(${hexToSwiftUIColor(fill.color)})`)
    }
  }

  // Alignment
  const align = textAlignToSwiftUI(node.textAlign)
  if (align) {
    modifiers.push(`.multilineTextAlignment(${align})`)
  }

  // Frame / sizing
  if (typeof node.width === 'number' || typeof node.height === 'number') {
    const args: string[] = []
    if (typeof node.width === 'number') {
      args.push(`width: ${node.width}`)
    }
    if (typeof node.height === 'number') {
      args.push(`height: ${node.height}`)
    }
    if (node.textAlign === 'left') args.push('alignment: .leading')
    else if (node.textAlign === 'right') args.push('alignment: .trailing')
    modifiers.push(`.frame(${args.join(', ')})`)
  }

  // Letter spacing
  if (node.letterSpacing) {
    modifiers.push(`.kerning(${node.letterSpacing})`)
  }

  // Line height (approximation via lineSpacing)
  if (node.lineHeight && node.fontSize) {
    const spacing = node.lineHeight * node.fontSize - node.fontSize
    if (spacing > 0) {
      modifiers.push(`.lineSpacing(${spacing.toFixed(1)})`)
    }
  }

  // Decorations
  if (node.underline) {
    modifiers.push('.underline()')
  }
  if (node.strikethrough) {
    modifiers.push('.strikethrough()')
  }

  modifiers.push(...effectsToSwiftUI(node.effects))
  modifiers.push(...commonModifiers(node))

  return renderWithModifiers(pad, `Text("${text}")`, modifiers)
}

function generateLineSwiftUI(node: LineNode, depth: number): string {
  const pad = indent(depth)
  const w = node.x2 !== undefined ? Math.abs(node.x2 - (node.x ?? 0)) : 0
  const modifiers: string[] = []

  if (w > 0) {
    modifiers.push(`.frame(width: ${w}, height: 1)`)
  } else {
    modifiers.push('.frame(height: 1)')
  }

  if (node.stroke && node.stroke.fill && node.stroke.fill.length > 0) {
    const sf = node.stroke.fill[0]
    if (sf.type === 'solid') {
      modifiers.push(`.background(${hexToSwiftUIColor(sf.color)})`)
    }
  } else {
    modifiers.push('.background(Color.gray)')
  }

  modifiers.push(...commonModifiers(node))

  return renderWithModifiers(pad, 'Rectangle()', modifiers)
}

function generatePathSwiftUI(node: PathNode | PolygonNode, depth: number): string {
  const pad = indent(depth)

  if (node.type === 'path') {
    const fillStr = fillToSwiftUI(node.fill)
    const fillColor = fillStr ?? 'Color.primary'
    const modifiers: string[] = []

    if (typeof node.width === 'number' || typeof node.height === 'number') {
      const args: string[] = []
      if (typeof node.width === 'number') args.push(`width: ${node.width}`)
      if (typeof node.height === 'number') args.push(`height: ${node.height}`)
      modifiers.push(`.frame(${args.join(', ')})`)
    }
    modifiers.push(...effectsToSwiftUI(node.effects))
    modifiers.push(...commonModifiers(node))

    const escapedD = escapeSwiftString(node.d)

    const lines = [
      `${pad}// ${node.name ?? 'Path'}`,
      `${pad}SVGPath("${escapedD}")`,
      `${pad}    .fill(${fillColor})`,
    ]
    for (const mod of modifiers) {
      lines.push(`${pad}    ${mod}`)
    }
    return lines.join('\n')
  }

  // Polygon
  const modifiers: string[] = []
  const fillStr = fillToSwiftUI(node.fill)
  if (fillStr) {
    modifiers.push(`.fill(${fillStr})`)
  }
  if (typeof node.width === 'number' || typeof node.height === 'number') {
    const args: string[] = []
    if (typeof node.width === 'number') args.push(`width: ${node.width}`)
    if (typeof node.height === 'number') args.push(`height: ${node.height}`)
    modifiers.push(`.frame(${args.join(', ')})`)
  }
  modifiers.push(...effectsToSwiftUI(node.effects))
  modifiers.push(...commonModifiers(node))

  const sides = node.polygonCount
  return renderWithModifiers(pad, `PolygonShape(sides: ${sides})`, modifiers)
}

function generateImageSwiftUI(node: ImageNode, depth: number): string {
  const pad = indent(depth)
  const modifiers: string[] = []

  // Resizing
  modifiers.push('.resizable()')
  if (node.objectFit === 'fit') {
    modifiers.push('.aspectRatio(contentMode: .fit)')
  } else {
    modifiers.push('.aspectRatio(contentMode: .fill)')
  }

  if (typeof node.width === 'number' || typeof node.height === 'number') {
    const args: string[] = []
    if (typeof node.width === 'number') args.push(`width: ${node.width}`)
    if (typeof node.height === 'number') args.push(`height: ${node.height}`)
    modifiers.push(`.frame(${args.join(', ')})`)
  }

  if (node.cornerRadius) {
    const cr = typeof node.cornerRadius === 'number' ? node.cornerRadius : node.cornerRadius[0]
    if (cr > 0) {
      modifiers.push(`.clipShape(RoundedRectangle(cornerRadius: ${cr}))`)
    }
  }

  modifiers.push(...effectsToSwiftUI(node.effects))
  modifiers.push(...commonModifiers(node))

  const src = node.src

  // Data URI — extract base64 and decode at runtime
  if (src.startsWith('data:image/')) {
    const base64Start = src.indexOf('base64,')
    if (base64Start !== -1) {
      const base64Data = src.slice(base64Start + 7)
      const truncated = base64Data.length > 80 ? base64Data.substring(0, 80) + '...' : base64Data
      const lines = [
        `${pad}// Embedded image (${node.name ?? 'image'})`,
        `${pad}// Base64 data: ${truncated}`,
        `${pad}if let data = Data(base64Encoded: "${base64Data}"),`,
        `${pad}   let uiImage = UIImage(data: data) {`,
        `${pad}    Image(uiImage: uiImage)`,
      ]
      for (const mod of modifiers) {
        lines.push(`${pad}        ${mod}`)
      }
      lines.push(`${pad}}`)
      return lines.join('\n')
    }
  }

  const escapedSrc = escapeSwiftString(src)
  if (src.startsWith('http://') || src.startsWith('https://')) {
    const lines = [
      `${pad}AsyncImage(url: URL(string: "${escapedSrc}")) { image in`,
      `${pad}    image`,
    ]
    for (const mod of modifiers) {
      lines.push(`${pad}        ${mod}`)
    }
    lines.push(`${pad}} placeholder: {`)
    lines.push(`${pad}    ProgressView()`)
    lines.push(`${pad}}`)
    return lines.join('\n')
  }

  return renderWithModifiers(pad, `Image("${escapedSrc}")`, modifiers)
}

export function generateSwiftUICode(
  nodes: PenNode[],
  viewName = 'GeneratedView',
): string {
  if (nodes.length === 0) {
    return `import SwiftUI\n\nstruct ${viewName}: View {\n    var body: some View {\n        EmptyView()\n    }\n}\n`
  }

  // Compute wrapper size for root ZStack
  let maxW = 0
  let maxH = 0
  for (const node of nodes) {
    const x = node.x ?? 0
    const y = node.y ?? 0
    const w = 'width' in node && typeof node.width === 'number' ? node.width : 0
    const h = 'height' in node && typeof node.height === 'number' ? node.height : 0
    maxW = Math.max(maxW, x + w)
    maxH = Math.max(maxH, y + h)
  }

  const childLines = nodes
    .map((n) => generateNodeSwiftUI(n, 3))
    .join('\n')

  const frameArgs: string[] = []
  if (maxW > 0) frameArgs.push(`width: ${maxW}`)
  if (maxH > 0) frameArgs.push(`height: ${maxH}`)
  const frameModifier = frameArgs.length > 0 ? `\n        .frame(${frameArgs.join(', ')})` : ''

  return `import SwiftUI

/// Helper: parses SVG path data into a SwiftUI Shape.
/// Usage: SVGPath("M10 20 L30 40 Z").fill(.red)
struct SVGPath: Shape {
    let pathData: String
    init(_ pathData: String) { self.pathData = pathData }
    func path(in rect: CGRect) -> Path {
        // Use a third-party SVG path parser or implement command parsing
        // For production, consider using SwiftSVG or similar library
        Path { _ in /* parse pathData here */ }
    }
}

/// Helper: regular polygon shape
struct PolygonShape: Shape {
    let sides: Int
    func path(in rect: CGRect) -> Path {
        let center = CGPoint(x: rect.midX, y: rect.midY)
        let radius = min(rect.width, rect.height) / 2
        var path = Path()
        for i in 0..<sides {
            let angle = CGFloat(i) * (2 * .pi / CGFloat(sides)) - .pi / 2
            let point = CGPoint(x: center.x + radius * cos(angle), y: center.y + radius * sin(angle))
            if i == 0 { path.move(to: point) } else { path.addLine(to: point) }
        }
        path.closeSubpath()
        return path
    }
}

struct ${viewName}: View {
    var body: some View {
        ZStack(alignment: .topLeading) {
${childLines}
        }${frameModifier}
    }
}

#Preview {
    ${viewName}()
}
`
}

export function generateSwiftUIFromDocument(
  doc: PenDocument,
  activePageId?: string | null,
): string {
  const children = activePageId !== undefined
    ? getActivePageChildren(doc, activePageId)
    : doc.children
  return generateSwiftUICode(children, 'GeneratedView')
}
