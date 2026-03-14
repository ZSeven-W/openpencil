import { useMemo } from 'react'
import type { PenNode, ContainerProps } from '@/types/pen'
import type { PenFill } from '@/types/styles'
import type { VariableDefinition } from '@/types/variables'
import { resolvePadding, type Padding } from '@/canvas/canvas-layout-engine'
import { parseSizing, estimateTextWidth } from '@/canvas/canvas-text-measure'
import { resolveColorRef, isVariableRef, resolveNumericRef } from '@/variables/resolve-variables'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface NodePreviewSvgProps {
  node: PenNode
  maxWidth: number
  maxHeight: number
  variables?: Record<string, VariableDefinition>
}

type Vars = Record<string, VariableDefinition>

const MAX_DEPTH = 6

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

function resolveFillColor(fills: PenFill[] | undefined, vars: Vars): string | undefined {
  if (!fills?.length) return undefined
  const f = fills[0]
  if (f.type === 'solid') {
    return resolveColorRef(f.color, vars) ?? f.color
  }
  if ((f.type === 'linear_gradient' || f.type === 'radial_gradient') && f.stops.length > 0) {
    return resolveColorRef(f.stops[0].color, vars) ?? f.stops[0].color
  }
  return undefined
}

function resolveStrokeColor(node: PenNode, vars: Vars): string | undefined {
  const stroke = 'stroke' in node ? (node as PenNode & { stroke?: import('@/types/styles').PenStroke }).stroke : undefined
  if (!stroke?.fill?.length) return undefined
  const f = stroke.fill[0]
  if (f.type === 'solid') {
    return resolveColorRef(f.color, vars) ?? f.color
  }
  return undefined
}

function resolveStrokeWidth(node: PenNode, vars: Vars): number {
  const stroke = 'stroke' in node ? (node as PenNode & { stroke?: import('@/types/styles').PenStroke }).stroke : undefined
  if (!stroke) return 0
  const t = stroke.thickness
  if (typeof t === 'number') return t
  if (typeof t === 'string' && isVariableRef(t)) {
    return (resolveNumericRef(t, vars) as number) ?? 1
  }
  if (Array.isArray(t)) return t[0] ?? 1
  return 0
}

// ---------------------------------------------------------------------------
// Size helpers (store-free)
// ---------------------------------------------------------------------------

function nodeWidth(node: PenNode, parentAvail?: number): number {
  if ('width' in node) {
    const s = parseSizing(node.width)
    if (typeof s === 'number' && s > 0) return s
    if (s === 'fill' && parentAvail && parentAvail > 0) return parentAvail
  }
  if (node.type === 'text') {
    const fontSize = node.fontSize ?? 16
    const content = typeof node.content === 'string'
      ? node.content
      : node.content.map((seg) => seg.text).join('')
    return Math.max(Math.ceil(estimateTextWidth(content, fontSize, node.letterSpacing ?? 0)), 10)
  }
  return 0
}

function nodeHeight(node: PenNode, parentAvail?: number): number {
  if ('height' in node) {
    const s = parseSizing(node.height)
    if (typeof s === 'number' && s > 0) return s
    if (s === 'fill' && parentAvail && parentAvail > 0) return parentAvail
  }
  if (node.type === 'text') {
    const fontSize = node.fontSize ?? 16
    const lineHeight = ('lineHeight' in node ? node.lineHeight : undefined) ?? 1.2
    return Math.ceil(fontSize * lineHeight)
  }
  return 0
}

// ---------------------------------------------------------------------------
// Layout computation (store-free)
// ---------------------------------------------------------------------------

interface LayoutChild {
  node: PenNode
  x: number
  y: number
  w: number
  h: number
}

function computeLayout(parent: PenNode, vars: Vars): LayoutChild[] {
  const children = ('children' in parent ? parent.children : undefined) ?? []
  const visible = children.filter((c) => ('visible' in c ? c.visible : true) !== false)
  if (!visible.length) return []

  const c = parent as PenNode & ContainerProps
  const layout = c.layout
  if (!layout || layout === 'none') {
    return visible.map((ch) => ({
      node: ch,
      x: ch.x ?? 0,
      y: ch.y ?? 0,
      w: nodeWidth(ch),
      h: nodeHeight(ch),
    }))
  }

  const pWRaw = parseSizing(c.width)
  const pW = typeof pWRaw === 'number' && pWRaw > 0 ? pWRaw : 100
  const pHRaw = parseSizing(c.height)
  const pH = typeof pHRaw === 'number' && pHRaw > 0 ? pHRaw : 100
  const rawPad = typeof c.padding === 'string' && isVariableRef(c.padding)
    ? (resolveNumericRef(c.padding, vars) as number) ?? 0
    : c.padding
  const pad = resolvePadding(rawPad as Padding['top'] | [number, number] | [number, number, number, number] | undefined)
  const rawGap = typeof c.gap === 'string' && isVariableRef(c.gap)
    ? (resolveNumericRef(c.gap, vars) as number) ?? 0
    : typeof c.gap === 'number' ? c.gap : 0
  const gap = rawGap as number

  const isVert = layout === 'vertical'
  const availW = pW - pad.left - pad.right
  const availH = pH - pad.top - pad.bottom

  // Measure children
  const sizes = visible.map((ch) => {
    const wSizing = 'width' in ch ? parseSizing(ch.width) : 0
    const hSizing = 'height' in ch ? parseSizing(ch.height) : 0
    return {
      w: wSizing === 'fill' ? availW : nodeWidth(ch, availW),
      h: hSizing === 'fill' ? availH : nodeHeight(ch, availH),
      wFill: wSizing === 'fill',
      hFill: hSizing === 'fill',
    }
  })

  // Fill distribution on main axis
  const totalGap = gap * Math.max(0, visible.length - 1)
  const fixedMain = sizes.reduce((sum, s) => {
    const mainProp = isVert ? s.h : s.w
    const isFill = isVert ? s.hFill : s.wFill
    return sum + (isFill ? 0 : mainProp)
  }, 0)
  const fillCount = sizes.filter((s) => isVert ? s.hFill : s.wFill).length
  const remaining = Math.max(0, (isVert ? availH : availW) - fixedMain - totalGap)
  const fillSize = fillCount > 0 ? remaining / fillCount : 0

  // Apply fill sizes
  for (const s of sizes) {
    if (isVert && s.hFill) s.h = fillSize
    if (!isVert && s.wFill) s.w = fillSize
    // Cross-axis fill
    if (isVert && s.wFill) s.w = availW
    if (!isVert && s.hFill) s.h = availH
  }

  const justify = normalizeJustify(c.justifyContent)
  const align = normalizeAlign(c.alignItems)

  const totalMain = sizes.reduce((sum, s) => sum + (isVert ? s.h : s.w), 0)
  const freeSpace = Math.max(0, (isVert ? availH : availW) - totalMain - totalGap)

  let mainPos = 0
  if (justify === 'center') mainPos = freeSpace / 2
  else if (justify === 'end') mainPos = freeSpace

  return visible.map((ch, i) => {
    const s = sizes[i]
    const crossAvail = isVert ? availW : availH
    const childCross = isVert ? s.w : s.h
    let crossPos = 0
    if (align === 'center') crossPos = (crossAvail - childCross) / 2
    else if (align === 'end') crossPos = crossAvail - childCross

    const x = isVert ? pad.left + crossPos : pad.left + mainPos
    const y = isVert ? pad.top + mainPos : pad.top + crossPos

    mainPos += (isVert ? s.h : s.w) + gap

    return { node: ch, x: x || 0, y: y || 0, w: s.w || 0, h: s.h || 0 }
  })
}

function normalizeJustify(v: unknown): 'start' | 'center' | 'end' {
  if (typeof v !== 'string') return 'start'
  if (v === 'center' || v === 'middle') return 'center'
  if (v === 'end' || v === 'flex-end' || v === 'right' || v === 'bottom') return 'end'
  return 'start'
}

function normalizeAlign(v: unknown): 'start' | 'center' | 'end' {
  if (typeof v !== 'string') return 'start'
  if (v === 'center' || v === 'middle') return 'center'
  if (v === 'end' || v === 'flex-end' || v === 'right' || v === 'bottom') return 'end'
  return 'start'
}

// ---------------------------------------------------------------------------
// Rounded rect path (for per-corner radii)
// ---------------------------------------------------------------------------

function roundedRectPath(
  w: number, h: number,
  tl: number, tr: number, br: number, bl: number,
): string {
  // Clamp radii to half the smallest dimension
  const maxR = Math.min(w, h) / 2
  tl = Math.min(tl, maxR)
  tr = Math.min(tr, maxR)
  br = Math.min(br, maxR)
  bl = Math.min(bl, maxR)
  return [
    `M${tl},0`,
    `H${w - tr}`,
    `A${tr},${tr},0,0,1,${w},${tr}`,
    `V${h - br}`,
    `A${br},${br},0,0,1,${w - br},${h}`,
    `H${bl}`,
    `A${bl},${bl},0,0,1,0,${h - bl}`,
    `V${tl}`,
    `A${tl},${tl},0,0,1,${tl},0`,
    'Z',
  ].join(' ')
}

// ---------------------------------------------------------------------------
// SVG node renderer
// ---------------------------------------------------------------------------

function renderNode(
  node: PenNode,
  vars: Vars,
  depth: number,
  parentW?: number,
  parentH?: number,
): React.ReactElement | null {
  if (depth > MAX_DEPTH) return null
  if (('visible' in node ? node.visible : true) === false) return null

  const opacity = typeof node.opacity === 'number' ? node.opacity
    : typeof node.opacity === 'string' && isVariableRef(node.opacity)
      ? (resolveNumericRef(node.opacity, vars) as number) ?? 1
      : 1

  // All nodes render at (0,0) in their local coordinate space.
  // Positioning is handled by the parent's <g transform="translate(...)">
  // via computeLayout, which captures each child's x/y.
  const x = 0
  const y = 0

  switch (node.type) {
    case 'frame':
    case 'rectangle': {
      const w = nodeWidth(node, parentW) || 0
      const h = nodeHeight(node, parentH) || 0
      if (w <= 0 && h <= 0) return null

      const fill = resolveFillColor('fill' in node ? node.fill : undefined, vars) ?? 'none'
      const strokeColor = resolveStrokeColor(node, vars)
      const strokeW = resolveStrokeWidth(node, vars)
      const cr = 'cornerRadius' in node ? node.cornerRadius : undefined

      const children = computeLayout(node, vars)
      const childElements = children.map((ch, i) => (
        <g key={ch.node.id ?? i} transform={`translate(${ch.x},${ch.y})`}>
          {renderNode(ch.node, vars, depth + 1, ch.w, ch.h)}
        </g>
      ))

      const clipId = node.id ? `clip-${node.id}` : undefined
      const shouldClip = 'clipContent' in node && node.clipContent

      if (Array.isArray(cr)) {
        const d = roundedRectPath(w, h, cr[0], cr[1], cr[2], cr[3])
        return (
          <g transform={`translate(${x},${y})`} opacity={opacity < 1 ? opacity : undefined}>
            {shouldClip && clipId && (
              <defs>
                <clipPath id={clipId}><path d={d} /></clipPath>
              </defs>
            )}
            <path
              d={d}
              fill={fill}
              stroke={strokeColor}
              strokeWidth={strokeW || undefined}
            />
            <g clipPath={shouldClip && clipId ? `url(#${clipId})` : undefined}>
              {childElements}
            </g>
          </g>
        )
      }

      const rx = typeof cr === 'number' ? cr : undefined
      return (
        <g transform={`translate(${x},${y})`} opacity={opacity < 1 ? opacity : undefined}>
          {shouldClip && clipId && (
            <defs>
              <clipPath id={clipId}>
                <rect width={w} height={h} rx={rx} />
              </clipPath>
            </defs>
          )}
          <rect
            width={w}
            height={h}
            rx={rx}
            fill={fill}
            stroke={strokeColor}
            strokeWidth={strokeW || undefined}
          />
          <g clipPath={shouldClip && clipId ? `url(#${clipId})` : undefined}>
            {childElements}
          </g>
        </g>
      )
    }

    case 'ellipse': {
      const w = nodeWidth(node, parentW) || 0
      const h = nodeHeight(node, parentH) || 0
      if (w <= 0 && h <= 0) return null

      const fill = resolveFillColor(node.fill, vars) ?? 'none'
      const strokeColor = resolveStrokeColor(node, vars)
      const strokeW = resolveStrokeWidth(node, vars)

      return (
        <ellipse
          cx={x + w / 2}
          cy={y + h / 2}
          rx={w / 2}
          ry={h / 2}
          fill={fill}
          stroke={strokeColor}
          strokeWidth={strokeW || undefined}
          opacity={opacity < 1 ? opacity : undefined}
        />
      )
    }

    case 'text': {
      const fontSize = node.fontSize ?? 16
      const fontWeight = node.fontWeight ?? 400
      const textAlign = node.textAlign ?? 'left'
      const content = typeof node.content === 'string'
        ? node.content
        : node.content.map((seg) => seg.text).join('')
      if (!content.trim()) return null

      const fills = node.fill
      const color = resolveFillColor(fills, vars) ?? '#000000'

      const w = nodeWidth(node, parentW)
      const h = parentH || (fontSize * (('lineHeight' in node ? node.lineHeight : undefined) ?? 1.2))
      let textX = x
      let anchor: 'start' | 'middle' | 'end' = 'start'
      if (textAlign === 'center' && w > 0) {
        textX = x + w / 2
        anchor = 'middle'
      } else if (textAlign === 'right' && w > 0) {
        textX = x + w
        anchor = 'end'
      }

      return (
        <text
          x={textX}
          y={y + h / 2}
          fontSize={fontSize}
          fontWeight={fontWeight}
          fontFamily="Inter, system-ui, sans-serif"
          fill={color}
          textAnchor={anchor}
          dominantBaseline="central"
          opacity={opacity < 1 ? opacity : undefined}
        >
          {content}
        </text>
      )
    }

    case 'path': {
      if (!node.d) return null
      const fill = resolveFillColor(node.fill, vars) ?? 'none'
      const strokeColor = resolveStrokeColor(node, vars)
      const strokeW = resolveStrokeWidth(node, vars)

      return (
        <path
          d={node.d}
          fill={fill}
          stroke={strokeColor}
          strokeWidth={strokeW || undefined}
          transform={`translate(${x},${y})`}
          opacity={opacity < 1 ? opacity : undefined}
        />
      )
    }

    case 'line': {
      const strokeColor = resolveStrokeColor(node, vars) ?? '#000000'
      const strokeW = resolveStrokeWidth(node, vars) || 1
      return (
        <line
          x1={x}
          y1={y}
          x2={node.x2 ?? x}
          y2={node.y2 ?? y}
          stroke={strokeColor}
          strokeWidth={strokeW}
          opacity={opacity < 1 ? opacity : undefined}
        />
      )
    }

    case 'group': {
      const children = computeLayout(node, vars)
      return (
        <g
          transform={`translate(${x},${y})`}
          opacity={opacity < 1 ? opacity : undefined}
        >
          {children.map((ch, i) => (
            <g key={ch.node.id ?? i} transform={`translate(${ch.x},${ch.y})`}>
              {renderNode(ch.node, vars, depth + 1, ch.w, ch.h)}
            </g>
          ))}
        </g>
      )
    }

    case 'image': {
      const w = nodeWidth(node, parentW) || 0
      const h = nodeHeight(node, parentH) || 0
      if (w <= 0 && h <= 0) return null
      const cr = 'cornerRadius' in node ? node.cornerRadius : undefined
      const rx = typeof cr === 'number' ? cr : Array.isArray(cr) ? cr[0] : undefined
      return (
        <rect
          x={x}
          y={y}
          width={w}
          height={h}
          rx={rx}
          fill="#E5E7EB"
          opacity={opacity < 1 ? opacity : undefined}
        />
      )
    }

    default:
      return null
  }
}

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

export default function NodePreviewSvg({ node, maxWidth, maxHeight, variables }: NodePreviewSvgProps) {
  const svg = useMemo(() => {
    const vars = variables ?? {}
    const w = nodeWidth(node)
    const h = nodeHeight(node)
    if (w <= 0 || h <= 0) return null

    // Add small padding to viewBox to prevent stroke clipping at edges
    const pad = 2
    const vbX = -pad
    const vbY = -pad
    const vbW = w + pad * 2
    const vbH = h + pad * 2

    const scale = Math.min(maxWidth / vbW, maxHeight / vbH, 1)
    const svgW = Math.max(8, Math.round(vbW * scale))
    const svgH = Math.max(6, Math.round(vbH * scale))

    return (
      <svg
        width={svgW}
        height={svgH}
        viewBox={`${vbX} ${vbY} ${vbW} ${vbH}`}
        xmlns="http://www.w3.org/2000/svg"
        className="transition-transform group-hover:scale-105"
      >
        {renderNode(node, vars, 0)}
      </svg>
    )
  }, [node, maxWidth, maxHeight, variables])

  return svg ?? <div className="w-16 h-8 rounded bg-muted" />
}
