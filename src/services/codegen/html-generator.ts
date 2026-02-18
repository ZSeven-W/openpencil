import type { PenDocument, PenNode, ContainerProps, TextNode } from '@/types/pen'
import type { PenFill, PenStroke, PenEffect, ShadowEffect } from '@/types/styles'

/**
 * Converts PenDocument nodes to HTML + CSS.
 */

let classCounter = 0

function resetClassCounter() {
  classCounter = 0
}

function nextClassName(prefix: string): string {
  classCounter++
  return `${prefix}-${classCounter}`
}

function indent(depth: number): string {
  return '  '.repeat(depth)
}

function fillToCSS(fills: PenFill[] | undefined): Record<string, string> {
  if (!fills || fills.length === 0) return {}
  const fill = fills[0]
  if (fill.type === 'solid') {
    return { background: fill.color }
  }
  if (fill.type === 'linear_gradient') {
    const angle = fill.angle ?? 180
    const stops = fill.stops.map((s) => `${s.color} ${Math.round(s.offset * 100)}%`).join(', ')
    return { background: `linear-gradient(${angle}deg, ${stops})` }
  }
  if (fill.type === 'radial_gradient') {
    const stops = fill.stops.map((s) => `${s.color} ${Math.round(s.offset * 100)}%`).join(', ')
    return { background: `radial-gradient(circle, ${stops})` }
  }
  return {}
}

function strokeToCSS(stroke: PenStroke | undefined): Record<string, string> {
  if (!stroke) return {}
  const css: Record<string, string> = {}
  const thickness = typeof stroke.thickness === 'number'
    ? stroke.thickness
    : stroke.thickness[0]
  css['border-width'] = `${thickness}px`
  css['border-style'] = 'solid'
  if (stroke.fill && stroke.fill.length > 0) {
    const sf = stroke.fill[0]
    if (sf.type === 'solid') {
      css['border-color'] = sf.color
    }
  }
  return css
}

function effectsToCSS(effects: PenEffect[] | undefined): Record<string, string> {
  if (!effects || effects.length === 0) return {}
  const shadows: string[] = []
  for (const effect of effects) {
    if (effect.type === 'shadow') {
      const s = effect as ShadowEffect
      const inset = s.inner ? 'inset ' : ''
      shadows.push(`${inset}${s.offsetX}px ${s.offsetY}px ${s.blur}px ${s.spread}px ${s.color}`)
    }
  }
  if (shadows.length > 0) {
    return { 'box-shadow': shadows.join(', ') }
  }
  return {}
}

function cornerRadiusToCSS(
  cr: number | [number, number, number, number] | undefined,
): Record<string, string> {
  if (cr === undefined) return {}
  if (typeof cr === 'number') {
    return cr === 0 ? {} : { 'border-radius': `${cr}px` }
  }
  return { 'border-radius': `${cr[0]}px ${cr[1]}px ${cr[2]}px ${cr[3]}px` }
}

function layoutToCSS(node: ContainerProps): Record<string, string> {
  const css: Record<string, string> = {}
  if (node.layout === 'vertical') {
    css.display = 'flex'
    css['flex-direction'] = 'column'
  } else if (node.layout === 'horizontal') {
    css.display = 'flex'
    css['flex-direction'] = 'row'
  }
  if (node.gap !== undefined && typeof node.gap === 'number') {
    css.gap = `${node.gap}px`
  }
  if (node.padding !== undefined) {
    if (typeof node.padding === 'number') {
      css.padding = `${node.padding}px`
    } else if (Array.isArray(node.padding)) {
      css.padding = node.padding.map((p) => `${p}px`).join(' ')
    }
  }
  if (node.justifyContent) {
    const map: Record<string, string> = {
      start: 'flex-start',
      center: 'center',
      end: 'flex-end',
      space_between: 'space-between',
      space_around: 'space-around',
    }
    css['justify-content'] = map[node.justifyContent] ?? node.justifyContent
  }
  if (node.alignItems) {
    const map: Record<string, string> = {
      start: 'flex-start',
      center: 'center',
      end: 'flex-end',
    }
    css['align-items'] = map[node.alignItems] ?? node.alignItems
  }
  return css
}

interface CSSRule {
  className: string
  properties: Record<string, string>
}

function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content
  return node.content.map((s) => s.text).join('')
}

function escapeHTML(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function generateNodeHTML(
  node: PenNode,
  depth: number,
  rules: CSSRule[],
): string {
  const pad = indent(depth)
  const css: Record<string, string> = {}

  // Position
  if (node.x !== undefined || node.y !== undefined) {
    css.position = 'absolute'
    if (node.x !== undefined) css.left = `${node.x}px`
    if (node.y !== undefined) css.top = `${node.y}px`
  }

  // Opacity
  if (node.opacity !== undefined && node.opacity !== 1 && typeof node.opacity === 'number') {
    css.opacity = String(node.opacity)
  }

  // Rotation
  if (node.rotation) {
    css.transform = `rotate(${node.rotation}deg)`
  }

  switch (node.type) {
    case 'frame':
    case 'rectangle':
    case 'group': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      Object.assign(css, fillToCSS(node.fill))
      Object.assign(css, strokeToCSS(node.stroke))
      Object.assign(css, cornerRadiusToCSS(node.cornerRadius))
      Object.assign(css, effectsToCSS(node.effects))
      Object.assign(css, layoutToCSS(node))

      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? node.type)
      rules.push({ className, properties: css })

      const children = node.children ?? []
      if (children.length === 0) {
        return `${pad}<div class="${className}"></div>`
      }
      const childrenHTML = children
        .map((c) => generateNodeHTML(c, depth + 1, rules))
        .join('\n')
      return `${pad}<div class="${className}">\n${childrenHTML}\n${pad}</div>`
    }

    case 'ellipse': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      css['border-radius'] = '50%'
      Object.assign(css, fillToCSS(node.fill))
      Object.assign(css, strokeToCSS(node.stroke))
      Object.assign(css, effectsToCSS(node.effects))

      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? 'ellipse')
      rules.push({ className, properties: css })
      return `${pad}<div class="${className}"></div>`
    }

    case 'text': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      if (node.fill) {
        const fill = node.fill[0]
        if (fill?.type === 'solid') css.color = fill.color
      }
      if (node.fontSize) css['font-size'] = `${node.fontSize}px`
      if (node.fontWeight) css['font-weight'] = String(node.fontWeight)
      if (node.fontStyle === 'italic') css['font-style'] = 'italic'
      if (node.textAlign) css['text-align'] = node.textAlign
      if (node.fontFamily) css['font-family'] = `'${node.fontFamily}', sans-serif`
      if (node.lineHeight) css['line-height'] = String(node.lineHeight)
      if (node.letterSpacing) css['letter-spacing'] = `${node.letterSpacing}px`
      if (node.underline) css['text-decoration'] = 'underline'
      if (node.strikethrough) css['text-decoration'] = 'line-through'
      Object.assign(css, effectsToCSS(node.effects))

      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? 'text')
      rules.push({ className, properties: css })

      const size = node.fontSize ?? 16
      const tag = size >= 32 ? 'h1' : size >= 24 ? 'h2' : size >= 20 ? 'h3' : 'p'
      const text = escapeHTML(getTextContent(node))
      return `${pad}<${tag} class="${className}">${text}</${tag}>`
    }

    case 'line': {
      const w = node.x2 !== undefined ? Math.abs(node.x2 - (node.x ?? 0)) : 0
      css.width = `${w}px`
      if (node.stroke) {
        const thickness = typeof node.stroke.thickness === 'number'
          ? node.stroke.thickness
          : node.stroke.thickness[0]
        css['border-top-width'] = `${thickness}px`
        css['border-top-style'] = 'solid'
        if (node.stroke.fill && node.stroke.fill.length > 0) {
          const sf = node.stroke.fill[0]
          if (sf.type === 'solid') css['border-top-color'] = sf.color
        }
      }
      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? 'line')
      rules.push({ className, properties: css })
      return `${pad}<hr class="${className}" />`
    }

    case 'polygon':
    case 'path': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      Object.assign(css, fillToCSS(node.fill))
      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? node.type)
      rules.push({ className, properties: css })
      if (node.type === 'path') {
        const w = typeof node.width === 'number' ? node.width : 100
        const h = typeof node.height === 'number' ? node.height : 100
        const fillColor = node.fill?.[0]?.type === 'solid' ? node.fill[0].color : 'currentColor'
        return `${pad}<svg class="${className}" viewBox="0 0 ${w} ${h}">\n${pad}  <path d="${node.d}" fill="${fillColor}" />\n${pad}</svg>`
      }
      return `${pad}<div class="${className}"></div>`
    }

    case 'ref':
      return `${pad}<!-- Ref: ${node.ref} -->`

    default:
      return `${pad}<!-- Unknown node -->`
  }
}

function cssRulesToString(rules: CSSRule[]): string {
  return rules
    .map((r) => {
      const props = Object.entries(r.properties)
        .map(([k, v]) => `  ${k}: ${v};`)
        .join('\n')
      return `.${r.className} {\n${props}\n}`
    })
    .join('\n\n')
}

export function generateHTMLCode(nodes: PenNode[]): { html: string; css: string } {
  resetClassCounter()
  const rules: CSSRule[] = []

  if (nodes.length === 0) {
    return {
      html: '<div class="container"></div>',
      css: '.container {\n  position: relative;\n}',
    }
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

  const containerCSS: Record<string, string> = { position: 'relative' }
  if (maxW > 0) containerCSS.width = `${maxW}px`
  if (maxH > 0) containerCSS.height = `${maxH}px`
  rules.push({ className: 'container', properties: containerCSS })

  const childrenHTML = nodes
    .map((n) => generateNodeHTML(n, 1, rules))
    .join('\n')

  const html = `<div class="container">\n${childrenHTML}\n</div>`
  const css = cssRulesToString(rules)

  return { html, css }
}

export function generateHTMLFromDocument(doc: PenDocument): { html: string; css: string } {
  return generateHTMLCode(doc.children)
}
