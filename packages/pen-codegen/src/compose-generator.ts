import type { PenDocument, PenNode, ContainerProps, TextNode, ImageNode, EllipseNode, LineNode, PathNode, PolygonNode } from '@zseven-w/pen-types'
import { getActivePageChildren } from '@zseven-w/pen-core'
import type { PenFill, PenStroke, PenEffect, ShadowEffect } from '@zseven-w/pen-types'
import { isVariableRef } from '@zseven-w/pen-core'
import { variableNameToCSS } from './css-variables-generator.js'

/**
 * Converts PenDocument nodes to Jetpack Compose (Kotlin) code.
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

function kebabToPascal(name: string): string {
  return name.split('-').map(s => s.charAt(0).toUpperCase() + s.slice(1)).join('')
}

/** Parse a hex color string to Compose Color() call. */
function hexToComposeColor(hex: string): string {
  if (hex.startsWith('$')) {
    return `Color.Unspecified /* ${varOrLiteral(hex)} */`
  }
  const clean = hex.replace('#', '')
  if (clean.length === 6) {
    return `Color(0xFF${clean.toUpperCase()})`
  }
  if (clean.length === 8) {
    // RRGGBBAA -> AARRGGBB for Compose
    const rr = clean.substring(0, 2)
    const gg = clean.substring(2, 4)
    const bb = clean.substring(4, 6)
    const aa = clean.substring(6, 8)
    return `Color(0x${aa.toUpperCase()}${rr.toUpperCase()}${gg.toUpperCase()}${bb.toUpperCase()})`
  }
  return `Color.Unspecified /* ${hex} */`
}

function fillToComposeBackground(fills: PenFill[] | undefined): string | null {
  if (!fills || fills.length === 0) return null
  const fill = fills[0]
  if (fill.type === 'solid') {
    return hexToComposeColor(fill.color)
  }
  if (fill.type === 'linear_gradient') {
    if (!fill.stops?.length) return null
    const colors = fill.stops.map((s) => hexToComposeColor(s.color)).join(', ')
    return `Brush.linearGradient(listOf(${colors}))`
  }
  if (fill.type === 'radial_gradient') {
    if (!fill.stops?.length) return null
    const colors = fill.stops.map((s) => hexToComposeColor(s.color)).join(', ')
    return `Brush.radialGradient(listOf(${colors}))`
  }
  return null
}

function fillToComposeModifier(
  fills: PenFill[] | undefined,
  cornerRadius: number | [number, number, number, number] | undefined,
): string | null {
  if (!fills || fills.length === 0) return null
  const fill = fills[0]
  const shapeStr = cornerRadiusToComposeShape(cornerRadius)

  if (fill.type === 'solid') {
    const color = hexToComposeColor(fill.color)
    if (shapeStr) {
      return `.background(${color}, ${shapeStr})`
    }
    return `.background(${color})`
  }
  if (fill.type === 'linear_gradient') {
    if (!fill.stops?.length) return null
    const colors = fill.stops.map((s) => hexToComposeColor(s.color)).join(', ')
    const brush = `Brush.linearGradient(listOf(${colors}))`
    if (shapeStr) {
      return `.background(${brush}, ${shapeStr})`
    }
    return `.background(${brush})`
  }
  if (fill.type === 'radial_gradient') {
    if (!fill.stops?.length) return null
    const colors = fill.stops.map((s) => hexToComposeColor(s.color)).join(', ')
    const brush = `Brush.radialGradient(listOf(${colors}))`
    if (shapeStr) {
      return `.background(${brush}, ${shapeStr})`
    }
    return `.background(${brush})`
  }
  return null
}

function cornerRadiusToComposeShape(
  cr: number | [number, number, number, number] | undefined,
): string | null {
  if (cr === undefined) return null
  if (typeof cr === 'number') {
    if (cr === 0) return null
    return `RoundedCornerShape(${cr}.dp)`
  }
  const [tl, tr, br, bl] = cr
  if (tl === tr && tr === br && br === bl) {
    return tl === 0 ? null : `RoundedCornerShape(${tl}.dp)`
  }
  return `RoundedCornerShape(topStart = ${tl}.dp, topEnd = ${tr}.dp, bottomEnd = ${br}.dp, bottomStart = ${bl}.dp)`
}

function strokeToComposeModifier(
  stroke: PenStroke | undefined,
  cornerRadius: number | [number, number, number, number] | undefined,
): string | null {
  if (!stroke) return null
  const thickness = typeof stroke.thickness === 'number'
    ? stroke.thickness
    : typeof stroke.thickness === 'string'
      ? stroke.thickness
      : stroke.thickness[0]
  const thicknessStr = typeof thickness === 'string' && isVariableRef(thickness)
    ? `/* ${varOrLiteral(thickness)} */ 1.dp`
    : `${thickness}.dp`

  let strokeColor = 'Color.Gray'
  if (stroke.fill && stroke.fill.length > 0) {
    const sf = stroke.fill[0]
    if (sf.type === 'solid') {
      strokeColor = hexToComposeColor(sf.color)
    }
  }

  const shape = cornerRadiusToComposeShape(cornerRadius) ?? 'RectangleShape'
  return `.border(${thicknessStr}, ${strokeColor}, ${shape})`
}

function effectsToComposeModifier(effects: PenEffect[] | undefined): string[] {
  if (!effects || effects.length === 0) return []
  const modifiers: string[] = []
  for (const effect of effects) {
    if (effect.type === 'shadow') {
      const s = effect as ShadowEffect
      const shape = 'RoundedCornerShape(0.dp)'
      modifiers.push(`.shadow(elevation = ${s.blur}.dp, shape = ${shape})`)
    }
    // blur effects not directly supported as modifier; skip with comment
    if (effect.type === 'blur' || effect.type === 'background_blur') {
      modifiers.push(`// .blur(radius = ${effect.radius}.dp) — requires custom implementation`)
    }
  }
  return modifiers
}

function paddingToCompose(
  padding: number | [number, number] | [number, number, number, number] | string | undefined,
): string | null {
  if (padding === undefined) return null
  if (typeof padding === 'string' && isVariableRef(padding)) {
    return `.padding(/* ${varOrLiteral(padding)} */ 0.dp)`
  }
  if (typeof padding === 'number') {
    return padding > 0 ? `.padding(${padding}.dp)` : null
  }
  if (Array.isArray(padding)) {
    if (padding.length === 2) {
      return `.padding(vertical = ${padding[0]}.dp, horizontal = ${padding[1]}.dp)`
    }
    if (padding.length === 4) {
      const [top, end, bottom, start] = padding
      return `.padding(start = ${start}.dp, top = ${top}.dp, end = ${end}.dp, bottom = ${bottom}.dp)`
    }
  }
  return null
}

function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content
  return node.content.map((s) => s.text).join('')
}

function escapeKotlinString(text: string): string {
  return text
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\$/g, '\\$')
}

function fontWeightToCompose(weight: number | string | undefined): string | null {
  if (weight === undefined) return null
  const w = typeof weight === 'number' ? weight : parseInt(weight, 10)
  if (isNaN(w)) return null
  if (w <= 100) return 'FontWeight.Thin'
  if (w <= 200) return 'FontWeight.ExtraLight'
  if (w <= 300) return 'FontWeight.Light'
  if (w <= 400) return 'FontWeight.Normal'
  if (w <= 500) return 'FontWeight.Medium'
  if (w <= 600) return 'FontWeight.SemiBold'
  if (w <= 700) return 'FontWeight.Bold'
  if (w <= 800) return 'FontWeight.ExtraBold'
  return 'FontWeight.Black'
}

function textAlignToCompose(align: string | undefined): string | null {
  if (!align) return null
  const map: Record<string, string> = {
    left: 'TextAlign.Start',
    center: 'TextAlign.Center',
    right: 'TextAlign.End',
    justify: 'TextAlign.Justify',
  }
  return map[align] ?? null
}

/** Build the Modifier chain as a multi-line string. */
function buildModifierChain(modifiers: string[], pad: string): string {
  if (modifiers.length === 0) return 'Modifier'
  return `Modifier\n${modifiers.map((m) => `${pad}    ${m}`).join('\n')}`
}

/** Generate Compose code for a single node. */
function generateNodeCompose(node: PenNode, depth: number): string {
  const pad = indent(depth)

  switch (node.type) {
    case 'frame':
    case 'rectangle':
    case 'group':
      return generateContainerCompose(node, depth)

    case 'ellipse':
      return generateEllipseCompose(node, depth)

    case 'text':
      return generateTextCompose(node, depth)

    case 'line':
      return generateLineCompose(node, depth)

    case 'polygon':
    case 'path':
      return generatePathCompose(node, depth)

    case 'image':
      return generateImageCompose(node, depth)

    case 'icon_font': {
      const size = typeof node.width === 'number' ? node.width : 24
      const color = node.fill?.[0]?.type === 'solid' ? node.fill[0].color : null
      const iconName = kebabToPascal(node.iconFontName || 'circle')
      const colorStr = color ? `, tint = Color(0xFF${color.replace('#', '').toUpperCase()})` : ''
      return `${pad}Icon(LucideIcons.${iconName}, contentDescription = "${node.name ?? 'icon'}", modifier = Modifier.size(${size}.dp)${colorStr})`
    }

    case 'ref':
      return `${pad}// Ref: ${node.ref}`

    default:
      return `${pad}// Unsupported node type`
  }
}

function commonModifiers(node: PenNode): string[] {
  const modifiers: string[] = []

  if (node.x !== undefined || node.y !== undefined) {
    const x = node.x ?? 0
    const y = node.y ?? 0
    modifiers.push(`.offset(x = ${x}.dp, y = ${y}.dp)`)
  }

  if (node.rotation) {
    modifiers.push(`.rotate(${node.rotation}f)`)
  }

  if (node.opacity !== undefined && node.opacity !== 1) {
    if (typeof node.opacity === 'string' && isVariableRef(node.opacity)) {
      modifiers.push(`.alpha(/* ${varOrLiteral(node.opacity)} */ 1f)`)
    } else if (typeof node.opacity === 'number') {
      modifiers.push(`.alpha(${node.opacity}f)`)
    }
  }

  return modifiers
}

function generateContainerCompose(
  node: PenNode & ContainerProps,
  depth: number,
): string {
  const pad = indent(depth)
  const children = node.children ?? []
  const hasLayout = node.layout === 'vertical' || node.layout === 'horizontal'

  // Build modifier list
  const modParts: string[] = []
  modParts.push(...commonModifiers(node))

  if (typeof node.width === 'number' && typeof node.height === 'number') {
    modParts.push(`.size(width = ${node.width}.dp, height = ${node.height}.dp)`)
  } else if (typeof node.width === 'number') {
    modParts.push(`.width(${node.width}.dp)`)
  } else if (typeof node.height === 'number') {
    modParts.push(`.height(${node.height}.dp)`)
  }

  modParts.push(...effectsToComposeModifier(node.effects))

  const fillMod = fillToComposeModifier(node.fill, node.cornerRadius)
  if (fillMod) modParts.push(fillMod)
  else {
    const shape = cornerRadiusToComposeShape(node.cornerRadius)
    if (shape) modParts.push(`.clip(${shape})`)
  }

  const strokeMod = strokeToComposeModifier(node.stroke, node.cornerRadius)
  if (strokeMod) modParts.push(strokeMod)

  const paddingMod = paddingToCompose(node.padding)
  if (paddingMod) modParts.push(paddingMod)

  if (node.clipContent) {
    modParts.push('.clipToBounds()')
  }

  const modifierStr = buildModifierChain(modParts, pad)
  const comment = node.name ? `${pad}// ${node.name}\n` : ''

  // No children: just a Box
  if (children.length === 0 && !hasLayout) {
    return `${comment}${pad}Box(\n${pad}    modifier = ${modifierStr}\n${pad})`
  }

  const childLines = children
    .map((c) => generateNodeCompose(c, depth + 1))
    .join('\n')

  if (node.layout === 'vertical') {
    const arrangementParts: string[] = []
    const gapStr = gapToCompose(node.gap)
    if (gapStr) {
      arrangementParts.push(`verticalArrangement = Arrangement.spacedBy(${gapStr})`)
    }
    const alignment = alignToComposeHorizontal(node.alignItems)
    if (alignment) {
      arrangementParts.push(`horizontalAlignment = ${alignment}`)
    }

    const params = [`modifier = ${modifierStr}`]
    params.push(...arrangementParts)

    return `${comment}${pad}Column(\n${params.map((p) => `${pad}    ${p}`).join(',\n')}\n${pad}) {\n${childLines}\n${pad}}`
  }

  if (node.layout === 'horizontal') {
    const arrangementParts: string[] = []
    const gapStr = gapToCompose(node.gap)
    if (gapStr) {
      arrangementParts.push(`horizontalArrangement = Arrangement.spacedBy(${gapStr})`)
    }
    const alignment = alignToComposeVertical(node.alignItems)
    if (alignment) {
      arrangementParts.push(`verticalAlignment = ${alignment}`)
    }

    const params = [`modifier = ${modifierStr}`]
    params.push(...arrangementParts)

    return `${comment}${pad}Row(\n${params.map((p) => `${pad}    ${p}`).join(',\n')}\n${pad}) {\n${childLines}\n${pad}}`
  }

  // No layout or layout === 'none': use Box (ZStack equivalent)
  return `${comment}${pad}Box(\n${pad}    modifier = ${modifierStr}\n${pad}) {\n${childLines}\n${pad}}`
}

function gapToCompose(gap: number | string | undefined): string | null {
  if (gap === undefined) return null
  if (typeof gap === 'string' && isVariableRef(gap)) {
    return `/* ${varOrLiteral(gap)} */ 0.dp`
  }
  if (typeof gap === 'number' && gap > 0) {
    return `${gap}.dp`
  }
  return null
}

function alignToComposeHorizontal(alignItems: string | undefined): string | null {
  if (!alignItems) return null
  const map: Record<string, string> = {
    start: 'Alignment.Start',
    center: 'Alignment.CenterHorizontally',
    end: 'Alignment.End',
  }
  return map[alignItems] ?? null
}

function alignToComposeVertical(alignItems: string | undefined): string | null {
  if (!alignItems) return null
  const map: Record<string, string> = {
    start: 'Alignment.Top',
    center: 'Alignment.CenterVertically',
    end: 'Alignment.Bottom',
  }
  return map[alignItems] ?? null
}

function generateEllipseCompose(node: EllipseNode, depth: number): string {
  const pad = indent(depth)
  const modParts: string[] = []

  modParts.push(...commonModifiers(node))

  if (typeof node.width === 'number' && typeof node.height === 'number') {
    modParts.push(`.size(width = ${node.width}.dp, height = ${node.height}.dp)`)
  } else if (typeof node.width === 'number') {
    modParts.push(`.size(${node.width}.dp)`)
  } else if (typeof node.height === 'number') {
    modParts.push(`.size(${node.height}.dp)`)
  }

  modParts.push('.clip(CircleShape)')

  if (node.fill && node.fill.length > 0) {
    const fill = node.fill[0]
    if (fill.type === 'solid') {
      modParts.push(`.background(${hexToComposeColor(fill.color)})`)
    } else {
      const bgStr = fillToComposeBackground(node.fill)
      if (bgStr) modParts.push(`.background(${bgStr})`)
    }
  }

  const strokeMod = strokeToComposeModifier(node.stroke, undefined)
  if (strokeMod) modParts.push(strokeMod)

  modParts.push(...effectsToComposeModifier(node.effects))

  const modifierStr = buildModifierChain(modParts, pad)
  return `${pad}Box(\n${pad}    modifier = ${modifierStr}\n${pad})`
}

function generateTextCompose(node: TextNode, depth: number): string {
  const pad = indent(depth)
  const text = escapeKotlinString(getTextContent(node))
  const params: string[] = [`text = "${text}"`]

  // Font size
  if (node.fontSize) {
    params.push(`fontSize = ${node.fontSize}.sp`)
  }

  // Font weight
  const weight = fontWeightToCompose(node.fontWeight)
  if (weight) {
    params.push(`fontWeight = ${weight}`)
  }

  // Font style
  if (node.fontStyle === 'italic') {
    params.push('fontStyle = FontStyle.Italic')
  }

  // Color
  if (node.fill && node.fill.length > 0) {
    const fill = node.fill[0]
    if (fill.type === 'solid') {
      params.push(`color = ${hexToComposeColor(fill.color)}`)
    }
  }

  // Text alignment
  const align = textAlignToCompose(node.textAlign)
  if (align) {
    params.push(`textAlign = ${align}`)
  }

  // Font family
  if (node.fontFamily) {
    params.push(`fontFamily = FontFamily(Font(R.font.${node.fontFamily.toLowerCase().replace(/\s+/g, '_')}))`)
  }

  // Letter spacing
  if (node.letterSpacing) {
    params.push(`letterSpacing = ${node.letterSpacing}.sp`)
  }

  // Line height
  if (node.lineHeight && node.fontSize) {
    const lineHeightSp = node.lineHeight * node.fontSize
    params.push(`lineHeight = ${lineHeightSp.toFixed(1)}.sp`)
  }

  // Text decoration
  const decorations: string[] = []
  if (node.underline) decorations.push('TextDecoration.Underline')
  if (node.strikethrough) decorations.push('TextDecoration.LineThrough')
  if (decorations.length === 1) {
    params.push(`textDecoration = ${decorations[0]}`)
  } else if (decorations.length > 1) {
    params.push(`textDecoration = TextDecoration.combine(listOf(${decorations.join(', ')}))`)
  }

  // Build modifier for size, offset, opacity
  const modParts: string[] = []
  modParts.push(...commonModifiers(node))

  if (typeof node.width === 'number') {
    modParts.push(`.width(${node.width}.dp)`)
  }
  if (typeof node.height === 'number') {
    modParts.push(`.height(${node.height}.dp)`)
  }

  modParts.push(...effectsToComposeModifier(node.effects))

  if (modParts.length > 0) {
    const modifierStr = buildModifierChain(modParts, pad)
    params.push(`modifier = ${modifierStr}`)
  }

  if (params.length <= 2) {
    return `${pad}Text(${params.join(', ')})`
  }
  return `${pad}Text(\n${params.map((p) => `${pad}    ${p}`).join(',\n')}\n${pad})`
}

function generateLineCompose(node: LineNode, depth: number): string {
  const pad = indent(depth)
  const w = node.x2 !== undefined ? Math.abs(node.x2 - (node.x ?? 0)) : 0
  const modParts: string[] = []

  modParts.push(...commonModifiers(node))

  if (w > 0) {
    modParts.push(`.width(${w}.dp)`)
  }

  let strokeColor = 'Color.Gray'
  let thickness = '1'
  if (node.stroke) {
    const t = typeof node.stroke.thickness === 'number'
      ? node.stroke.thickness
      : typeof node.stroke.thickness === 'string'
        ? node.stroke.thickness
        : node.stroke.thickness[0]
    if (typeof t === 'string' && isVariableRef(t)) {
      thickness = `/* ${varOrLiteral(t)} */ 1`
    } else {
      thickness = String(t)
    }
    if (node.stroke.fill && node.stroke.fill.length > 0) {
      const sf = node.stroke.fill[0]
      if (sf.type === 'solid') {
        strokeColor = hexToComposeColor(sf.color)
      }
    }
  }

  const modifierStr = modParts.length > 0
    ? `,\n${pad}    modifier = ${buildModifierChain(modParts, pad)}`
    : ''

  return `${pad}Divider(\n${pad}    color = ${strokeColor},\n${pad}    thickness = ${thickness}.dp${modifierStr}\n${pad})`
}

function generatePathCompose(node: PathNode | PolygonNode, depth: number): string {
  const pad = indent(depth)

  if (node.type === 'path') {
    const fills = node.fill
    const fillColor = fills && fills.length > 0 && fills[0].type === 'solid'
      ? hexToComposeColor(fills[0].color)
      : 'Color.Black'

    const modParts: string[] = []
    modParts.push(...commonModifiers(node))
    if (typeof node.width === 'number' && typeof node.height === 'number') {
      modParts.push(`.size(width = ${node.width}.dp, height = ${node.height}.dp)`)
    } else if (typeof node.width === 'number') {
      modParts.push(`.width(${node.width}.dp)`)
    } else if (typeof node.height === 'number') {
      modParts.push(`.height(${node.height}.dp)`)
    }
    modParts.push(...effectsToComposeModifier(node.effects))

    const modifierStr = buildModifierChain(modParts, pad)
    const escapedD = escapeKotlinString(node.d)

    const lines = [
      `${pad}// ${node.name ?? 'Path'}`,
      `${pad}Canvas(`,
      `${pad}    modifier = ${modifierStr}`,
      `${pad}) {`,
      `${pad}    val pathData = "${escapedD}"`,
      `${pad}    val path = PathParser().parsePathString(pathData).toPath()`,
      `${pad}    drawPath(path, color = ${fillColor})`,
      `${pad}}`,
    ]
    return lines.join('\n')
  }

  // Polygon
  const modParts: string[] = []
  modParts.push(...commonModifiers(node))

  if (typeof node.width === 'number' && typeof node.height === 'number') {
    modParts.push(`.size(width = ${node.width}.dp, height = ${node.height}.dp)`)
  }
  modParts.push(...effectsToComposeModifier(node.effects))

  const fillColor = node.fill && node.fill.length > 0 && node.fill[0].type === 'solid'
    ? hexToComposeColor(node.fill[0].color)
    : 'Color.Black'

  const modifierStr = buildModifierChain(modParts, pad)
  const sides = node.polygonCount

  const lines = [
    `${pad}// Polygon (${sides}-sided)`,
    `${pad}Canvas(`,
    `${pad}    modifier = ${modifierStr}`,
    `${pad}) {`,
    `${pad}    val center = Offset(size.width / 2, size.height / 2)`,
    `${pad}    val radius = minOf(size.width, size.height) / 2`,
    `${pad}    val path = Path().apply {`,
    `${pad}        for (i in 0 until ${sides}) {`,
    `${pad}            val angle = i * (2 * Math.PI / ${sides}).toFloat() - (Math.PI / 2).toFloat()`,
    `${pad}            val x = center.x + radius * cos(angle)`,
    `${pad}            val y = center.y + radius * sin(angle)`,
    `${pad}            if (i == 0) moveTo(x, y) else lineTo(x, y)`,
    `${pad}        }`,
    `${pad}        close()`,
    `${pad}    }`,
    `${pad}    drawPath(path, color = ${fillColor})`,
    `${pad}}`,
  ]
  return lines.join('\n')
}

function generateImageCompose(node: ImageNode, depth: number): string {
  const pad = indent(depth)
  const modParts: string[] = []

  modParts.push(...commonModifiers(node))

  if (typeof node.width === 'number' && typeof node.height === 'number') {
    modParts.push(`.size(width = ${node.width}.dp, height = ${node.height}.dp)`)
  } else if (typeof node.width === 'number') {
    modParts.push(`.width(${node.width}.dp)`)
  } else if (typeof node.height === 'number') {
    modParts.push(`.height(${node.height}.dp)`)
  }

  if (node.cornerRadius) {
    const shape = cornerRadiusToComposeShape(node.cornerRadius)
    if (shape) modParts.push(`.clip(${shape})`)
  }

  modParts.push(...effectsToComposeModifier(node.effects))

  const modifierStr = buildModifierChain(modParts, pad)
  const src = node.src

  const contentScale = node.objectFit === 'fit'
    ? 'ContentScale.Fit'
    : node.objectFit === 'crop'
      ? 'ContentScale.Crop'
      : 'ContentScale.FillBounds'

  // Data URI — decode base64 at runtime
  if (src.startsWith('data:image/')) {
    const base64Start = src.indexOf('base64,')
    if (base64Start !== -1) {
      const base64Data = src.slice(base64Start + 7)
      const truncated = base64Data.length > 80 ? base64Data.substring(0, 80) + '...' : base64Data
      const lines = [
        `${pad}// Embedded image (${node.name ?? 'image'})`,
        `${pad}// Base64 data: ${truncated}`,
        `${pad}val bytes = Base64.decode("${escapeKotlinString(base64Data)}", Base64.DEFAULT)`,
        `${pad}val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)`,
        `${pad}Image(`,
        `${pad}    bitmap = bitmap.asImageBitmap(),`,
        `${pad}    contentDescription = ${node.name ? `"${escapeKotlinString(node.name)}"` : 'null'},`,
        `${pad}    modifier = ${modifierStr},`,
        `${pad}    contentScale = ${contentScale}`,
        `${pad})`,
      ]
      return lines.join('\n')
    }
  }

  const escapedSrc = escapeKotlinString(src)
  if (escapedSrc.startsWith('http://') || escapedSrc.startsWith('https://')) {
    // Use Coil's AsyncImage for remote URLs
    const lines = [
      `${pad}AsyncImage(`,
      `${pad}    model = "${escapedSrc}",`,
      `${pad}    contentDescription = ${node.name ? `"${escapeKotlinString(node.name)}"` : 'null'},`,
      `${pad}    modifier = ${modifierStr},`,
      `${pad}    contentScale = ${contentScale}`,
      `${pad})`,
    ]
    return lines.join('\n')
  }

  const lines = [
    `${pad}Image(`,
    `${pad}    painter = painterResource(id = R.drawable.${escapedSrc.replace(/[^a-zA-Z0-9_]/g, '_')}),`,
    `${pad}    contentDescription = ${node.name ? `"${escapeKotlinString(node.name)}"` : 'null'},`,
    `${pad}    modifier = ${modifierStr},`,
    `${pad}    contentScale = ${contentScale}`,
    `${pad})`,
  ]
  return lines.join('\n')
}

/** @deprecated Use AI code generation pipeline instead. Will be removed in v1.0.0. */
export function generateComposeCode(
  nodes: PenNode[],
  composableName = 'GeneratedDesign',
): string {
  if (nodes.length === 0) {
    return `@Composable\nfun ${composableName}() {\n    // Empty design\n}\n`
  }

  // Compute wrapper size
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
    .map((n) => generateNodeCompose(n, 2))
    .join('\n')

  const sizeMods: string[] = []
  if (maxW > 0 && maxH > 0) {
    sizeMods.push(`.size(width = ${maxW}.dp, height = ${maxH}.dp)`)
  } else if (maxW > 0) {
    sizeMods.push(`.width(${maxW}.dp)`)
  } else if (maxH > 0) {
    sizeMods.push(`.height(${maxH}.dp)`)
  }

  const modifierStr = sizeMods.length > 0
    ? `Modifier\n${sizeMods.map((m) => `            ${m}`).join('\n')}`
    : 'Modifier'

  return `import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Divider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun ${composableName}() {
    Box(
        modifier = ${modifierStr}
    ) {
${childLines}
    }
}
`
}

/** @deprecated Use AI code generation pipeline instead. Will be removed in v1.0.0. */
export function generateComposeFromDocument(
  doc: PenDocument,
  activePageId?: string | null,
): string {
  const children = activePageId !== undefined
    ? getActivePageChildren(doc, activePageId)
    : doc.children
  return generateComposeCode(children, 'GeneratedDesign')
}
