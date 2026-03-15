import type { PenNode, PenNodeBase, LineNode } from '@/types/pen'
import type { PenFill, PenStroke } from '@/types/styles'
import { generateId } from '@/stores/document-store'

/** Inherited style context passed from parent SVG/g elements */
interface StyleCtx {
  fill: string | null
  stroke: string | null
  strokeWidth: number
}

/**
 * Parse an SVG string into editable PenNode array.
 * Handles fill/stroke inheritance, path coordinate scaling, and viewBox transforms.
 */
export function parseSvgToNodes(
  svgText: string,
  maxDim = 400,
): PenNode[] {
  const parser = new DOMParser()
  const doc = parser.parseFromString(svgText, 'image/svg+xml')
  const svg = doc.querySelector('svg')
  if (!svg) return []

  // Resolve source dimensions from viewBox and width/height
  const vb = svg.getAttribute('viewBox')?.split(/[\s,]+/).map(Number)
  const vbW = vb?.[2] || 100
  const vbH = vb?.[3] || 100
  // Presentation size — only use if pure numeric (ignore "1em", "100%", etc.)
  const rawW = svg.getAttribute('width') ?? ''
  const rawH = svg.getAttribute('height') ?? ''
  const svgW = /^[\d.]+$/.test(rawW) ? parseFloat(rawW) : vbW
  const svgH = /^[\d.]+$/.test(rawH) ? parseFloat(rawH) : vbH

  // Target output size clamped to maxDim
  let outW = svgW, outH = svgH
  if (outW > maxDim || outH > maxDim) {
    const s = maxDim / Math.max(outW, outH)
    outW *= s
    outH *= s
  }

  // Scale from viewBox coordinate space to output space
  const scale = Math.min(outW / vbW, outH / vbH)

  // Build inherited style context from <svg> attributes
  const rootCtx: StyleCtx = {
    fill: svg.getAttribute('fill'),
    stroke: svg.getAttribute('stroke'),
    strokeWidth: parseFloat(svg.getAttribute('stroke-width') ?? '') || 1,
  }

  const nodes = parseChildren(svg, scale, rootCtx)

  if (nodes.length === 0) return []
  if (nodes.length === 1) return nodes

  // Wrap multiple elements in a frame
  return [
    {
      id: generateId(),
      type: 'frame',
      name: 'SVG',
      width: Math.ceil(outW),
      height: Math.ceil(outH),
      layout: 'none' as const,
      children: nodes,
    },
  ]
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

const SKIP_TAGS = new Set([
  'defs', 'style', 'title', 'desc', 'metadata',
  'clippath', 'mask', 'filter', 'lineargradient', 'radialgradient',
  'symbol', 'marker', 'pattern',
  'script', 'foreignobject', 'animate', 'animatemotion', 'set',
])

function parseChildren(
  parent: Element,
  scale: number,
  ctx: StyleCtx,
): PenNode[] {
  const nodes: PenNode[] = []
  for (const el of parent.children) {
    const node = parseElement(el, scale, ctx)
    if (node) nodes.push(node)
  }
  return nodes
}

function parseElement(
  el: Element,
  scale: number,
  parentCtx: StyleCtx,
): PenNode | null {
  const tag = el.tagName.toLowerCase()
  if (SKIP_TAGS.has(tag)) return null

  // Merge this element's style context with parent
  const ctx = mergeStyleCtx(parentCtx, el)

  // Handle <g> by recursing with inherited styles
  if (tag === 'g') {
    const children = parseChildren(el, scale, ctx)
    if (children.length === 0) return null
    if (children.length === 1) return children[0]
    const bounds = computeChildrenBounds(children)
    // Make children coordinates relative to the group origin
    for (const child of children) {
      offsetChild(child, -bounds.x, -bounds.y)
    }
    return {
      id: generateId(),
      type: 'frame',
      name: el.getAttribute('id') ?? 'Group',
      x: bounds.x,
      y: bounds.y,
      width: Math.ceil(bounds.w),
      height: Math.ceil(bounds.h),
      layout: 'none' as const,
      children,
    }
  }

  // Resolve fill & stroke from element + inherited context
  const fill = resolveFill(el, ctx)
  const stroke = resolveStroke(el, ctx, scale)
  const opacity = parseFloat(getAttr(el, 'opacity') ?? '1')

  const base = {
    id: generateId(),
    opacity: opacity !== 1 ? opacity : undefined,
  }

  switch (tag) {
    case 'path': {
      const d = el.getAttribute('d')
      if (!d) return null
      const scaledD = scaleSvgPath(d, scale)
      const bbox = getPathBBox(scaledD)
      return {
        ...base,
        type: 'path' as const,
        name: el.getAttribute('id') ?? 'Path',
        d: scaledD,
        x: bbox.x,
        y: bbox.y,
        width: Math.ceil(bbox.w),
        height: Math.ceil(bbox.h),
        fill,
        stroke,
      }
    }

    case 'rect': {
      const x = num(el, 'x') * scale
      const y = num(el, 'y') * scale
      const w = num(el, 'width') * scale
      const h = num(el, 'height') * scale
      const rx = num(el, 'rx') * scale
      return {
        ...base,
        type: 'rectangle' as const,
        name: el.getAttribute('id') ?? 'Rectangle',
        x, y,
        width: Math.ceil(w),
        height: Math.ceil(h),
        cornerRadius: rx || undefined,
        fill,
        stroke,
      }
    }

    case 'circle': {
      const cx = num(el, 'cx') * scale
      const cy = num(el, 'cy') * scale
      const r = num(el, 'r') * scale
      return {
        ...base,
        type: 'ellipse' as const,
        name: el.getAttribute('id') ?? 'Circle',
        x: cx - r, y: cy - r,
        width: Math.ceil(r * 2),
        height: Math.ceil(r * 2),
        fill,
        stroke,
      }
    }

    case 'ellipse': {
      const cx = num(el, 'cx') * scale
      const cy = num(el, 'cy') * scale
      const rx = num(el, 'rx') * scale
      const ry = num(el, 'ry') * scale
      return {
        ...base,
        type: 'ellipse' as const,
        name: el.getAttribute('id') ?? 'Ellipse',
        x: cx - rx, y: cy - ry,
        width: Math.ceil(rx * 2),
        height: Math.ceil(ry * 2),
        fill,
        stroke,
      }
    }

    case 'line': {
      return {
        ...base,
        type: 'line' as const,
        name: el.getAttribute('id') ?? 'Line',
        x: num(el, 'x1') * scale,
        y: num(el, 'y1') * scale,
        x2: num(el, 'x2') * scale,
        y2: num(el, 'y2') * scale,
        stroke: stroke ?? { thickness: 1, fill: [{ type: 'solid', color: '#000000' }] },
      }
    }

    case 'polygon':
    case 'polyline': {
      const pts = el.getAttribute('points')
      if (!pts) return null
      const scaledD = scaleSvgPath(pointsToD(pts, tag === 'polygon'), scale)
      const bbox = getPathBBox(scaledD)
      return {
        ...base,
        type: 'path' as const,
        name: el.getAttribute('id') ?? (tag === 'polygon' ? 'Polygon' : 'Polyline'),
        d: scaledD,
        x: bbox.x,
        y: bbox.y,
        width: Math.ceil(bbox.w),
        height: Math.ceil(bbox.h),
        fill: tag === 'polygon' ? fill : noFill(),
        stroke,
      }
    }

    default:
      return null
  }
}

// ---------------------------------------------------------------------------
// Style resolution with inheritance
// ---------------------------------------------------------------------------

/** Read an attribute from the element, checking inline `style` first */
function getAttr(el: Element, name: string): string | null {
  // Check inline style first (higher priority)
  const style = el.getAttribute('style')
  if (style) {
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const m = style.match(new RegExp(`${escaped}\\s*:\\s*([^;]+)`))
    if (m) return m[1].trim()
  }
  return el.getAttribute(name)
}

/** Build style context by merging parent context with this element's attributes */
function mergeStyleCtx(parent: StyleCtx, el: Element): StyleCtx {
  return {
    fill: getAttr(el, 'fill') ?? parent.fill,
    stroke: getAttr(el, 'stroke') ?? parent.stroke,
    strokeWidth: parseFloat(getAttr(el, 'stroke-width') ?? '') || parent.strokeWidth,
  }
}

/** Normalize a color string: resolve currentColor, inherit → black */
function normalizeColor(raw: string): string {
  if (raw === 'currentColor' || raw === 'inherit') return '#000000'
  return raw
}

/** Explicit "no fill" — transparent so canvas DEFAULT_FILL won't apply */
function noFill(): PenFill[] {
  return [{ type: 'solid', color: 'transparent' }]
}

/** Resolve fill from element attribute + inherited context → PenFill[] */
function resolveFill(el: Element, ctx: StyleCtx): PenFill[] | undefined {
  const raw = getAttr(el, 'fill') ?? ctx.fill

  // Explicit "none" → transparent fill
  if (raw === 'none') return noFill()
  // url(#gradient) references — not supported, use black fallback
  if (raw && raw.startsWith('url(')) return [{ type: 'solid', color: '#000000' }]
  // Has a color
  if (raw) return [{ type: 'solid', color: normalizeColor(raw) }]
  // SVG spec default: black fill
  return [{ type: 'solid', color: '#000000' }]
}

/** Resolve stroke from element attribute + inherited context → PenStroke */
function resolveStroke(el: Element, ctx: StyleCtx, scale: number): PenStroke | undefined {
  const raw = getAttr(el, 'stroke') ?? ctx.stroke
  if (!raw || raw === 'none') return undefined
  if (raw.startsWith('url(')) return undefined

  const width = (parseFloat(getAttr(el, 'stroke-width') ?? '') || ctx.strokeWidth) * scale
  return {
    thickness: width,
    fill: [{ type: 'solid', color: normalizeColor(raw) }],
  }
}

// ---------------------------------------------------------------------------
// SVG path data scaling
// ---------------------------------------------------------------------------

/**
 * Scale all coordinates in an SVG path `d` string by a factor.
 * Handles M/L/C/S/Q/T/H/V/A commands (both absolute and relative).
 * For arc (A) commands, only rx/ry/x/y are scaled — flags and rotation are preserved.
 */
function scaleSvgPath(d: string, scale: number): string {
  if (scale === 1) return d

  // Tokenize into commands and numbers
  const tokens = d.match(/[a-zA-Z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g)
  if (!tokens) return d

  let result = ''
  let cmd = ''
  let paramIdx = 0

  for (const tok of tokens) {
    if (/[a-zA-Z]/.test(tok)) {
      cmd = tok
      paramIdx = 0
      result += tok
      continue
    }

    const n = parseFloat(tok)
    const upper = cmd.toUpperCase()

    if (upper === 'A') {
      // Arc: rx ry x-rotation large-arc-flag sweep-flag x y (7 params per arc)
      const pos = paramIdx % 7
      // Scale rx(0), ry(1), x(5), y(6); keep rotation(2), flags(3,4) unchanged
      const shouldScale = pos === 0 || pos === 1 || pos === 5 || pos === 6
      result += ' ' + (shouldScale ? n * scale : n)
    } else {
      // All other commands: every param is a coordinate → scale it
      result += ' ' + n * scale
    }
    paramIdx++
  }

  return result.trim()
}

// ---------------------------------------------------------------------------
// Path bounding box (uses browser SVG engine for accuracy)
// ---------------------------------------------------------------------------

/** Compute the bounding box of an SVG path d string via the browser's SVG DOM */
function getPathBBox(d: string): { x: number; y: number; w: number; h: number } {
  try {
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path')
    path.setAttribute('d', d)
    svg.appendChild(path)
    svg.style.position = 'absolute'
    svg.style.width = '0'
    svg.style.height = '0'
    svg.style.overflow = 'hidden'
    document.body.appendChild(svg)
    const bbox = path.getBBox()
    document.body.removeChild(svg)
    return { x: bbox.x, y: bbox.y, w: bbox.width, h: bbox.height }
  } catch {
    return { x: 0, y: 0, w: 100, h: 100 }
  }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

function num(el: Element, attr: string): number {
  return parseFloat(getAttr(el, attr) ?? '0') || 0
}

/** Compute the bounding box that encloses all children (including stroke extent) */
function computeChildrenBounds(children: PenNode[]): { x: number; y: number; w: number; h: number } {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const child of children) {
    const cx = child.x ?? 0
    const cy = child.y ?? 0
    // Expand by half stroke width so the group fully contains visual bounds
    const stroke = 'stroke' in child ? (child as PenNode & { stroke?: PenStroke }).stroke : undefined
    const thickness = stroke?.thickness
    const halfStroke = (typeof thickness === 'number' ? thickness : 0) / 2
    if (child.type === 'line') {
      const lineChild = child as LineNode
      const x2 = lineChild.x2 ?? cx
      const y2 = lineChild.y2 ?? cy
      minX = Math.min(minX, cx - halfStroke, x2 - halfStroke)
      minY = Math.min(minY, cy - halfStroke, y2 - halfStroke)
      maxX = Math.max(maxX, cx + halfStroke, x2 + halfStroke)
      maxY = Math.max(maxY, cy + halfStroke, y2 + halfStroke)
    } else {
      const sized = child as PenNode & { width?: number; height?: number }
      const cw = sized.width ?? 0
      const ch = sized.height ?? 0
      minX = Math.min(minX, cx - halfStroke)
      minY = Math.min(minY, cy - halfStroke)
      maxX = Math.max(maxX, cx + cw + halfStroke)
      maxY = Math.max(maxY, cy + ch + halfStroke)
    }
  }
  if (!isFinite(minX)) return { x: 0, y: 0, w: 0, h: 0 }
  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY }
}

/** Offset a child node's position (make relative to parent origin) */
function offsetChild(node: PenNode, dx: number, dy: number) {
  const mutable = node as PenNodeBase
  if (node.type === 'line') {
    const lineNode = node as LineNode
    mutable.x = (mutable.x ?? 0) + dx
    mutable.y = (mutable.y ?? 0) + dy
    lineNode.x2 = (lineNode.x2 ?? 0) + dx
    lineNode.y2 = (lineNode.y2 ?? 0) + dy
  } else {
    mutable.x = (mutable.x ?? 0) + dx
    mutable.y = (mutable.y ?? 0) + dy
  }
}

function pointsToD(points: string, close: boolean): string {
  const nums = points.trim().split(/[\s,]+/).map(Number)
  if (nums.length < 2) return ''
  let d = `M${nums[0]} ${nums[1]}`
  for (let i = 2; i < nums.length - 1; i += 2) {
    d += `L${nums[i]} ${nums[i + 1]}`
  }
  if (close) d += 'Z'
  return d
}
