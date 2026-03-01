import { decodeBinarySchema, compileSchema } from 'kiwi-schema'
import { inflateRaw } from 'pako'
import { decompress as zstdDecompress } from 'fzstd'
import type { PenNode, FrameNode, TextNode } from '@/types/pen'
import type { PenFill, PenStroke, PenEffect, SolidFill } from '@/types/styles'

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/** Check if HTML clipboard data contains Figma markers */
export function isFigmaHTML(html: string): boolean {
  return html.includes('<!--(figma)') && html.includes('<!--(figmeta)')
}

// ---------------------------------------------------------------------------
// Figma clipboard binary decoder (replaces fig-kiwi)
// Handles both deflate (pako) and zstandard (fzstd) compression.
// ---------------------------------------------------------------------------

const META_START = '<!--(figmeta)'
const META_END = '(/figmeta)-->'
const FIG_START = '<!--(figma)'
const FIG_END = '(/figma)-->'

function parseHTMLString(html: string) {
  const msi = html.indexOf(META_START)
  const mei = html.indexOf(META_END)
  const fsi = html.indexOf(FIG_START)
  const fei = html.indexOf(FIG_END)
  if (msi === -1 || fsi === -1) throw new Error('Missing figma clipboard markers')

  const metaB64 = html.substring(msi + META_START.length, mei)
  const figB64 = html.substring(fsi + FIG_START.length, fei)

  const meta = JSON.parse(atob(metaB64))
  // Decode base64 to Uint8Array
  const binStr = atob(figB64)
  const figma = new Uint8Array(binStr.length)
  for (let i = 0; i < binStr.length; i++) figma[i] = binStr.charCodeAt(i)

  return { meta, figma }
}

function parseArchive(data: Uint8Array) {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength)
  let offset = 0

  // Read prelude string (expect "fig-kiwi")
  const prelude = String.fromCharCode(...data.slice(0, 8))
  if (prelude !== 'fig-kiwi') throw new Error(`Unexpected prelude: "${prelude}"`)
  offset = 8

  // Read version
  const version = view.getUint32(offset, true)
  offset += 4

  // Read chunks
  const chunks: Uint8Array[] = []
  while (offset + 4 < data.length) {
    const size = view.getUint32(offset, true)
    offset += 4
    chunks.push(data.slice(offset, offset + size))
    offset += size
  }
  return { version, chunks }
}

/** Try deflate first, then zstandard */
function decompressChunk(chunk: Uint8Array): Uint8Array {
  // Try deflate (pako inflateRaw)
  try {
    const result = inflateRaw(chunk)
    return result
  } catch {
    // Not deflate
  }
  // Try zstandard
  try {
    const result = zstdDecompress(chunk)
    return result
  } catch {
    // Not zstandard either
  }
  throw new Error(`[figma-paste] neither deflate nor zstd could decompress (${chunk.length} bytes)`)
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function readHTMLMessage(html: string): { meta: any; message: any } {
  const { meta, figma } = parseHTMLString(html)
  const { chunks } = parseArchive(figma)

  if (chunks.length < 2) throw new Error(`Expected ≥2 archive chunks, got ${chunks.length}`)

  const schemaData = decompressChunk(chunks[0])
  const messageData = decompressChunk(chunks[1])

  const schema = decodeBinarySchema(schemaData)
  const compiled = compileSchema(schema)
  const message = compiled.decodeMessage(messageData)

  return { meta, message }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

interface TreeNode {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  nc: any // NodeChange from fig-kiwi – all fields optional
  children: TreeNode[]
}

// ---------------------------------------------------------------------------
// GUID helpers
// ---------------------------------------------------------------------------

function guidKey(guid: { sessionID: number; localID: number }): string {
  return `${guid.sessionID}:${guid.localID}`
}

// ---------------------------------------------------------------------------
// Tree builder
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function buildNodeTree(nodeChanges: any[]): TreeNode[] {
  const map = new Map<string, TreeNode>()
  const roots: TreeNode[] = []

  for (const nc of nodeChanges) {
    if (!nc.guid) continue
    map.set(guidKey(nc.guid), { nc, children: [] })
  }

  for (const nc of nodeChanges) {
    if (!nc.guid) continue
    const treeNode = map.get(guidKey(nc.guid))!
    if (nc.parentIndex?.guid) {
      const parent = map.get(guidKey(nc.parentIndex.guid))
      if (parent) {
        parent.children.push(treeNode)
      } else {
        roots.push(treeNode)
      }
    } else {
      roots.push(treeNode)
    }
  }

  // Sort children by the position string that encodes sibling order
  for (const [, tn] of map) {
    tn.children.sort((a, b) => {
      const pa = a.nc.parentIndex?.position ?? ''
      const pb = b.nc.parentIndex?.position ?? ''
      return pa < pb ? -1 : pa > pb ? 1 : 0
    })
  }

  return roots
}

// ---------------------------------------------------------------------------
// Color / fill helpers
// ---------------------------------------------------------------------------

function figmaColorToHex(c: {
  r: number
  g: number
  b: number
  a?: number
}): string {
  const r = Math.round(c.r * 255)
  const g = Math.round(c.g * 255)
  const b = Math.round(c.b * 255)
  return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`
}

function figmaColorToHexAlpha(c: {
  r: number
  g: number
  b: number
  a?: number
}): string {
  const hex = figmaColorToHex(c)
  if (c.a !== undefined && c.a < 1) {
    return hex + Math.round(c.a * 255).toString(16).padStart(2, '0')
  }
  return hex
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function convertFills(paints: any[] | undefined): PenFill[] | undefined {
  if (!paints || paints.length === 0) return undefined
  const fills: PenFill[] = []

  for (const p of paints) {
    if (p.visible === false) continue
    switch (p.type) {
      case 'SOLID':
        if (p.color) {
          const f: SolidFill = { type: 'solid', color: figmaColorToHex(p.color) }
          if (p.opacity !== undefined && p.opacity < 1) f.opacity = p.opacity
          fills.push(f)
        }
        break

      case 'GRADIENT_LINEAR':
        if (p.stops?.length) {
          fills.push({
            type: 'linear_gradient',
            stops: p.stops.map((s: { color?: { r: number; g: number; b: number; a?: number }; position?: number }) => ({
              offset: s.position ?? 0,
              color: s.color ? figmaColorToHexAlpha(s.color) : '#000000',
            })),
            opacity: p.opacity,
          })
        }
        break

      case 'GRADIENT_RADIAL':
      case 'GRADIENT_ANGULAR':
      case 'GRADIENT_DIAMOND':
        if (p.stops?.length) {
          fills.push({
            type: 'radial_gradient',
            stops: p.stops.map((s: { color?: { r: number; g: number; b: number; a?: number }; position?: number }) => ({
              offset: s.position ?? 0,
              color: s.color ? figmaColorToHexAlpha(s.color) : '#000000',
            })),
            opacity: p.opacity,
          })
        }
        break

      case 'IMAGE':
        // Image data doesn't survive clipboard transfer – gray placeholder
        fills.push({ type: 'solid', color: '#CCCCCC' })
        break
    }
  }

  return fills.length > 0 ? fills : undefined
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function convertStroke(nc: any): PenStroke | undefined {
  const fills = convertFills(nc.strokePaints)
  if (!fills) return undefined

  const stroke: PenStroke = { thickness: nc.strokeWeight ?? 1, fill: fills }
  if (nc.strokeAlign === 'INSIDE') stroke.align = 'inside'
  else if (nc.strokeAlign === 'OUTSIDE') stroke.align = 'outside'
  if (nc.strokeCap === 'ROUND') stroke.cap = 'round'
  else if (nc.strokeCap === 'SQUARE') stroke.cap = 'square'
  if (nc.strokeJoin === 'ROUND') stroke.join = 'round'
  else if (nc.strokeJoin === 'BEVEL') stroke.join = 'bevel'
  if (nc.dashPattern?.length) stroke.dashPattern = nc.dashPattern
  return stroke
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function convertEffects(effects: any[] | undefined): PenEffect[] | undefined {
  if (!effects?.length) return undefined
  const out: PenEffect[] = []

  for (const e of effects) {
    if (e.visible === false) continue
    switch (e.type) {
      case 'DROP_SHADOW':
      case 'INNER_SHADOW':
        out.push({
          type: 'shadow',
          inner: e.type === 'INNER_SHADOW',
          offsetX: e.offset?.x ?? 0,
          offsetY: e.offset?.y ?? 0,
          blur: e.radius ?? 0,
          spread: e.spread ?? 0,
          color: e.color ? figmaColorToHexAlpha(e.color) : '#00000040',
        })
        break
      case 'FOREGROUND_BLUR':
        out.push({ type: 'blur', radius: e.radius ?? 0 })
        break
      case 'BACKGROUND_BLUR':
        out.push({ type: 'background_blur', radius: e.radius ?? 0 })
        break
    }
  }

  return out.length > 0 ? out : undefined
}

// ---------------------------------------------------------------------------
// Corner radius
// ---------------------------------------------------------------------------

/** Figma internally stores "max radius" as a very large int (e.g. 33554400).
 *  Values > 1000 (Figma UI max) are clamped to min(width, height) / 2 (pill shape). */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function convertCornerRadius(
  nc: any,
  width?: number,
  height?: number,
): number | [number, number, number, number] | undefined {
  const maxVisual =
    width !== undefined && height !== undefined
      ? Math.min(width, height) / 2
      : 999

  function clamp(v: number): number {
    return v > 1000 ? maxVisual : v
  }

  if (nc.rectangleCornerRadiiIndependent) {
    let tl = clamp(nc.rectangleTopLeftCornerRadius ?? 0)
    let tr = clamp(nc.rectangleTopRightCornerRadius ?? 0)
    let br = clamp(nc.rectangleBottomRightCornerRadius ?? 0)
    let bl = clamp(nc.rectangleBottomLeftCornerRadius ?? 0)
    if (tl === 0 && tr === 0 && br === 0 && bl === 0) return undefined
    if (tl === tr && tr === br && br === bl) return tl
    return [tl, tr, br, bl]
  }
  const r = clamp(nc.cornerRadius ?? 0)
  return r > 0 ? r : undefined
}

// ---------------------------------------------------------------------------
// Position / transform
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function getPosition(nc: any): { x: number; y: number; rotation?: number } {
  if (!nc.transform) return { x: 0, y: 0 }
  const t = nc.transform
  const x = t.m02 ?? 0
  const y = t.m12 ?? 0
  const rotation = Math.atan2(t.m10 ?? 0, t.m00 ?? 1) * (180 / Math.PI)
  return { x, y, rotation: Math.abs(rotation) > 0.01 ? rotation : undefined }
}

// ---------------------------------------------------------------------------
// Layout (auto-layout)
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function convertLayout(nc: any): Partial<FrameNode> {
  const props: Partial<FrameNode> = {}

  if (nc.stackMode === 'VERTICAL') props.layout = 'vertical'
  else if (nc.stackMode === 'HORIZONTAL') props.layout = 'horizontal'

  if (nc.stackSpacing > 0) props.gap = nc.stackSpacing

  // Padding – Figma stores per-side or symmetric values
  const hasPerSide =
    nc.stackPaddingRight !== undefined ||
    nc.stackPaddingBottom !== undefined ||
    nc.stackHorizontalPadding !== undefined ||
    nc.stackVerticalPadding !== undefined

  if (!hasPerSide && nc.stackPadding > 0) {
    props.padding = nc.stackPadding
  } else {
    const top = nc.stackPadding ?? nc.stackVerticalPadding ?? 0
    const right = nc.stackPaddingRight ?? nc.stackHorizontalPadding ?? 0
    const bottom = nc.stackPaddingBottom ?? nc.stackVerticalPadding ?? 0
    const left = nc.stackHorizontalPadding ?? 0
    if (top > 0 || right > 0 || bottom > 0 || left > 0) {
      if (top === right && right === bottom && bottom === left) {
        props.padding = top
      } else if (top === bottom && left === right) {
        props.padding = [top, left]
      } else {
        props.padding = [top, right, bottom, left]
      }
    }
  }

  // Primary-axis distribution
  switch (nc.stackJustify) {
    case 'CENTER':
      props.justifyContent = 'center'
      break
    case 'MAX':
      props.justifyContent = 'end'
      break
    case 'SPACE_EVENLY':
      props.justifyContent = 'space_between'
      break
  }

  // Cross-axis alignment
  switch (nc.stackCounterAlign) {
    case 'CENTER':
      props.alignItems = 'center'
      break
    case 'MAX':
      props.alignItems = 'end'
      break
  }

  if (nc.frameMaskDisabled === false) props.clipContent = true

  return props
}

// ---------------------------------------------------------------------------
// Font weight from Figma font-style string
// ---------------------------------------------------------------------------

function parseFontWeight(style?: string): number | undefined {
  if (!style) return undefined
  const l = style.toLowerCase()
  if (l.includes('thin') || l.includes('hairline')) return 100
  if (l.includes('extralight') || l.includes('ultralight')) return 200
  if (l.includes('light')) return 300
  if (l.includes('regular') || l.includes('normal')) return 400
  if (l.includes('medium')) return 500
  if (l.includes('semibold') || l.includes('demibold')) return 600
  if (l.includes('extrabold') || l.includes('ultrabold')) return 800
  if (l.includes('bold')) return 700
  if (l.includes('black') || l.includes('heavy')) return 900
  return undefined
}

// ---------------------------------------------------------------------------
// Image placeholder
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function hasImageFill(nc: any): boolean {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return nc.fillPaints?.some((p: any) => p.type === 'IMAGE') ?? false
}

function createImagePlaceholder(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  nc: any,
  pos: { x: number; y: number; rotation?: number },
): FrameNode {
  return {
    id: crypto.randomUUID(),
    type: 'frame',
    name: nc.name || 'Image',
    x: pos.x,
    y: pos.y,
    rotation: pos.rotation,
    width: nc.size?.x ?? 100,
    height: nc.size?.y ?? 100,
    fill: [{ type: 'solid', color: '#CCCCCC' }],
    layout: 'vertical',
    justifyContent: 'center',
    alignItems: 'center',
    children: [
      {
        id: crypto.randomUUID(),
        type: 'text',
        content: 'Image',
        fontSize: 14,
        fontWeight: 500,
        fill: [{ type: 'solid', color: '#666666' }],
      } as TextNode,
    ],
  }
}

// ---------------------------------------------------------------------------
// Node converter (recursive)
// ---------------------------------------------------------------------------

function convertNode(tn: TreeNode): PenNode | null {
  const nc = tn.nc
  const type: string | undefined = nc.type

  // Skip non-visual / structural
  if (!type || type === 'DOCUMENT' || type === 'CANVAS' || type === 'SLICE')
    return null
  if (nc.visible === false) return null

  // Image fills → placeholder
  if (hasImageFill(nc)) return createImagePlaceholder(nc, getPosition(nc))

  // Recurse children
  const children = tn.children
    .map(convertNode)
    .filter((n): n is PenNode => n !== null)

  const pos = getPosition(nc)
  const base = {
    id: crypto.randomUUID(),
    name: nc.name as string | undefined,
    x: pos.x,
    y: pos.y,
    rotation: pos.rotation,
    opacity:
      nc.opacity !== undefined && nc.opacity < 1 ? nc.opacity : undefined,
  }

  switch (type) {
    // Frames, sections, components, instances → frame
    case 'FRAME':
    case 'SECTION':
    case 'SYMBOL':
    case 'INSTANCE': {
      const layout = convertLayout(nc)
      const node: FrameNode = {
        ...base,
        type: 'frame',
        width: nc.size?.x,
        height: nc.size?.y,
        ...layout,
        cornerRadius: convertCornerRadius(nc, nc.size?.x, nc.size?.y),
        fill: convertFills(nc.fillPaints),
        stroke: convertStroke(nc),
        effects: convertEffects(nc.effects),
        children: children.length > 0 ? children : undefined,
      }
      return node
    }

    case 'GROUP':
      return {
        ...base,
        type: 'group',
        children: children.length > 0 ? children : undefined,
      } as PenNode

    case 'RECTANGLE':
    case 'ROUNDED_RECTANGLE':
      return {
        ...base,
        type: 'rectangle',
        width: nc.size?.x,
        height: nc.size?.y,
        cornerRadius: convertCornerRadius(nc, nc.size?.x, nc.size?.y),
        fill: convertFills(nc.fillPaints),
        stroke: convertStroke(nc),
        effects: convertEffects(nc.effects),
      } as PenNode

    case 'ELLIPSE':
      return {
        ...base,
        type: 'ellipse',
        width: nc.size?.x,
        height: nc.size?.y,
        fill: convertFills(nc.fillPaints),
        stroke: convertStroke(nc),
        effects: convertEffects(nc.effects),
      } as PenNode

    case 'TEXT': {
      const content: string = nc.textData?.characters ?? ''
      let textAlign: 'left' | 'center' | 'right' | 'justify' | undefined
      switch (nc.textAlignHorizontal) {
        case 'CENTER':
          textAlign = 'center'
          break
        case 'RIGHT':
          textAlign = 'right'
          break
        case 'JUSTIFIED':
          textAlign = 'justify'
          break
      }
      let textAlignVertical: 'top' | 'middle' | 'bottom' | undefined
      switch (nc.textAlignVertical) {
        case 'CENTER':
          textAlignVertical = 'middle'
          break
        case 'BOTTOM':
          textAlignVertical = 'bottom'
          break
      }
      let lineHeight: number | undefined
      if (nc.lineHeight?.value) {
        lineHeight =
          nc.lineHeight.units === 'PERCENT'
            ? (nc.lineHeight.value / 100) * (nc.fontSize ?? 16)
            : nc.lineHeight.value
      }
      let letterSpacing: number | undefined
      if (nc.letterSpacing?.value && nc.letterSpacing.units === 'PIXELS') {
        letterSpacing = nc.letterSpacing.value
      }
      let textGrowth: 'auto' | 'fixed-width' | 'fixed-width-height' | undefined
      switch (nc.textAutoResize) {
        case 'WIDTH_AND_HEIGHT':
          textGrowth = 'auto'
          break
        case 'HEIGHT':
          textGrowth = 'fixed-width'
          break
        case 'NONE':
          textGrowth = 'fixed-width-height'
          break
      }
      return {
        ...base,
        type: 'text',
        width: nc.size?.x,
        height: nc.size?.y,
        content,
        fontFamily: nc.fontName?.family,
        fontSize: nc.fontSize,
        fontWeight: parseFontWeight(nc.fontName?.style),
        fontStyle: nc.fontName?.style?.toLowerCase().includes('italic')
          ? 'italic'
          : undefined,
        letterSpacing,
        lineHeight,
        textAlign,
        textAlignVertical,
        textGrowth,
        underline: nc.textDecoration === 'UNDERLINE' ? true : undefined,
        strikethrough:
          nc.textDecoration === 'STRIKETHROUGH' ? true : undefined,
        fill: convertFills(nc.fillPaints),
        effects: convertEffects(nc.effects),
      } as PenNode
    }

    case 'LINE':
      return {
        ...base,
        type: 'line',
        x2: nc.size?.x ?? 100,
        y2: 0,
        stroke: convertStroke(nc) ?? {
          thickness: 1,
          fill: [{ type: 'solid', color: '#000000' }],
        },
        effects: convertEffects(nc.effects),
      } as PenNode

    // Vector, Star, Polygon → rectangle placeholder
    case 'VECTOR':
    case 'STAR':
    case 'REGULAR_POLYGON':
    case 'BOOLEAN_OPERATION':
      return {
        ...base,
        type: 'rectangle',
        width: nc.size?.x,
        height: nc.size?.y,
        fill: convertFills(nc.fillPaints) ?? [
          { type: 'solid', color: '#CCCCCC' },
        ],
        stroke: convertStroke(nc),
        effects: convertEffects(nc.effects),
      } as PenNode

    default:
      return null
  }
}

// ---------------------------------------------------------------------------
// Coordinate normalization – absolute → parent-relative
// ---------------------------------------------------------------------------

function normalizeCoordinates(
  nodes: PenNode[],
  parentX = 0,
  parentY = 0,
): void {
  for (const node of nodes) {
    node.x = (node.x ?? 0) - parentX
    node.y = (node.y ?? 0) - parentY

    if ('children' in node && node.children) {
      const absX = parentX + node.x
      const absY = parentY + node.y
      normalizeCoordinates(node.children, absX, absY)
    }
  }
}

// ---------------------------------------------------------------------------
// Viewport centering
// ---------------------------------------------------------------------------

export function offsetToViewportCenter(
  nodes: PenNode[],
  centerX: number,
  centerY: number,
): void {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity

  for (const n of nodes) {
    const x = n.x ?? 0
    const y = n.y ?? 0
    const w = 'width' in n && typeof n.width === 'number' ? n.width : 0
    const h = 'height' in n && typeof n.height === 'number' ? n.height : 0
    minX = Math.min(minX, x)
    minY = Math.min(minY, y)
    maxX = Math.max(maxX, x + w)
    maxY = Math.max(maxY, y + h)
  }

  if (!isFinite(minX)) return

  const dx = centerX - (minX + maxX) / 2
  const dy = centerY - (minY + maxY) / 2

  for (const n of nodes) {
    n.x = (n.x ?? 0) + dx
    n.y = (n.y ?? 0) + dy
  }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/** Decode Figma clipboard HTML and convert to PenNode[]. */
export function parseFigmaClipboard(html: string): PenNode[] {
  const parsed = readHTMLMessage(html)
  const nodeChanges = parsed.message?.nodeChanges
  if (!nodeChanges?.length) return []

  const tree = buildNodeTree(nodeChanges)

  // Recursively unwrap DOCUMENT / CANVAS wrappers to reach content nodes
  function collectContentRoots(roots: TreeNode[]): TreeNode[] {
    const content: TreeNode[] = []
    for (const root of roots) {
      if (root.nc.type === 'DOCUMENT' || root.nc.type === 'CANVAS') {
        content.push(...collectContentRoots(root.children))
      } else {
        content.push(root)
      }
    }
    return content
  }

  const contentRoots = collectContentRoots(tree)

  const penNodes: PenNode[] = []
  for (const root of contentRoots) {
    const converted = convertNode(root)
    if (converted) penNodes.push(converted)
  }

  normalizeCoordinates(penNodes)
  return penNodes
}
