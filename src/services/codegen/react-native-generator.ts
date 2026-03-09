import type { PenDocument, PenNode, ContainerProps, TextNode, ImageNode, LineNode, PathNode, PolygonNode } from '@/types/pen'
import { getActivePageChildren } from '@/stores/document-tree-utils'
import type { PenFill, PenStroke, PenEffect, ShadowEffect } from '@/types/styles'
import { isVariableRef } from '@/variables/resolve-variables'
import { variableNameToCSS } from '@/services/codegen/css-variables-generator'

/**
 * Converts PenDocument nodes to React Native code with inline styles.
 * $variable references are output as /* var(--name) *​/ comments.
 */

/** Convert a `$variable` ref to a comment placeholder, or return the raw value. */
function varOrLiteral(value: string): string {
  if (isVariableRef(value)) {
    return `var(${variableNameToCSS(value.slice(1))})`
  }
  return value
}

function indent(depth: number): string {
  return '  '.repeat(depth)
}

function kebabToPascal(name: string): string {
  return name.split('-').map(s => s.charAt(0).toUpperCase() + s.slice(1)).join('')
}

/** Return a hex color string, or a comment for variable refs. */
function hexColor(value: string): string {
  if (isVariableRef(value)) {
    return `/* ${varOrLiteral(value)} */ '#000000'`
  }
  return `'${value}'`
}

/** Extract backgroundColor from fills. */
function fillToStyle(fills: PenFill[] | undefined): Record<string, string> {
  if (!fills || fills.length === 0) return {}
  const fill = fills[0]
  if (fill.type === 'solid') {
    return { backgroundColor: hexColor(fill.color) }
  }
  return {}
}

/** Extract text color from fills. */
function fillToTextColor(fills: PenFill[] | undefined): Record<string, string> {
  if (!fills || fills.length === 0) return {}
  const fill = fills[0]
  if (fill.type === 'solid') {
    return { color: hexColor(fill.color) }
  }
  return {}
}

/** Extract border styles from stroke. */
function strokeToStyle(stroke: PenStroke | undefined): Record<string, string> {
  if (!stroke) return {}
  const styles: Record<string, string> = {}
  if (typeof stroke.thickness === 'string' && isVariableRef(stroke.thickness)) {
    styles.borderWidth = `/* ${varOrLiteral(stroke.thickness)} */ 1`
  } else {
    const thickness = typeof stroke.thickness === 'number'
      ? stroke.thickness
      : stroke.thickness[0]
    styles.borderWidth = String(thickness)
  }
  if (stroke.fill && stroke.fill.length > 0) {
    const sf = stroke.fill[0]
    if (sf.type === 'solid') {
      styles.borderColor = hexColor(sf.color)
    }
  }
  return styles
}

/** Extract shadow styles from effects. */
function effectsToStyle(effects: PenEffect[] | undefined): Record<string, string> {
  if (!effects || effects.length === 0) return {}
  const styles: Record<string, string> = {}
  for (const effect of effects) {
    if (effect.type === 'shadow') {
      const s = effect as ShadowEffect
      styles.shadowColor = `'${s.color}'`
      styles.shadowOffset = `{ width: ${s.offsetX}, height: ${s.offsetY} }`
      styles.shadowOpacity = '1'
      styles.shadowRadius = String(s.blur)
      styles.elevation = String(Math.max(1, Math.round(s.blur / 2)))
    }
  }
  return styles
}

/** Extract borderRadius styles from corner radius. */
function cornerRadiusToStyle(
  cr: number | [number, number, number, number] | undefined,
): Record<string, string> {
  if (cr === undefined) return {}
  if (typeof cr === 'number') {
    return cr === 0 ? {} : { borderRadius: String(cr) }
  }
  const [tl, tr, br, bl] = cr
  if (tl === tr && tr === br && br === bl) {
    return tl === 0 ? {} : { borderRadius: String(tl) }
  }
  return {
    borderTopLeftRadius: String(tl),
    borderTopRightRadius: String(tr),
    borderBottomRightRadius: String(br),
    borderBottomLeftRadius: String(bl),
  }
}

/** Extract layout styles from container props. */
function layoutToStyle(node: ContainerProps): Record<string, string> {
  const styles: Record<string, string> = {}
  if (node.layout === 'vertical') {
    styles.flexDirection = "'column'"
  } else if (node.layout === 'horizontal') {
    styles.flexDirection = "'row'"
  }
  if (node.gap !== undefined) {
    if (typeof node.gap === 'string' && isVariableRef(node.gap)) {
      styles.gap = `/* ${varOrLiteral(node.gap)} */ 0`
    } else if (typeof node.gap === 'number' && node.gap > 0) {
      styles.gap = String(node.gap)
    }
  }
  if (node.padding !== undefined) {
    if (typeof node.padding === 'string' && isVariableRef(node.padding)) {
      styles.padding = `/* ${varOrLiteral(node.padding)} */ 0`
    } else if (typeof node.padding === 'number') {
      styles.padding = String(node.padding)
    } else if (Array.isArray(node.padding)) {
      if (node.padding.length === 2) {
        styles.paddingVertical = String(node.padding[0])
        styles.paddingHorizontal = String(node.padding[1])
      } else if (node.padding.length === 4) {
        styles.paddingTop = String(node.padding[0])
        styles.paddingRight = String(node.padding[1])
        styles.paddingBottom = String(node.padding[2])
        styles.paddingLeft = String(node.padding[3])
      }
    }
  }
  if (node.justifyContent) {
    const jcMap: Record<string, string> = {
      start: "'flex-start'",
      center: "'center'",
      end: "'flex-end'",
      space_between: "'space-between'",
      space_around: "'space-around'",
    }
    if (jcMap[node.justifyContent]) styles.justifyContent = jcMap[node.justifyContent]
  }
  if (node.alignItems) {
    const aiMap: Record<string, string> = {
      start: "'flex-start'",
      center: "'center'",
      end: "'flex-end'",
    }
    if (aiMap[node.alignItems]) styles.alignItems = aiMap[node.alignItems]
  }
  if (node.clipContent) {
    styles.overflow = "'hidden'"
  }
  return styles
}

/** Extract text-specific styles. */
function textToStyle(node: TextNode): Record<string, string> {
  const styles: Record<string, string> = {}
  if (node.fontSize) styles.fontSize = String(node.fontSize)
  if (node.fontWeight) {
    const w = typeof node.fontWeight === 'number' ? node.fontWeight : parseInt(node.fontWeight, 10)
    if (!isNaN(w)) styles.fontWeight = `'${w}'`
  }
  if (node.fontStyle === 'italic') styles.fontStyle = "'italic'"
  if (node.textAlign) {
    const taMap: Record<string, string> = {
      left: "'left'",
      center: "'center'",
      right: "'right'",
    }
    if (taMap[node.textAlign]) styles.textAlign = taMap[node.textAlign]
  }
  if (node.fontFamily) styles.fontFamily = `'${node.fontFamily}'`
  if (node.letterSpacing) styles.letterSpacing = String(node.letterSpacing)
  if (node.lineHeight && node.fontSize) {
    styles.lineHeight = String(Math.round(node.fontSize * node.lineHeight))
  }
  if (node.underline && node.strikethrough) {
    styles.textDecorationLine = "'underline line-through'"
  } else if (node.underline) {
    styles.textDecorationLine = "'underline'"
  } else if (node.strikethrough) {
    styles.textDecorationLine = "'line-through'"
  }
  return styles
}

/** Format a style object as an inline style string. */
function formatStyle(styles: Record<string, string>): string {
  const entries = Object.entries(styles)
  if (entries.length === 0) return '{}'
  const parts = entries.map(([k, v]) => {
    // Values that are already quoted, numeric, or contain special syntax
    if (
      v.startsWith("'") ||
      v.startsWith('"') ||
      v.startsWith('{') ||
      v.startsWith('/*') ||
      /^-?\d+(\.\d+)?$/.test(v)
    ) {
      return `${k}: ${v}`
    }
    return `${k}: ${v}`
  })
  return `{ ${parts.join(', ')} }`
}

function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content
  return node.content.map((s) => s.text).join('')
}

function escapeJSX(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/{/g, '&#123;')
    .replace(/}/g, '&#125;')
}

/** Check if any node in the tree is a path or polygon. */
function hasSvgNodes(nodes: PenNode[]): boolean {
  for (const node of nodes) {
    if (node.type === 'path' || node.type === 'polygon') return true
    if ('children' in node && node.children) {
      if (hasSvgNodes(node.children)) return true
    }
  }
  return false
}

/** Collect common position/opacity/rotation styles. */
function commonStyles(node: PenNode): Record<string, string> {
  const styles: Record<string, string> = {}
  if (node.x !== undefined || node.y !== undefined) {
    styles.position = "'absolute'"
    if (node.x !== undefined) styles.left = String(node.x)
    if (node.y !== undefined) styles.top = String(node.y)
  }
  if (node.opacity !== undefined && node.opacity !== 1) {
    if (typeof node.opacity === 'string' && isVariableRef(node.opacity)) {
      styles.opacity = `/* ${varOrLiteral(node.opacity)} */ 1`
    } else if (typeof node.opacity === 'number') {
      styles.opacity = String(node.opacity)
    }
  }
  if (node.rotation) {
    styles.transform = `[{ rotate: '${node.rotation}deg' }]`
  }
  return styles
}

/** Main node renderer. Returns JSX string for a single node. */
function generateNodeRN(node: PenNode, depth: number): string {
  const pad = indent(depth)

  switch (node.type) {
    case 'frame':
    case 'rectangle':
    case 'group': {
      const styles: Record<string, string> = {
        ...commonStyles(node),
      }
      if (typeof node.width === 'number') styles.width = String(node.width)
      if (typeof node.height === 'number') styles.height = String(node.height)
      Object.assign(styles, fillToStyle(node.fill))
      Object.assign(styles, strokeToStyle(node.stroke))
      Object.assign(styles, cornerRadiusToStyle(node.cornerRadius))
      Object.assign(styles, effectsToStyle(node.effects))
      Object.assign(styles, layoutToStyle(node))

      const childNodes = node.children ?? []
      const comment = node.name ? `${pad}{/* ${node.name} */}\n` : ''
      if (childNodes.length === 0) {
        return `${comment}${pad}<View style=${formatStyle(styles)} />`
      }
      const childrenJSX = childNodes
        .map((c) => generateNodeRN(c, depth + 1))
        .join('\n')
      return `${comment}${pad}<View style=${formatStyle(styles)}>\n${childrenJSX}\n${pad}</View>`
    }

    case 'ellipse': {
      const w = typeof node.width === 'number' ? node.width : 100
      const h = typeof node.height === 'number' ? node.height : 100
      const styles: Record<string, string> = {
        ...commonStyles(node),
        width: String(w),
        height: String(h),
        borderRadius: String(Math.min(w, h) / 2),
      }
      Object.assign(styles, fillToStyle(node.fill))
      Object.assign(styles, strokeToStyle(node.stroke))
      Object.assign(styles, effectsToStyle(node.effects))
      return `${pad}<View style=${formatStyle(styles)} />`
    }

    case 'text': {
      const styles: Record<string, string> = {
        ...commonStyles(node),
      }
      if (typeof node.width === 'number') styles.width = String(node.width)
      if (typeof node.height === 'number') styles.height = String(node.height)
      Object.assign(styles, fillToTextColor(node.fill))
      Object.assign(styles, textToStyle(node))
      Object.assign(styles, effectsToStyle(node.effects))

      const text = escapeJSX(getTextContent(node))
      return `${pad}<Text style=${formatStyle(styles)}>${text}</Text>`
    }

    case 'line': {
      const lineNode = node as LineNode
      const w = lineNode.x2 !== undefined ? Math.abs(lineNode.x2 - (lineNode.x ?? 0)) : 0
      const styles: Record<string, string> = {
        ...commonStyles(node),
        width: String(w),
      }
      if (lineNode.stroke) {
        const thickness = typeof lineNode.stroke.thickness === 'number'
          ? lineNode.stroke.thickness
          : typeof lineNode.stroke.thickness === 'string'
            ? 1
            : lineNode.stroke.thickness[0]
        styles.height = String(thickness)
        if (lineNode.stroke.fill && lineNode.stroke.fill.length > 0) {
          const sf = lineNode.stroke.fill[0]
          if (sf.type === 'solid') {
            styles.backgroundColor = hexColor(sf.color)
          }
        } else {
          styles.backgroundColor = "'#999999'"
        }
      } else {
        styles.height = '1'
        styles.backgroundColor = "'#999999'"
      }
      return `${pad}<View style=${formatStyle(styles)} />`
    }

    case 'path': {
      const pathNode = node as PathNode
      const w = typeof pathNode.width === 'number' ? pathNode.width : 100
      const h = typeof pathNode.height === 'number' ? pathNode.height : 100
      const fillColor = pathNode.fill?.[0]?.type === 'solid'
        ? varOrLiteral(pathNode.fill[0].color)
        : 'currentColor'
      const posStyles = commonStyles(node)
      const posStr = Object.keys(posStyles).length > 0
        ? ` style=${formatStyle(posStyles)}`
        : ''
      const viewTag = Object.keys(posStyles).length > 0 ? 'View' : null
      const svgContent = [
        `${pad}${viewTag ? `<View${posStr}>` : ''}`,
        `${pad}${viewTag ? '  ' : ''}<Svg width={${w}} height={${h}} viewBox="0 0 ${w} ${h}">`,
        `${pad}${viewTag ? '    ' : '  '}<SvgPath d="${pathNode.d}" fill="${fillColor}" />`,
        `${pad}${viewTag ? '  ' : ''}</Svg>`,
      ]
      if (viewTag) svgContent.push(`${pad}</View>`)
      return svgContent.filter(Boolean).join('\n')
    }

    case 'polygon': {
      const polyNode = node as PolygonNode
      const w = typeof polyNode.width === 'number' ? polyNode.width : 100
      const h = typeof polyNode.height === 'number' ? polyNode.height : 100
      const fillColor = polyNode.fill?.[0]?.type === 'solid'
        ? varOrLiteral(polyNode.fill[0].color)
        : 'none'
      const sides = polyNode.polygonCount
      const points = polygonPoints(sides, w, h)
      const posStyles = commonStyles(node)
      const posStr = Object.keys(posStyles).length > 0
        ? ` style=${formatStyle(posStyles)}`
        : ''
      const viewTag = Object.keys(posStyles).length > 0 ? 'View' : null
      const svgContent = [
        `${pad}${viewTag ? `<View${posStr}>` : ''}`,
        `${pad}${viewTag ? '  ' : ''}<Svg width={${w}} height={${h}} viewBox="0 0 ${w} ${h}">`,
        `${pad}${viewTag ? '    ' : '  '}<SvgPolygon points="${points}" fill="${fillColor}" />`,
        `${pad}${viewTag ? '  ' : ''}</Svg>`,
      ]
      if (viewTag) svgContent.push(`${pad}</View>`)
      return svgContent.filter(Boolean).join('\n')
    }

    case 'image': {
      const imgNode = node as ImageNode
      const styles: Record<string, string> = {
        ...commonStyles(node),
      }
      if (typeof imgNode.width === 'number') styles.width = String(imgNode.width)
      if (typeof imgNode.height === 'number') styles.height = String(imgNode.height)
      if (imgNode.objectFit === 'fit') {
        styles.resizeMode = "'contain'"
      } else if (imgNode.objectFit === 'fill') {
        styles.resizeMode = "'stretch'"
      } else {
        styles.resizeMode = "'cover'"
      }
      Object.assign(styles, cornerRadiusToStyle(imgNode.cornerRadius))
      Object.assign(styles, effectsToStyle(imgNode.effects))

      const src = imgNode.src
      if (src.startsWith('http://') || src.startsWith('https://') || src.startsWith('data:')) {
        return `${pad}<Image source={{ uri: '${src}' }} style=${formatStyle(styles)} />`
      }
      return `${pad}<Image source={require('${src}')} style=${formatStyle(styles)} />`
    }

    case 'icon_font': {
      const size = typeof node.width === 'number' ? node.width : 24
      const color = node.fill?.[0]?.type === 'solid' ? varOrLiteral(node.fill[0].color) : 'currentColor'
      const iconComp = kebabToPascal(node.iconFontName || 'circle')
      return `${pad}<${iconComp} size={${size}} color="${color}" />`
    }

    case 'ref':
      return `${pad}{/* Ref: ${node.ref} */}`

    default:
      return `${pad}{/* Unknown node */}`
  }
}

/** Generate polygon points string for SVG. */
function polygonPoints(sides: number, w: number, h: number): string {
  const cx = w / 2
  const cy = h / 2
  const r = Math.min(w, h) / 2
  const points: string[] = []
  for (let i = 0; i < sides; i++) {
    const angle = (i * 2 * Math.PI) / sides - Math.PI / 2
    const px = cx + r * Math.cos(angle)
    const py = cy + r * Math.sin(angle)
    points.push(`${px.toFixed(1)},${py.toFixed(1)}`)
  }
  return points.join(' ')
}

export function generateReactNativeCode(
  nodes: PenNode[],
  componentName = 'GeneratedDesign',
): string {
  if (nodes.length === 0) {
    return [
      "import React from 'react'",
      "import { View } from 'react-native'",
      '',
      `export function ${componentName}() {`,
      '  return <View style={{ position: \'relative\' }} />',
      '}',
      '',
    ].join('\n')
  }

  // Compute bounding box for root wrapper
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

  const needsSvg = hasSvgNodes(nodes)

  // Build imports
  const rnImports = new Set<string>(['View'])
  collectImports(nodes, rnImports)
  const rnImportList = Array.from(rnImports).sort()

  const lines: string[] = [
    "import React from 'react'",
    `import { ${rnImportList.join(', ')} } from 'react-native'`,
  ]

  if (needsSvg) {
    const svgImports: string[] = ['default as Svg']
    if (hasNodeType(nodes, 'path')) svgImports.push('Path as SvgPath')
    if (hasNodeType(nodes, 'polygon')) svgImports.push('Polygon as SvgPolygon')
    lines.push(`import Svg, { ${svgImports.filter((s) => s !== 'default as Svg').join(', ')} } from 'react-native-svg'`)
  }

  lines.push('')

  // Root wrapper styles
  const rootStyles: Record<string, string> = { position: "'relative'" }
  if (maxW > 0) rootStyles.width = String(maxW)
  if (maxH > 0) rootStyles.height = String(maxH)

  const childrenJSX = nodes
    .map((n) => generateNodeRN(n, 2))
    .join('\n')

  lines.push(`export function ${componentName}() {`)
  lines.push('  return (')
  lines.push(`    <View style=${formatStyle(rootStyles)}>`)
  lines.push(childrenJSX)
  lines.push('    </View>')
  lines.push('  )')
  lines.push('}')
  lines.push('')

  return lines.join('\n')
}

/** Collect required react-native imports by walking the node tree. */
function collectImports(nodes: PenNode[], imports: Set<string>): void {
  for (const node of nodes) {
    switch (node.type) {
      case 'text':
        imports.add('Text')
        break
      case 'image':
        imports.add('Image')
        break
      case 'frame':
      case 'rectangle':
      case 'group':
      case 'ellipse':
      case 'line':
        // View is already included
        break
    }
    if ('children' in node && node.children) {
      collectImports(node.children, imports)
    }
  }
}

/** Check if any node in the tree matches a given type. */
function hasNodeType(nodes: PenNode[], type: string): boolean {
  for (const node of nodes) {
    if (node.type === type) return true
    if ('children' in node && node.children) {
      if (hasNodeType(node.children, type)) return true
    }
  }
  return false
}

export function generateReactNativeFromDocument(
  doc: PenDocument,
  activePageId?: string | null,
): string {
  const children = activePageId !== undefined
    ? getActivePageChildren(doc, activePageId)
    : doc.children
  return generateReactNativeCode(children, 'GeneratedDesign')
}
