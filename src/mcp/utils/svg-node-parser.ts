/**
 * Node.js-compatible SVG → PenNode[] parser.
 * Uses regex-based XML parsing — no browser DOMParser or document.* APIs.
 *
 * Reuses patterns proven in:
 * - icon-resolver.ts → featherBodyToPathD() (shape → path conversion)
 * - server/api/ai/icon.ts → parseIconBody() (regex SVG extraction)
 * - svg-parser.ts → scaleSvgPath() (path coordinate scaling)
 */

import type { PenNode } from '../../types/pen'
import type { PenFill, PenStroke } from '../../types/styles'
import { generateId } from '../utils/id'

interface StyleCtx {
  fill: string | null
  stroke: string | null
  strokeWidth: number
}

// Tags to skip during parsing
const SKIP_TAGS = new Set([
  'defs', 'style', 'title', 'desc', 'metadata',
  'clippath', 'mask', 'filter', 'lineargradient', 'radialgradient',
  'symbol', 'marker', 'pattern', 'script', 'foreignobject',
  'animate', 'animatemotion', 'set',
])

/**
 * Parse an SVG string into PenNode array. Node.js compatible (no browser APIs).
 */
export function parseSvgToNodesServer(
  svgText: string,
  maxDim = 400,
): PenNode[] {
  // Extract <svg> tag and its attributes
  const svgMatch = svgText.match(/<svg\b([^>]*)>([\s\S]*)<\/svg>/i)
  if (!svgMatch) return []

  const svgAttrs = svgMatch[1]
  const svgBody = svgMatch[2]

  // Parse viewBox and dimensions
  const vbMatch = svgAttrs.match(/\bviewBox\s*=\s*"([^"]+)"/)
  const vb = vbMatch?.[1].split(/[\s,]+/).map(Number)
  const vbW = vb?.[2] || 100
  const vbH = vb?.[3] || 100
  // Presentation size — only use if pure numeric (ignore "1em", "100%", etc.)
  const rawW = extractAttr(svgAttrs, 'width') ?? ''
  const rawH = extractAttr(svgAttrs, 'height') ?? ''
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

  // Build root style context
  const rootCtx: StyleCtx = {
    fill: extractAttr(svgAttrs, 'fill'),
    stroke: extractAttr(svgAttrs, 'stroke'),
    strokeWidth: parseFloat(extractAttr(svgAttrs, 'stroke-width') ?? '') || 1,
  }

  const nodes = parseElements(svgBody, scale, rootCtx)

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
// Element parsing (regex-based recursive descent)
// ---------------------------------------------------------------------------

/** Parse all elements inside an SVG/g body string */
function parseElements(body: string, scale: number, ctx: StyleCtx): PenNode[] {
  const nodes: PenNode[] = []

  // Match opening tags (self-closing or paired) at the top level
  // We need to handle <g> specially since it has children
  const tagRe = /<(\w+)\b([^>]*?)(?:\/>|>([\s\S]*?)<\/\1>)/gi
  let match: RegExpExecArray | null

  while ((match = tagRe.exec(body)) !== null) {
    const tag = match[1].toLowerCase()
    const attrs = match[2]
    const innerBody = match[3] ?? ''

    if (SKIP_TAGS.has(tag)) continue

    const node = parseTag(tag, attrs, innerBody, scale, ctx)
    if (node) nodes.push(node)
  }

  return nodes
}

function parseTag(
  tag: string,
  attrs: string,
  innerBody: string,
  scale: number,
  parentCtx: StyleCtx,
): PenNode | null {
  // Merge style context
  const ctx = mergeStyleCtx(parentCtx, attrs)

  // Handle <g> by recursing
  if (tag === 'g') {
    const children = parseElements(innerBody, scale, ctx)
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
      name: extractAttr(attrs, 'id') ?? 'Group',
      x: bounds.x,
      y: bounds.y,
      width: Math.ceil(bounds.w),
      height: Math.ceil(bounds.h),
      layout: 'none' as const,
      children,
    }
  }

  // Resolve fill & stroke
  const fill = resolveFill(attrs, ctx)
  const stroke = resolveStroke(attrs, ctx, scale)
  const opacity = parseFloat(extractAttr(attrs, 'opacity') ?? '1')

  const base = {
    id: generateId(),
    opacity: opacity !== 1 ? opacity : undefined,
  }

  switch (tag) {
    case 'path': {
      const d = extractAttr(attrs, 'd')
      if (!d) return null
      const scaledD = scaleSvgPath(d, scale)
      const bbox = estimatePathBBox(scaledD)
      return {
        ...base,
        type: 'path' as const,
        name: extractAttr(attrs, 'id') ?? 'Path',
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
      const x = attrNum(attrs, 'x') * scale
      const y = attrNum(attrs, 'y') * scale
      const w = attrNum(attrs, 'width') * scale
      const h = attrNum(attrs, 'height') * scale
      const rx = attrNum(attrs, 'rx') * scale
      return {
        ...base,
        type: 'rectangle' as const,
        name: extractAttr(attrs, 'id') ?? 'Rectangle',
        x, y,
        width: Math.ceil(w),
        height: Math.ceil(h),
        cornerRadius: rx || undefined,
        fill,
        stroke,
      }
    }

    case 'circle': {
      const cx = attrNum(attrs, 'cx') * scale
      const cy = attrNum(attrs, 'cy') * scale
      const r = attrNum(attrs, 'r') * scale
      return {
        ...base,
        type: 'ellipse' as const,
        name: extractAttr(attrs, 'id') ?? 'Circle',
        x: cx - r, y: cy - r,
        width: Math.ceil(r * 2),
        height: Math.ceil(r * 2),
        fill,
        stroke,
      }
    }

    case 'ellipse': {
      const cx = attrNum(attrs, 'cx') * scale
      const cy = attrNum(attrs, 'cy') * scale
      const rx = attrNum(attrs, 'rx') * scale
      const ry = attrNum(attrs, 'ry') * scale
      return {
        ...base,
        type: 'ellipse' as const,
        name: extractAttr(attrs, 'id') ?? 'Ellipse',
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
        name: extractAttr(attrs, 'id') ?? 'Line',
        x: attrNum(attrs, 'x1') * scale,
        y: attrNum(attrs, 'y1') * scale,
        x2: attrNum(attrs, 'x2') * scale,
        y2: attrNum(attrs, 'y2') * scale,
        stroke: stroke ?? { thickness: 1, fill: [{ type: 'solid', color: '#000000' }] },
      }
    }

    case 'polygon':
    case 'polyline': {
      const pts = extractAttr(attrs, 'points')
      if (!pts) return null
      const d = pointsToD(pts, tag === 'polygon')
      const scaledD = scaleSvgPath(d, scale)
      const bbox = estimatePathBBox(scaledD)
      return {
        ...base,
        type: 'path' as const,
        name: extractAttr(attrs, 'id') ?? (tag === 'polygon' ? 'Polygon' : 'Polyline'),
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
// Style resolution
// ---------------------------------------------------------------------------

function mergeStyleCtx(parent: StyleCtx, attrs: string): StyleCtx {
  return {
    fill: extractStyleOrAttr(attrs, 'fill') ?? parent.fill,
    stroke: extractStyleOrAttr(attrs, 'stroke') ?? parent.stroke,
    strokeWidth:
      parseFloat(extractStyleOrAttr(attrs, 'stroke-width') ?? '') || parent.strokeWidth,
  }
}

function normalizeColor(raw: string): string {
  if (raw === 'currentColor' || raw === 'inherit') return '#000000'
  return raw
}

function noFill(): PenFill[] {
  return [{ type: 'solid', color: 'transparent' }]
}

function resolveFill(attrs: string, ctx: StyleCtx): PenFill[] | undefined {
  const raw = extractStyleOrAttr(attrs, 'fill') ?? ctx.fill
  if (raw === 'none') return noFill()
  if (raw && raw.startsWith('url(')) return [{ type: 'solid', color: '#000000' }]
  if (raw) return [{ type: 'solid', color: normalizeColor(raw) }]
  return [{ type: 'solid', color: '#000000' }]
}

function resolveStroke(attrs: string, ctx: StyleCtx, scale: number): PenStroke | undefined {
  const raw = extractStyleOrAttr(attrs, 'stroke') ?? ctx.stroke
  if (!raw || raw === 'none') return undefined
  if (raw.startsWith('url(')) return undefined
  const width =
    (parseFloat(extractStyleOrAttr(attrs, 'stroke-width') ?? '') || ctx.strokeWidth) * scale
  return {
    thickness: width,
    fill: [{ type: 'solid', color: normalizeColor(raw) }],
  }
}

// ---------------------------------------------------------------------------
// Attribute extraction (regex-based)
// ---------------------------------------------------------------------------

/** Extract an attribute value from an attrs string */
function extractAttr(attrs: string, name: string): string | null {
  const re = new RegExp(`\\b${name}\\s*=\\s*"([^"]*)"`)
  return attrs.match(re)?.[1] ?? null
}

/** Extract attribute value, checking inline style first */
function extractStyleOrAttr(attrs: string, name: string): string | null {
  // Check inline style= first
  const styleMatch = attrs.match(/\bstyle\s*=\s*"([^"]*)"/)
  if (styleMatch) {
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const m = styleMatch[1].match(new RegExp(`${escaped}\\s*:\\s*([^;]+)`))
    if (m) return m[1].trim()
  }
  return extractAttr(attrs, name)
}

/** Parse a numeric attribute from an attrs string, defaulting to 0 */
function parseAttrNum(attrs: string, name: string): number {
  const val = extractAttr(attrs, name)
  return val ? parseFloat(val) || 0 : 0
}

function attrNum(attrs: string, name: string): number {
  return parseAttrNum(attrs, name)
}

// ---------------------------------------------------------------------------
// SVG path scaling (copied from svg-parser.ts — pure string ops, Node.js safe)
// ---------------------------------------------------------------------------

function scaleSvgPath(d: string, scale: number): string {
  if (scale === 1) return d

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
      const pos = paramIdx % 7
      const shouldScale = pos === 0 || pos === 1 || pos === 5 || pos === 6
      result += ' ' + (shouldScale ? n * scale : n)
    } else {
      result += ' ' + n * scale
    }
    paramIdx++
  }

  return result.trim()
}

// ---------------------------------------------------------------------------
// Path bounding box estimation (no browser DOM)
// ---------------------------------------------------------------------------

/**
 * Estimate bounding box from SVG path coordinates.
 * Scans all numeric coordinates in the path to find min/max.
 * Not pixel-perfect for curves, but sufficient for layout.
 */
function estimatePathBBox(d: string): { x: number; y: number; w: number; h: number } {
  const tokens = d.match(/[a-zA-Z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g)
  if (!tokens) return { x: 0, y: 0, w: 100, h: 100 }

  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  let curX = 0
  let curY = 0
  let cmd = ''
  let paramIdx = 0

  function track(x: number, y: number) {
    if (x < minX) minX = x
    if (x > maxX) maxX = x
    if (y < minY) minY = y
    if (y > maxY) maxY = y
  }

  for (const tok of tokens) {
    if (/[a-zA-Z]/.test(tok)) {
      cmd = tok
      paramIdx = 0
      continue
    }

    const n = parseFloat(tok)
    const upper = cmd.toUpperCase()
    const isRel = cmd !== upper

    switch (upper) {
      case 'M':
      case 'L':
      case 'T': {
        if (paramIdx % 2 === 0) {
          curX = isRel ? curX + n : n
        } else {
          curY = isRel ? curY + n : n
          track(curX, curY)
        }
        break
      }
      case 'H': {
        curX = isRel ? curX + n : n
        track(curX, curY)
        break
      }
      case 'V': {
        curY = isRel ? curY + n : n
        track(curX, curY)
        break
      }
      case 'C': {
        // 6 params: cx1 cy1 cx2 cy2 x y
        const pos = paramIdx % 6
        if (pos % 2 === 0) {
          const x = isRel ? curX + n : n
          track(x, curY) // approximate: track control points too
          if (pos === 4) curX = x
        } else {
          const y = isRel ? curY + n : n
          track(curX, y)
          if (pos === 5) curY = y
        }
        break
      }
      case 'S':
      case 'Q': {
        // S: 4 params (cx2 cy2 x y), Q: 4 params (cx cy x y)
        const pos = paramIdx % 4
        if (pos % 2 === 0) {
          const x = isRel ? curX + n : n
          track(x, curY)
          if (pos === 2) curX = x
        } else {
          const y = isRel ? curY + n : n
          track(curX, y)
          if (pos === 3) curY = y
        }
        break
      }
      case 'A': {
        // 7 params: rx ry rotation large-arc sweep x y
        const pos = paramIdx % 7
        if (pos === 5) {
          curX = isRel ? curX + n : n
        } else if (pos === 6) {
          curY = isRel ? curY + n : n
          track(curX, curY)
        }
        break
      }
      case 'Z':
        break
    }
    paramIdx++
  }

  if (!isFinite(minX)) return { x: 0, y: 0, w: 100, h: 100 }
  return {
    x: Math.floor(minX),
    y: Math.floor(minY),
    w: Math.ceil(maxX - Math.floor(minX)) || 1,
    h: Math.ceil(maxY - Math.floor(minY)) || 1,
  }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/** Compute the bounding box that encloses all children (including stroke extent) */
function computeChildrenBounds(children: PenNode[]): { x: number; y: number; w: number; h: number } {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const child of children) {
    const cx = (child as any).x ?? 0
    const cy = (child as any).y ?? 0
    // Expand by half stroke width so the group fully contains visual bounds
    const halfStroke = ((child as any).stroke?.thickness ?? 0) / 2
    if (child.type === 'line') {
      const x2 = (child as any).x2 ?? cx
      const y2 = (child as any).y2 ?? cy
      minX = Math.min(minX, cx - halfStroke, x2 - halfStroke)
      minY = Math.min(minY, cy - halfStroke, y2 - halfStroke)
      maxX = Math.max(maxX, cx + halfStroke, x2 + halfStroke)
      maxY = Math.max(maxY, cy + halfStroke, y2 + halfStroke)
    } else {
      const cw = (child as any).width ?? 0
      const ch = (child as any).height ?? 0
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
  if (node.type === 'line') {
    ;(node as any).x = ((node as any).x ?? 0) + dx
    ;(node as any).y = ((node as any).y ?? 0) + dy
    ;(node as any).x2 = ((node as any).x2 ?? 0) + dx
    ;(node as any).y2 = ((node as any).y2 ?? 0) + dy
  } else {
    ;(node as any).x = ((node as any).x ?? 0) + dx
    ;(node as any).y = ((node as any).y ?? 0) + dy
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
