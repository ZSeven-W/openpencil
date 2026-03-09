import type { PenDocument, PenNode, ContainerProps, TextNode } from '@/types/pen'
import { getActivePageChildren } from '@/stores/document-tree-utils'
import type { PenFill, PenStroke, PenEffect, ShadowEffect } from '@/types/styles'
import { isVariableRef } from '@/variables/resolve-variables'
import { variableNameToCSS } from '@/services/codegen/css-variables-generator'

/**
 * Converts PenDocument nodes to Svelte component code.
 * Generates a single .svelte file with markup + scoped <style>.
 */

function varOrLiteral(value: string): string {
  if (isVariableRef(value)) return `var(${variableNameToCSS(value.slice(1))})`
  return value
}

function indent(depth: number): string {
  return '  '.repeat(depth)
}

// ---------------------------------------------------------------------------
// CSS helpers
// ---------------------------------------------------------------------------

function fillToCSS(fills: PenFill[] | undefined): Record<string, string> {
  if (!fills || fills.length === 0) return {}
  const fill = fills[0]
  if (fill.type === 'solid') return { background: varOrLiteral(fill.color) }
  if (fill.type === 'linear_gradient') {
    if (!fill.stops?.length) return {}
    const angle = fill.angle ?? 180
    const stops = fill.stops.map((s) => `${varOrLiteral(s.color)} ${Math.round(s.offset * 100)}%`).join(', ')
    return { background: `linear-gradient(${angle}deg, ${stops})` }
  }
  if (fill.type === 'radial_gradient') {
    if (!fill.stops?.length) return {}
    const stops = fill.stops.map((s) => `${varOrLiteral(s.color)} ${Math.round(s.offset * 100)}%`).join(', ')
    return { background: `radial-gradient(circle, ${stops})` }
  }
  return {}
}

function strokeToCSS(stroke: PenStroke | undefined): Record<string, string> {
  if (!stroke) return {}
  const css: Record<string, string> = {}
  if (typeof stroke.thickness === 'string' && isVariableRef(stroke.thickness)) {
    css['border-width'] = varOrLiteral(stroke.thickness)
  } else {
    const t = typeof stroke.thickness === 'number' ? stroke.thickness : stroke.thickness[0]
    css['border-width'] = `${t}px`
  }
  css['border-style'] = 'solid'
  if (stroke.fill?.[0]?.type === 'solid') css['border-color'] = varOrLiteral(stroke.fill[0].color)
  return css
}

function effectsToCSS(effects: PenEffect[] | undefined): Record<string, string> {
  if (!effects || effects.length === 0) return {}
  const shadows: string[] = []
  for (const effect of effects) {
    if (effect.type === 'shadow') {
      const s = effect as ShadowEffect
      shadows.push(`${s.inner ? 'inset ' : ''}${s.offsetX}px ${s.offsetY}px ${s.blur}px ${s.spread}px ${s.color}`)
    }
  }
  return shadows.length > 0 ? { 'box-shadow': shadows.join(', ') } : {}
}

function cornerRadiusToCSS(cr: number | [number, number, number, number] | undefined): Record<string, string> {
  if (cr === undefined) return {}
  if (typeof cr === 'number') return cr === 0 ? {} : { 'border-radius': `${cr}px` }
  return { 'border-radius': `${cr[0]}px ${cr[1]}px ${cr[2]}px ${cr[3]}px` }
}

function layoutToCSS(node: ContainerProps): Record<string, string> {
  const css: Record<string, string> = {}
  if (node.layout === 'vertical') { css.display = 'flex'; css['flex-direction'] = 'column' }
  else if (node.layout === 'horizontal') { css.display = 'flex'; css['flex-direction'] = 'row' }
  if (node.gap !== undefined) {
    css.gap = typeof node.gap === 'string' && isVariableRef(node.gap)
      ? varOrLiteral(node.gap)
      : typeof node.gap === 'number' ? `${node.gap}px` : ''
  }
  if (node.padding !== undefined) {
    if (typeof node.padding === 'string' && isVariableRef(node.padding)) css.padding = varOrLiteral(node.padding)
    else if (typeof node.padding === 'number') css.padding = `${node.padding}px`
    else if (Array.isArray(node.padding)) css.padding = node.padding.map((p) => `${p}px`).join(' ')
  }
  if (node.justifyContent) {
    const map: Record<string, string> = { start: 'flex-start', center: 'center', end: 'flex-end', space_between: 'space-between', space_around: 'space-around' }
    css['justify-content'] = map[node.justifyContent] ?? node.justifyContent
  }
  if (node.alignItems) {
    const map: Record<string, string> = { start: 'flex-start', center: 'center', end: 'flex-end' }
    css['align-items'] = map[node.alignItems] ?? node.alignItems
  }
  if (node.clipContent) css.overflow = 'hidden'
  return css
}

// ---------------------------------------------------------------------------
// Node → markup + CSS rules
// ---------------------------------------------------------------------------

interface CSSRule { className: string; properties: Record<string, string> }

let classCounter = 0
function resetClassCounter() { classCounter = 0 }
function nextClassName(prefix: string): string { return `${prefix}-${++classCounter}` }

function escapeHTML(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content
  return node.content.map((s) => s.text).join('')
}

function generateNodeMarkup(node: PenNode, depth: number, rules: CSSRule[]): string {
  const pad = indent(depth)
  const css: Record<string, string> = {}

  if (node.x !== undefined || node.y !== undefined) {
    css.position = 'absolute'
    if (node.x !== undefined) css.left = `${node.x}px`
    if (node.y !== undefined) css.top = `${node.y}px`
  }
  if (node.opacity !== undefined && node.opacity !== 1) {
    if (typeof node.opacity === 'string' && isVariableRef(node.opacity)) css.opacity = varOrLiteral(node.opacity)
    else if (typeof node.opacity === 'number') css.opacity = String(node.opacity)
  }
  if (node.rotation) css.transform = `rotate(${node.rotation}deg)`

  switch (node.type) {
    case 'frame':
    case 'rectangle':
    case 'group': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      Object.assign(css, fillToCSS(node.fill), strokeToCSS(node.stroke), cornerRadiusToCSS(node.cornerRadius), effectsToCSS(node.effects), layoutToCSS(node))
      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? node.type)
      rules.push({ className, properties: css })
      const children = node.children ?? []
      if (children.length === 0) return `${pad}<div class="${className}" />`
      const childrenMarkup = children.map((c) => generateNodeMarkup(c, depth + 1, rules)).join('\n')
      return `${pad}<div class="${className}">\n${childrenMarkup}\n${pad}</div>`
    }

    case 'ellipse': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      css['border-radius'] = '50%'
      Object.assign(css, fillToCSS(node.fill), strokeToCSS(node.stroke), effectsToCSS(node.effects))
      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? 'ellipse')
      rules.push({ className, properties: css })
      return `${pad}<div class="${className}" />`
    }

    case 'text': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      if (node.fill) { const f = node.fill[0]; if (f?.type === 'solid') css.color = varOrLiteral(f.color) }
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
      return `${pad}<${tag} class="${className}">${escapeHTML(getTextContent(node))}</${tag}>`
    }

    case 'line': {
      const w = node.x2 !== undefined ? Math.abs(node.x2 - (node.x ?? 0)) : 0
      css.width = `${w}px`
      if (node.stroke) {
        const t = typeof node.stroke.thickness === 'number' ? node.stroke.thickness : node.stroke.thickness[0]
        css['border-top-width'] = `${t}px`
        css['border-top-style'] = 'solid'
        if (node.stroke.fill?.[0]?.type === 'solid') css['border-top-color'] = varOrLiteral(node.stroke.fill[0].color)
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
        const fillColor = node.fill?.[0]?.type === 'solid' ? varOrLiteral(node.fill[0].color) : 'currentColor'
        return `${pad}<svg class="${className}" viewBox="0 0 ${w} ${h}">\n${pad}  <path d="${node.d}" fill="${fillColor}" />\n${pad}</svg>`
      }
      return `${pad}<div class="${className}" />`
    }

    case 'image': {
      if (typeof node.width === 'number') css.width = `${node.width}px`
      if (typeof node.height === 'number') css.height = `${node.height}px`
      css['object-fit'] = node.objectFit ?? 'fill'
      Object.assign(css, cornerRadiusToCSS(node.cornerRadius))
      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? 'image')
      rules.push({ className, properties: css })
      return `${pad}<img class="${className}" src="${escapeHTML(node.src)}" alt="${escapeHTML(node.name ?? '')}" />`
    }

    case 'icon_font': {
      const size = typeof node.width === 'number' ? node.width : 24
      css.width = `${size}px`
      css.height = `${size}px`
      if (node.fill?.[0]?.type === 'solid') css.color = varOrLiteral(node.fill[0].color)
      const className = nextClassName(node.name?.replace(/\s+/g, '-').toLowerCase() ?? 'icon')
      rules.push({ className, properties: css })
      return `${pad}<i class="${className}" data-lucide="${escapeHTML(node.iconFontName ?? 'circle')}" />`
    }

    case 'ref':
      return `${pad}<!-- Ref: ${node.ref} -->`

    default:
      return `${pad}<!-- Unknown node -->`
  }
}

function cssRulesToString(rules: CSSRule[]): string {
  return rules.map((r) => {
    const props = Object.entries(r.properties).map(([k, v]) => `  ${k}: ${v};`).join('\n')
    return `.${r.className} {\n${props}\n}`
  }).join('\n\n')
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function generateSvelteCode(nodes: PenNode[]): string {
  resetClassCounter()
  const rules: CSSRule[] = []

  let maxW = 0, maxH = 0
  for (const node of nodes) {
    const x = node.x ?? 0, y = node.y ?? 0
    const w = 'width' in node && typeof node.width === 'number' ? node.width : 0
    const h = 'height' in node && typeof node.height === 'number' ? node.height : 0
    maxW = Math.max(maxW, x + w)
    maxH = Math.max(maxH, y + h)
  }

  const containerCSS: Record<string, string> = { position: 'relative' }
  if (maxW > 0) containerCSS.width = `${maxW}px`
  if (maxH > 0) containerCSS.height = `${maxH}px`
  rules.push({ className: 'container', properties: containerCSS })

  const markup = nodes.length === 0
    ? '<div class="container" />'
    : `<div class="container">\n${nodes.map((n) => generateNodeMarkup(n, 1, rules)).join('\n')}\n</div>`

  const css = cssRulesToString(rules)

  return `${markup}

<style>
${css}
</style>
`
}

export function generateSvelteFromDocument(doc: PenDocument, activePageId?: string | null): string {
  const children = activePageId !== undefined ? getActivePageChildren(doc, activePageId) : doc.children
  return generateSvelteCode(children)
}
