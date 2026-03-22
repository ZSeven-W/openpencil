import type { PenDocument, PenNode, ContainerProps, TextNode, ImageNode, EllipseNode, LineNode, PathNode, PolygonNode } from '@zseven-w/pen-types'
import { getActivePageChildren } from '@zseven-w/pen-core'
import type { PenFill, PenStroke, PenEffect, ShadowEffect, BlurEffect } from '@zseven-w/pen-types'
import { isVariableRef } from '@zseven-w/pen-core'
import { variableNameToCSS } from './css-variables-generator.js'

/**
 * Converts PenDocument nodes to Flutter (Dart) code.
 * $variable references are output as var(--name) comments for manual mapping.
 */

function varOrLiteral(value: string): string {
  if (isVariableRef(value)) return `var(${variableNameToCSS(value.slice(1))})`
  return value
}

function indent(depth: number): string {
  return '  '.repeat(depth)
}

function hexToFlutterColor(hex: string): string {
  if (hex.startsWith('$')) return `Color(0x00000000) /* ${varOrLiteral(hex)} */`
  const clean = hex.replace('#', '')
  if (clean.length === 6) return `Color(0xFF${clean.toUpperCase()})`
  if (clean.length === 8) {
    const [rr, gg, bb, aa] = [clean.substring(0, 2), clean.substring(2, 4), clean.substring(4, 6), clean.substring(6, 8)]
    return `Color(0x${aa.toUpperCase()}${rr.toUpperCase()}${gg.toUpperCase()}${bb.toUpperCase()})`
  }
  return `Color(0x00000000) /* ${hex} */`
}

function escapeDartString(text: string): string {
  return text.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/\$/g, '\\$').replace(/\n/g, '\\n')
}

function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content
  return node.content.map((s) => s.text).join('')
}

function fillToDecoration(fills: PenFill[] | undefined): { color?: string; gradient?: string } | null {
  if (!fills || fills.length === 0) return null
  const fill = fills[0]
  if (fill.type === 'solid') return { color: hexToFlutterColor(fill.color) }
  if (fill.type === 'linear_gradient') {
    if (!fill.stops?.length) return null
    const colors = fill.stops.map((s) => hexToFlutterColor(s.color)).join(', ')
    return { gradient: `LinearGradient(colors: [${colors}])` }
  }
  if (fill.type === 'radial_gradient') {
    if (!fill.stops?.length) return null
    const colors = fill.stops.map((s) => hexToFlutterColor(s.color)).join(', ')
    return { gradient: `RadialGradient(colors: [${colors}])` }
  }
  return null
}

function fillColorOnly(fills: PenFill[] | undefined): string | null {
  if (!fills || fills.length === 0) return null
  const fill = fills[0]
  return fill.type === 'solid' ? hexToFlutterColor(fill.color) : null
}

function cornerRadiusToFlutter(cr: number | [number, number, number, number] | undefined): string | null {
  if (cr === undefined) return null
  if (typeof cr === 'number') return cr > 0 ? `BorderRadius.circular(${cr})` : null
  const [tl, tr, br, bl] = cr
  if (tl === tr && tr === br && br === bl) return tl > 0 ? `BorderRadius.circular(${tl})` : null
  return `BorderRadius.only(topLeft: Radius.circular(${tl}), topRight: Radius.circular(${tr}), bottomRight: Radius.circular(${br}), bottomLeft: Radius.circular(${bl}))`
}

function strokeToFlutterBorder(stroke: PenStroke | undefined): string | null {
  if (!stroke) return null
  const thickness = typeof stroke.thickness === 'number'
    ? stroke.thickness
    : typeof stroke.thickness === 'string' ? stroke.thickness : stroke.thickness[0]
  const thicknessStr = typeof thickness === 'string' && isVariableRef(thickness)
    ? `/* ${varOrLiteral(thickness)} */ 1` : String(thickness)
  let strokeColor = 'Colors.grey'
  if (stroke.fill && stroke.fill.length > 0 && stroke.fill[0].type === 'solid') {
    strokeColor = hexToFlutterColor(stroke.fill[0].color)
  }
  return `Border.all(color: ${strokeColor}, width: ${thicknessStr})`
}

function effectsToBoxShadows(effects: PenEffect[] | undefined): string[] {
  if (!effects || effects.length === 0) return []
  const shadows: string[] = []
  for (const effect of effects) {
    if (effect.type === 'shadow') {
      const s = effect as ShadowEffect
      shadows.push(`BoxShadow(color: ${hexToFlutterColor(s.color)}, blurRadius: ${s.blur}, offset: Offset(${s.offsetX}, ${s.offsetY}))`)
    }
  }
  return shadows
}

function hasBlurEffect(effects: PenEffect[] | undefined): BlurEffect | null {
  if (!effects) return null
  const found = effects.find((e) => e.type === 'blur' || e.type === 'background_blur')
  return found ? (found as BlurEffect) : null
}

function paddingToFlutter(
  padding: number | [number, number] | [number, number, number, number] | string | undefined,
): string | null {
  if (padding === undefined) return null
  if (typeof padding === 'string' && isVariableRef(padding)) return `EdgeInsets.all(/* ${varOrLiteral(padding)} */ 0)`
  if (typeof padding === 'number') return padding > 0 ? `EdgeInsets.all(${padding})` : null
  if (Array.isArray(padding)) {
    if (padding.length === 2) return `EdgeInsets.symmetric(vertical: ${padding[0]}, horizontal: ${padding[1]})`
    if (padding.length === 4) {
      const [top, right, bottom, left] = padding
      return `EdgeInsets.fromLTRB(${left}, ${top}, ${right}, ${bottom})`
    }
  }
  return null
}

function crossAxisToFlutter(alignItems: string | undefined): string | null {
  if (!alignItems) return null
  const m: Record<string, string> = { start: 'CrossAxisAlignment.start', center: 'CrossAxisAlignment.center', end: 'CrossAxisAlignment.end' }
  return m[alignItems] ?? null
}

function mainAxisToFlutter(justifyContent: string | undefined): string | null {
  if (!justifyContent) return null
  const m: Record<string, string> = {
    start: 'MainAxisAlignment.start', center: 'MainAxisAlignment.center', end: 'MainAxisAlignment.end',
    space_between: 'MainAxisAlignment.spaceBetween', space_around: 'MainAxisAlignment.spaceAround',
  }
  return m[justifyContent] ?? null
}

function fontWeightToFlutter(weight: number | string | undefined): string | null {
  if (weight === undefined) return null
  const w = typeof weight === 'number' ? weight : parseInt(weight, 10)
  if (isNaN(w)) return null
  if (w <= 100) return 'FontWeight.w100'
  if (w <= 200) return 'FontWeight.w200'
  if (w <= 300) return 'FontWeight.w300'
  if (w <= 400) return 'FontWeight.w400'
  if (w <= 500) return 'FontWeight.w500'
  if (w <= 600) return 'FontWeight.w600'
  if (w <= 700) return 'FontWeight.w700'
  if (w <= 800) return 'FontWeight.w800'
  return 'FontWeight.w900'
}

function textAlignToFlutter(align: string | undefined): string | null {
  if (!align) return null
  const m: Record<string, string> = { left: 'TextAlign.left', center: 'TextAlign.center', right: 'TextAlign.right', justify: 'TextAlign.justify' }
  return m[align] ?? null
}

function buildBoxDecoration(
  fills: PenFill[] | undefined, cornerRadius: number | [number, number, number, number] | undefined,
  stroke: PenStroke | undefined, effects: PenEffect[] | undefined, pad: string,
): string | null {
  const fillResult = fillToDecoration(fills)
  const borderRadius = cornerRadiusToFlutter(cornerRadius)
  const border = strokeToFlutterBorder(stroke)
  const shadows = effectsToBoxShadows(effects)
  if (!fillResult && !borderRadius && !border && shadows.length === 0) return null

  const parts: string[] = []
  if (fillResult?.color) parts.push(`${pad}    color: ${fillResult.color},`)
  if (fillResult?.gradient) parts.push(`${pad}    gradient: ${fillResult.gradient},`)
  if (borderRadius) parts.push(`${pad}    borderRadius: ${borderRadius},`)
  if (border) parts.push(`${pad}    border: ${border},`)
  if (shadows.length > 0) {
    parts.push(`${pad}    boxShadow: [`)
    for (const s of shadows) parts.push(`${pad}      ${s},`)
    parts.push(`${pad}    ],`)
  }
  return `BoxDecoration(\n${parts.join('\n')}\n${pad}  )`
}

// Wrapper helpers
function wrapOpacity(widget: string, node: PenNode, depth: number): string {
  if (node.opacity === undefined || node.opacity === 1) return widget
  const pad = indent(depth)
  if (typeof node.opacity === 'string' && isVariableRef(node.opacity))
    return `${pad}Opacity(\n${pad}  opacity: /* ${varOrLiteral(node.opacity)} */ 1.0,\n${pad}  child: ${widget.trimStart()},\n${pad})`
  if (typeof node.opacity === 'number')
    return `${pad}Opacity(\n${pad}  opacity: ${node.opacity},\n${pad}  child: ${widget.trimStart()},\n${pad})`
  return widget
}

function wrapRotation(widget: string, node: PenNode, depth: number): string {
  if (!node.rotation) return widget
  const pad = indent(depth)
  return `${pad}Transform.rotate(\n${pad}  angle: ${node.rotation} * pi / 180,\n${pad}  child: ${widget.trimStart()},\n${pad})`
}

function wrapBlur(widget: string, effects: PenEffect[] | undefined, depth: number): string {
  const blur = hasBlurEffect(effects)
  if (!blur) return widget
  const pad = indent(depth)
  const r = blur.radius ?? 0
  return `${pad}BackdropFilter(\n${pad}  filter: ImageFilter.blur(sigmaX: ${r}, sigmaY: ${r}),\n${pad}  child: ${widget.trimStart()},\n${pad})`
}

function applyWrappers(widget: string, node: PenNode, depth: number): string {
  let result = wrapBlur(widget, (node as any).effects, depth)
  result = wrapOpacity(result, node, depth)
  result = wrapRotation(result, node, depth)
  return result
}

// Node generators
function generateNodeFlutter(node: PenNode, depth: number): string {
  switch (node.type) {
    case 'frame': case 'rectangle': case 'group': return generateContainerFlutter(node, depth)
    case 'ellipse': return generateEllipseFlutter(node as EllipseNode, depth)
    case 'text': return generateTextFlutter(node as TextNode, depth)
    case 'line': return generateLineFlutter(node as LineNode, depth)
    case 'path': return generatePathFlutter(node as PathNode, depth)
    case 'polygon': return generatePolygonFlutter(node as PolygonNode, depth)
    case 'image': return generateImageFlutter(node as ImageNode, depth)
    case 'icon_font': {
      const size = typeof node.width === 'number' ? node.width : 24
      const color = node.fill?.[0]?.type === 'solid' ? node.fill[0].color : null
      const iconName = (node.iconFontName || 'circle').replace(/-/g, '_')
      const colorStr = color ? `, color: Color(0xFF${color.replace('#', '')})` : ''
      return `${indent(depth)}Icon(LucideIcons.${iconName}, size: ${size}${colorStr})`
    }
    case 'ref': return `${indent(depth)}// Ref: ${(node as any).ref}`
    default: return `${indent(depth)}// Unsupported node type`
  }
}

function generateContainerFlutter(node: PenNode & ContainerProps, depth: number): string {
  const pad = indent(depth)
  const children = node.children ?? []
  const hasLayout = node.layout === 'vertical' || node.layout === 'horizontal'
  const gap = typeof node.gap === 'number' ? node.gap : 0
  const gapIsVar = typeof node.gap === 'string' && isVariableRef(node.gap)
  const gapComment = gapIsVar ? ` /* ${varOrLiteral(node.gap as string)} */` : ''
  const decoration = buildBoxDecoration(node.fill, node.cornerRadius, node.stroke, node.effects, pad)
  const paddingStr = paddingToFlutter(node.padding)
  const comment = node.name ? `${pad}// ${node.name}\n` : ''

  let innerWidget: string

  if (children.length === 0 && !hasLayout) {
    innerWidget = buildContainer(pad, decoration, paddingStr, node, null)
  } else if (hasLayout) {
    const isVertical = node.layout === 'vertical'
    const layoutType = isVertical ? 'Column' : 'Row'
    const crossAxis = crossAxisToFlutter(node.alignItems)
    const mainAxis = mainAxisToFlutter(node.justifyContent)
    const layoutParams: string[] = []
    if (mainAxis) layoutParams.push(`${pad}    mainAxisAlignment: ${mainAxis},`)
    if (crossAxis) layoutParams.push(`${pad}    crossAxisAlignment: ${crossAxis},`)
    layoutParams.push(`${pad}    mainAxisSize: MainAxisSize.min,`)

    const childWidgets: string[] = []
    for (let i = 0; i < children.length; i++) {
      childWidgets.push(generateNodeFlutter(children[i], depth + 2))
      if (i < children.length - 1 && (gap > 0 || gapIsVar)) {
        const spacer = isVertical
          ? `${indent(depth + 2)}SizedBox(height: ${gapIsVar ? `0${gapComment}` : gap}),`
          : `${indent(depth + 2)}SizedBox(width: ${gapIsVar ? `0${gapComment}` : gap}),`
        childWidgets.push(spacer)
      }
    }
    const layoutWidget = [
      `${pad}  ${layoutType}(`, ...layoutParams,
      `${pad}    children: [`, ...childWidgets.map((c) => c + ','),
      `${pad}    ],`, `${pad}  )`,
    ].join('\n')
    innerWidget = buildContainer(pad, decoration, paddingStr, node, layoutWidget)
  } else {
    const childWidgets = children.map((c) => {
      const childStr = generateNodeFlutter(c, depth + 3)
      const cx = c.x ?? 0, cy = c.y ?? 0
      if (cx !== 0 || cy !== 0) {
        const cPad = indent(depth + 2)
        return `${cPad}Positioned(\n${cPad}  left: ${cx},\n${cPad}  top: ${cy},\n${cPad}  child: ${childStr.trimStart()},\n${cPad})`
      }
      return childStr
    })
    const stackWidget = [
      `${pad}  Stack(`, `${pad}    children: [`,
      ...childWidgets.map((c) => c + ','), `${pad}    ],`, `${pad}  )`,
    ].join('\n')
    innerWidget = buildContainer(pad, decoration, paddingStr, node, stackWidget)
  }

  return `${comment}${applyWrappers(innerWidget, node, depth)}`
}

function buildContainer(
  pad: string, decoration: string | null, paddingStr: string | null,
  node: PenNode & ContainerProps, child: string | null,
): string {
  const parts: string[] = [`${pad}Container(`]
  if (typeof node.width === 'number') parts.push(`${pad}  width: ${node.width},`)
  if (typeof node.height === 'number') parts.push(`${pad}  height: ${node.height},`)
  if (paddingStr) parts.push(`${pad}  padding: ${paddingStr},`)
  if (decoration) parts.push(`${pad}  decoration: ${decoration},`)
  if (node.clipContent) parts.push(`${pad}  clipBehavior: Clip.hardEdge,`)
  if (child) parts.push(`${pad}  child: ${child.trimStart()},`)
  parts.push(`${pad})`)
  return parts.join('\n')
}

function generateEllipseFlutter(node: EllipseNode, depth: number): string {
  const pad = indent(depth)
  const w = typeof node.width === 'number' ? node.width : undefined
  const h = typeof node.height === 'number' ? node.height : undefined
  const fillResult = fillToDecoration(node.fill)
  const shadows = effectsToBoxShadows(node.effects)
  const border = strokeToFlutterBorder(node.stroke)

  const decParts: string[] = [`${pad}    shape: BoxShape.circle,`]
  if (fillResult?.color) decParts.push(`${pad}    color: ${fillResult.color},`)
  if (fillResult?.gradient) decParts.push(`${pad}    gradient: ${fillResult.gradient},`)
  if (border) decParts.push(`${pad}    border: ${border},`)
  if (shadows.length > 0) {
    decParts.push(`${pad}    boxShadow: [`)
    for (const s of shadows) decParts.push(`${pad}      ${s},`)
    decParts.push(`${pad}    ],`)
  }

  const parts: string[] = [`${pad}Container(`]
  if (w !== undefined) parts.push(`${pad}  width: ${w},`)
  if (h !== undefined) parts.push(`${pad}  height: ${h},`)
  parts.push(`${pad}  decoration: BoxDecoration(\n${decParts.join('\n')}\n${pad}  ),`)
  parts.push(`${pad})`)
  return applyWrappers(parts.join('\n'), node, depth)
}

function generateTextFlutter(node: TextNode, depth: number): string {
  const pad = indent(depth)
  const text = escapeDartString(getTextContent(node))
  const styleParts: string[] = []
  if (node.fontSize) styleParts.push(`fontSize: ${node.fontSize}`)
  const fw = fontWeightToFlutter(node.fontWeight)
  if (fw) styleParts.push(`fontWeight: ${fw}`)
  if (node.fontStyle === 'italic') styleParts.push('fontStyle: FontStyle.italic')
  if (node.fontFamily) styleParts.push(`fontFamily: '${escapeDartString(node.fontFamily)}'`)
  if (node.letterSpacing) styleParts.push(`letterSpacing: ${node.letterSpacing}`)
  if (node.lineHeight && node.fontSize) styleParts.push(`height: ${node.lineHeight}`)
  const textColor = fillColorOnly(node.fill)
  if (textColor) styleParts.push(`color: ${textColor}`)

  const decorations: string[] = []
  if (node.underline) decorations.push('TextDecoration.underline')
  if (node.strikethrough) decorations.push('TextDecoration.lineThrough')
  if (decorations.length === 1) styleParts.push(`decoration: ${decorations[0]}`)
  else if (decorations.length > 1) styleParts.push(`decoration: TextDecoration.combine([${decorations.join(', ')}])`)

  const textAlign = textAlignToFlutter(node.textAlign)
  const params: string[] = [`${pad}  '${text}'`]
  if (textAlign) params.push(`${pad}  textAlign: ${textAlign}`)
  if (styleParts.length > 0) params.push(`${pad}  style: TextStyle(${styleParts.join(', ')})`)

  const textWidget = `${pad}Text(\n${params.join(',\n')},\n${pad})`
  let widget: string
  if (typeof node.width === 'number' || typeof node.height === 'number') {
    const sp: string[] = [`${pad}SizedBox(`]
    if (typeof node.width === 'number') sp.push(`${pad}  width: ${node.width},`)
    if (typeof node.height === 'number') sp.push(`${pad}  height: ${node.height},`)
    sp.push(`${pad}  child: ${textWidget.trimStart()},`)
    sp.push(`${pad})`)
    widget = sp.join('\n')
  } else {
    widget = textWidget
  }
  return applyWrappers(widget, node, depth)
}

function generateLineFlutter(node: LineNode, depth: number): string {
  const pad = indent(depth)
  const w = node.x2 !== undefined ? Math.abs(node.x2 - (node.x ?? 0)) : undefined
  const thickness = node.stroke
    ? (typeof node.stroke.thickness === 'number' ? node.stroke.thickness
      : typeof node.stroke.thickness === 'string' ? 1 : node.stroke.thickness[0])
    : 1
  let color = 'Colors.grey'
  if (node.stroke?.fill && node.stroke.fill.length > 0 && node.stroke.fill[0].type === 'solid')
    color = hexToFlutterColor(node.stroke.fill[0].color)

  const parts: string[] = [`${pad}Container(`]
  if (w !== undefined) parts.push(`${pad}  width: ${w},`)
  parts.push(`${pad}  height: ${thickness},`)
  parts.push(`${pad}  color: ${color},`)
  parts.push(`${pad})`)
  return applyWrappers(parts.join('\n'), node, depth)
}

function generatePathFlutter(node: PathNode, depth: number): string {
  const pad = indent(depth)
  const fillColor = fillColorOnly(node.fill) ?? 'Colors.black'
  const w = typeof node.width === 'number' ? node.width : 24
  const h = typeof node.height === 'number' ? node.height : 24
  const widget = [
    `${pad}// ${node.name ?? 'Path'}`,
    `${pad}CustomPaint(`,
    `${pad}  size: Size(${w}, ${h}),`,
    `${pad}  painter: _PathPainter('${escapeDartString(node.d)}', ${fillColor}),`,
    `${pad})`,
  ].join('\n')
  return applyWrappers(widget, node, depth)
}

function generatePolygonFlutter(node: PolygonNode, depth: number): string {
  const pad = indent(depth)
  const fillColor = fillColorOnly(node.fill) ?? 'Colors.black'
  const w = typeof node.width === 'number' ? node.width : 24
  const h = typeof node.height === 'number' ? node.height : 24
  const widget = [
    `${pad}// Polygon (${node.polygonCount}-sided)`,
    `${pad}CustomPaint(`,
    `${pad}  size: Size(${w}, ${h}),`,
    `${pad}  painter: _PolygonPainter(${node.polygonCount}, ${fillColor}),`,
    `${pad})`,
  ].join('\n')
  return applyWrappers(widget, node, depth)
}

function generateImageFlutter(node: ImageNode, depth: number): string {
  const pad = indent(depth)
  const w = typeof node.width === 'number' ? node.width : undefined
  const h = typeof node.height === 'number' ? node.height : undefined
  const fit = node.objectFit === 'fit' ? 'BoxFit.contain' : 'BoxFit.cover'
  const src = node.src

  let ctor: string, firstArg: string
  if (src.startsWith('data:image/')) {
    const base64Data = src.replace(/^data:image\/[^;]+;base64,/, '')
    ctor = 'Image.memory'
    firstArg = `base64Decode('${escapeDartString(base64Data)}')`
  } else if (src.startsWith('http://') || src.startsWith('https://')) {
    ctor = 'Image.network'
    firstArg = `'${escapeDartString(src)}'`
  } else {
    ctor = 'Image.asset'
    firstArg = `'${escapeDartString(src)}'`
  }

  const parts: string[] = [`${pad}${ctor}(`]
  parts.push(`${pad}  ${firstArg},`)
  if (w !== undefined) parts.push(`${pad}  width: ${w},`)
  if (h !== undefined) parts.push(`${pad}  height: ${h},`)
  parts.push(`${pad}  fit: ${fit},`)
  parts.push(`${pad})`)
  let widget = parts.join('\n')

  if (node.cornerRadius) {
    const br = cornerRadiusToFlutter(node.cornerRadius)
    if (br) widget = `${pad}ClipRRect(\n${pad}  borderRadius: ${br},\n${pad}  child: ${widget.trimStart()},\n${pad})`
  }
  return applyWrappers(widget, node, depth)
}

function getHelperClasses(nodes: PenNode[]): string {
  let needsPath = false, needsPolygon = false
  function walk(list: PenNode[]) {
    for (const n of list) {
      if (n.type === 'path') needsPath = true
      if (n.type === 'polygon') needsPolygon = true
      if ('children' in n && (n as any).children) walk((n as any).children)
    }
  }
  walk(nodes)
  const helpers: string[] = []
  if (needsPath) {
    helpers.push(
`class _PathPainter extends CustomPainter {
  final String pathData;
  final Color color;
  _PathPainter(this.pathData, this.color);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = color;
    final path = parseSvgPathData(pathData);
    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}`)
  }
  if (needsPolygon) {
    helpers.push(
`class _PolygonPainter extends CustomPainter {
  final int sides;
  final Color color;
  _PolygonPainter(this.sides, this.color);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = color;
    final path = Path();
    final cx = size.width / 2, cy = size.height / 2, r = size.width / 2;
    for (var i = 0; i < sides; i++) {
      final angle = (i * 2 * pi / sides) - (pi / 2);
      final x = cx + r * cos(angle);
      final y = cy + r * sin(angle);
      i == 0 ? path.moveTo(x, y) : path.lineTo(x, y);
    }
    path.close();
    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}`)
  }
  return helpers.join('\n\n')
}

export function generateFlutterCode(
  nodes: PenNode[],
  widgetName = 'GeneratedDesign',
): string {
  if (nodes.length === 0) {
    return `import 'package:flutter/material.dart';\n\nclass ${widgetName} extends StatelessWidget {\n  const ${widgetName}({super.key});\n\n  @override\n  Widget build(BuildContext context) {\n    return const SizedBox.shrink();\n  }\n}\n`
  }

  let maxW = 0, maxH = 0
  for (const node of nodes) {
    const x = node.x ?? 0, y = node.y ?? 0
    const w = 'width' in node && typeof node.width === 'number' ? node.width : 0
    const h = 'height' in node && typeof node.height === 'number' ? node.height : 0
    maxW = Math.max(maxW, x + w)
    maxH = Math.max(maxH, y + h)
  }

  const childWidgets = nodes.map((n) => {
    const childStr = generateNodeFlutter(n, 5)
    const cx = n.x ?? 0, cy = n.y ?? 0
    if (cx !== 0 || cy !== 0) {
      const cPad = indent(4)
      return `${cPad}Positioned(\n${cPad}  left: ${cx},\n${cPad}  top: ${cy},\n${cPad}  child: ${childStr.trimStart()},\n${cPad})`
    }
    return childStr
  })

  const sizeArgs: string[] = []
  if (maxW > 0) sizeArgs.push(`\n          width: ${maxW},`)
  if (maxH > 0) sizeArgs.push(`\n          height: ${maxH},`)
  const sizedBoxParams = sizeArgs.length > 0 ? sizeArgs.join('') : '\n          width: double.infinity,\n          height: double.infinity,'

  const helpers = getHelperClasses(nodes)
  const helperSection = helpers ? `\n\n${helpers}` : ''

  return `import 'dart:convert';
import 'dart:math';
import 'package:flutter/material.dart';

class ${widgetName} extends StatelessWidget {
  const ${widgetName}({super.key});

  @override
  Widget build(BuildContext context) {
    return SizedBox(${sizedBoxParams}
      child: Stack(
        children: [
${childWidgets.map((c) => c + ',').join('\n')}
        ],
      ),
    );
  }
}${helperSection}
`
}

export function generateFlutterFromDocument(
  doc: PenDocument,
  activePageId?: string | null,
): string {
  const children = activePageId !== undefined
    ? getActivePageChildren(doc, activePageId)
    : doc.children
  return generateFlutterCode(children, 'GeneratedDesign')
}
