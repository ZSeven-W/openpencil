import type { PenDocument, PenNode, ContainerProps, TextNode } from '@/types/pen'
import type { PenFill, PenStroke, PenEffect, ShadowEffect } from '@/types/styles'
import { isVariableRef } from '@/variables/resolve-variables'
import { variableNameToCSS } from '@/services/codegen/css-variables-generator'

/**
 * Converts PenDocument nodes to React + Tailwind code.
 * $variable references are output as var(--name) CSS custom properties.
 */

/** Convert a `$variable` ref to `var(--name)`, or return the raw value. */
function varOrLiteral(value: string): string {
  if (isVariableRef(value)) {
    return `var(${variableNameToCSS(value.slice(1))})`
  }
  return value
}

function indent(depth: number): string {
  return '  '.repeat(depth)
}

function fillToTailwind(fills: PenFill[] | undefined): string[] {
  if (!fills || fills.length === 0) return []
  const fill = fills[0]
  if (fill.type === 'solid') {
    return [`bg-[${varOrLiteral(fill.color)}]`]
  }
  return []
}

function fillToTextColor(fills: PenFill[] | undefined): string[] {
  if (!fills || fills.length === 0) return []
  const fill = fills[0]
  if (fill.type === 'solid') {
    return [`text-[${varOrLiteral(fill.color)}]`]
  }
  return []
}

function strokeToTailwind(stroke: PenStroke | undefined): string[] {
  if (!stroke) return []
  const classes: string[] = []
  if (typeof stroke.thickness === 'string' && isVariableRef(stroke.thickness)) {
    classes.push('border', `border-[${varOrLiteral(stroke.thickness)}]`)
  } else {
    const thickness = typeof stroke.thickness === 'number'
      ? stroke.thickness
      : stroke.thickness[0]
    classes.push('border', `border-[${thickness}px]`)
  }
  if (stroke.fill && stroke.fill.length > 0) {
    const sf = stroke.fill[0]
    if (sf.type === 'solid') {
      classes.push(`border-[${varOrLiteral(sf.color)}]`)
    }
  }
  return classes
}

function effectsToTailwind(effects: PenEffect[] | undefined): string[] {
  if (!effects || effects.length === 0) return []
  const classes: string[] = []
  for (const effect of effects) {
    if (effect.type === 'shadow') {
      const s = effect as ShadowEffect
      classes.push(`shadow-[${s.offsetX}px_${s.offsetY}px_${s.blur}px_${s.spread}px_${s.color}]`)
    }
  }
  return classes
}

function cornerRadiusToTailwind(
  cr: number | [number, number, number, number] | undefined,
): string[] {
  if (cr === undefined) return []
  if (typeof cr === 'number') {
    if (cr === 0) return []
    return [`rounded-[${cr}px]`]
  }
  const [tl, tr, br, bl] = cr
  if (tl === tr && tr === br && br === bl) {
    return tl === 0 ? [] : [`rounded-[${tl}px]`]
  }
  return [`rounded-[${tl}px_${tr}px_${br}px_${bl}px]`]
}

function layoutToTailwind(node: ContainerProps): string[] {
  const classes: string[] = []
  if (node.layout === 'vertical') {
    classes.push('flex', 'flex-col')
  } else if (node.layout === 'horizontal') {
    classes.push('flex', 'flex-row')
  }
  if (node.gap !== undefined) {
    if (typeof node.gap === 'string' && isVariableRef(node.gap)) {
      classes.push(`gap-[${varOrLiteral(node.gap)}]`)
    } else if (typeof node.gap === 'number' && node.gap > 0) {
      classes.push(`gap-[${node.gap}px]`)
    }
  }
  if (node.padding !== undefined) {
    if (typeof node.padding === 'string' && isVariableRef(node.padding)) {
      classes.push(`p-[${varOrLiteral(node.padding)}]`)
    } else if (typeof node.padding === 'number') {
      classes.push(`p-[${node.padding}px]`)
    } else if (Array.isArray(node.padding)) {
      if (node.padding.length === 2) {
        classes.push(`py-[${node.padding[0]}px]`, `px-[${node.padding[1]}px]`)
      } else if (node.padding.length === 4) {
        classes.push(
          `pt-[${node.padding[0]}px]`,
          `pr-[${node.padding[1]}px]`,
          `pb-[${node.padding[2]}px]`,
          `pl-[${node.padding[3]}px]`,
        )
      }
    }
  }
  if (node.justifyContent) {
    const jcMap: Record<string, string> = {
      start: 'justify-start',
      center: 'justify-center',
      end: 'justify-end',
      space_between: 'justify-between',
      space_around: 'justify-around',
    }
    if (jcMap[node.justifyContent]) classes.push(jcMap[node.justifyContent])
  }
  if (node.alignItems) {
    const aiMap: Record<string, string> = {
      start: 'items-start',
      center: 'items-center',
      end: 'items-end',
    }
    if (aiMap[node.alignItems]) classes.push(aiMap[node.alignItems])
  }
  return classes
}

function sizeToTailwind(
  width: number | string | undefined,
  height: number | string | undefined,
): string[] {
  const classes: string[] = []
  if (typeof width === 'number') classes.push(`w-[${width}px]`)
  if (typeof height === 'number') classes.push(`h-[${height}px]`)
  return classes
}

function opacityToTailwind(opacity: number | string | undefined): string[] {
  if (opacity === undefined || opacity === 1) return []
  if (typeof opacity === 'string' && isVariableRef(opacity)) {
    return [`opacity-[${varOrLiteral(opacity)}]`]
  }
  if (typeof opacity === 'number') {
    const pct = Math.round(opacity * 100)
    return [`opacity-[${pct}%]`]
  }
  return []
}

function textTag(node: TextNode): string {
  const size = node.fontSize ?? 16
  if (size >= 32) return 'h1'
  if (size >= 24) return 'h2'
  if (size >= 20) return 'h3'
  return 'p'
}

function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content
  return node.content.map((s) => s.text).join('')
}

function textToTailwind(node: TextNode): string[] {
  const classes: string[] = []
  if (node.fontSize) classes.push(`text-[${node.fontSize}px]`)
  if (node.fontWeight) {
    const w = typeof node.fontWeight === 'number' ? node.fontWeight : parseInt(node.fontWeight, 10)
    if (!isNaN(w)) classes.push(`font-[${w}]`)
  }
  if (node.fontStyle === 'italic') classes.push('italic')
  if (node.textAlign) {
    const taMap: Record<string, string> = {
      left: 'text-left',
      center: 'text-center',
      right: 'text-right',
      justify: 'text-justify',
    }
    if (taMap[node.textAlign]) classes.push(taMap[node.textAlign])
  }
  if (node.fontFamily) classes.push(`font-['${node.fontFamily.replace(/\s/g, '_')}']`)
  if (node.lineHeight) classes.push(`leading-[${node.lineHeight}]`)
  if (node.letterSpacing) classes.push(`tracking-[${node.letterSpacing}px]`)
  if (node.underline) classes.push('underline')
  if (node.strikethrough) classes.push('line-through')
  return classes
}

function generateNodeJSX(node: PenNode, depth: number): string {
  const pad = indent(depth)
  const classes: string[] = []

  // Position
  if (node.x !== undefined || node.y !== undefined) {
    classes.push('absolute')
    if (node.x !== undefined) classes.push(`left-[${node.x}px]`)
    if (node.y !== undefined) classes.push(`top-[${node.y}px]`)
  }

  // Opacity
  classes.push(...opacityToTailwind(node.opacity))

  // Rotation
  if (node.rotation) {
    classes.push(`rotate-[${node.rotation}deg]`)
  }

  switch (node.type) {
    case 'frame':
    case 'rectangle':
    case 'group': {
      classes.push(
        ...sizeToTailwind(node.width, node.height),
        ...fillToTailwind(node.fill),
        ...strokeToTailwind(node.stroke),
        ...cornerRadiusToTailwind(node.cornerRadius),
        ...effectsToTailwind(node.effects),
        ...layoutToTailwind(node),
      )
      const childNodes = node.children ?? []
      if (childNodes.length === 0) {
        return `${pad}<div className="${classes.join(' ')}" />`
      }
      const childrenJSX = childNodes
        .map((c) => generateNodeJSX(c, depth + 1))
        .join('\n')
      const comment = node.name ? `${pad}{/* ${node.name} */}\n` : ''
      return `${comment}${pad}<div className="${classes.join(' ')}">\n${childrenJSX}\n${pad}</div>`
    }

    case 'ellipse': {
      classes.push(
        'rounded-full',
        ...sizeToTailwind(node.width, node.height),
        ...fillToTailwind(node.fill),
        ...strokeToTailwind(node.stroke),
        ...effectsToTailwind(node.effects),
      )
      return `${pad}<div className="${classes.join(' ')}" />`
    }

    case 'text': {
      const tag = textTag(node)
      classes.push(
        ...sizeToTailwind(node.width, node.height),
        ...fillToTextColor(node.fill),
        ...textToTailwind(node),
        ...effectsToTailwind(node.effects),
      )
      const text = escapeJSX(getTextContent(node))
      return `${pad}<${tag} className="${classes.join(' ')}">${text}</${tag}>`
    }

    case 'line': {
      const w = node.x2 !== undefined ? Math.abs(node.x2 - (node.x ?? 0)) : 0
      classes.push(`w-[${w}px]`)
      if (node.stroke) {
        const thickness = typeof node.stroke.thickness === 'number'
          ? node.stroke.thickness
          : typeof node.stroke.thickness === 'string' ? node.stroke.thickness : node.stroke.thickness[0]
        if (typeof thickness === 'string' && isVariableRef(thickness)) {
          classes.push(`border-t-[${varOrLiteral(thickness)}]`)
        } else {
          classes.push(`border-t-[${thickness}px]`)
        }
        if (node.stroke.fill && node.stroke.fill.length > 0) {
          const sf = node.stroke.fill[0]
          if (sf.type === 'solid') {
            classes.push(`border-[${varOrLiteral(sf.color)}]`)
          }
        }
      }
      return `${pad}<hr className="${classes.join(' ')}" />`
    }

    case 'polygon':
    case 'path': {
      // For complex shapes, output an SVG inline
      classes.push(...sizeToTailwind(node.width, node.height))
      if (node.type === 'path') {
        const w = typeof node.width === 'number' ? node.width : 100
        const h = typeof node.height === 'number' ? node.height : 100
        const fillColor = node.fill?.[0]?.type === 'solid' ? varOrLiteral(node.fill[0].color) : 'currentColor'
        return `${pad}<svg className="${classes.join(' ')}" viewBox="0 0 ${w} ${h}">\n${pad}  <path d="${node.d}" fill="${fillColor}" />\n${pad}</svg>`
      }
      classes.push(...fillToTailwind(node.fill))
      return `${pad}<div className="${classes.join(' ')}" />`
    }

    case 'ref':
      return `${pad}{/* Ref: ${node.ref} */}`

    default:
      return `${pad}{/* Unknown node */}`
  }
}

function escapeJSX(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/{/g, '&#123;')
    .replace(/}/g, '&#125;')
}

export function generateReactCode(
  nodes: PenNode[],
  componentName = 'GeneratedDesign',
): string {
  if (nodes.length === 0) {
    return `export function ${componentName}() {\n  return <div className="relative" />\n}\n`
  }

  // Find bounding box for the root wrapper
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

  const wrapperClasses = ['relative']
  if (maxW > 0) wrapperClasses.push(`w-[${maxW}px]`)
  if (maxH > 0) wrapperClasses.push(`h-[${maxH}px]`)

  const childrenJSX = nodes
    .map((n) => generateNodeJSX(n, 2))
    .join('\n')

  return `export function ${componentName}() {
  return (
    <div className="${wrapperClasses.join(' ')}">
${childrenJSX}
    </div>
  )
}
`
}

export function generateReactFromDocument(doc: PenDocument): string {
  return generateReactCode(doc.children, 'GeneratedDesign')
}
