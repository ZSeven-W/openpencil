import type {
  FigmaNodeChange,
  FigmaGUID,
  FigmaDecodedFile,
  FigmaMatrix,
} from './figma-types'
import type { PenNode, PenPage, PenDocument } from '@/types/pen'
import { mapFigmaFills } from './figma-fill-mapper'
import { mapFigmaStroke } from './figma-stroke-mapper'
import { mapFigmaEffects } from './figma-effect-mapper'
import { mapFigmaLayout, mapWidthSizing, mapHeightSizing } from './figma-layout-mapper'
import { mapFigmaTextProps } from './figma-text-mapper'
import { lookupIconByName } from '@/services/ai/icon-resolver'

const SKIPPED_TYPES = new Set([
  'SLICE', 'CONNECTOR', 'SHAPE_WITH_TEXT', 'STICKY', 'STAMP',
  'HIGHLIGHT', 'WASHI_TAPE', 'CODE_BLOCK', 'MEDIA', 'WIDGET',
  'SECTION_OVERLAY', 'NONE',
])

interface TreeNode {
  figma: FigmaNodeChange
  children: TreeNode[]
}

/**
 * Convert a decoded .fig file to a PenDocument.
 */
export function figmaToPenDocument(
  decoded: FigmaDecodedFile,
  fileName: string,
  pageIndex: number = 0,
): { document: PenDocument; warnings: string[]; imageBlobs: Map<number, Uint8Array> } {
  const warnings: string[] = []

  // Build tree from flat nodeChanges array
  const tree = buildTree(decoded.nodeChanges)

  if (!tree) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No document root found'],
      imageBlobs: new Map(),
    }
  }

  // Find pages (CANVAS nodes are children of DOCUMENT)
  const pages = tree.children.filter((c) => c.figma.type === 'CANVAS')
  const page = pages[pageIndex] ?? pages[0]

  if (!page) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No pages found in Figma file'],
      imageBlobs: new Map(),
    }
  }

  // Collect component (SYMBOL) nodes for instance resolution
  const componentMap = new Map<string, string>()
  let idCounter = 1
  collectComponents(page, componentMap, () => `fig_${idCounter++}`)

  // Convert the page's children to PenNodes
  const ctx: ConversionContext = {
    componentMap,
    warnings,
    generateId: () => `fig_${idCounter++}`,
    blobs: decoded.blobs,
  }

  const children = convertChildren(page, ctx)

  // Collect image blobs referenced by fills
  const imageBlobs = collectImageBlobs(decoded.blobs)

  const pageName = page.figma.name ?? 'Page 1'
  const penPage: PenPage = {
    id: `figma-page-${pageIndex}`,
    name: pageName,
    children,
  }

  return {
    document: {
      version: '1',
      name: fileName,
      pages: [penPage],
      children: [],
    },
    warnings,
    imageBlobs,
  }
}

/**
 * Convert ALL pages from a decoded .fig file into a single PenDocument.
 * Each page's children are placed side by side with a horizontal gap.
 */
export function figmaAllPagesToPenDocument(
  decoded: FigmaDecodedFile,
  fileName: string,
): { document: PenDocument; warnings: string[]; imageBlobs: Map<number, Uint8Array> } {
  const warnings: string[] = []

  const tree = buildTree(decoded.nodeChanges)
  if (!tree) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No document root found'],
      imageBlobs: new Map(),
    }
  }

  const pages = tree.children.filter((c) => c.figma.type === 'CANVAS')
  if (pages.length === 0) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No pages found in Figma file'],
      imageBlobs: new Map(),
    }
  }

  // Collect components across ALL pages first (shared context)
  const componentMap = new Map<string, string>()
  let idCounter = 1
  const genId = () => `fig_${idCounter++}`
  for (const page of pages) {
    collectComponents(page, componentMap, genId)
  }

  const penPages: PenPage[] = []

  for (let i = 0; i < pages.length; i++) {
    const page = pages[i]
    const ctx: ConversionContext = {
      componentMap,
      warnings,
      generateId: genId,
      blobs: decoded.blobs,
    }

    const pageChildren = convertChildren(page, ctx)
    const pageName = page.figma.name ?? `Page ${i + 1}`

    penPages.push({
      id: `figma-page-${i}`,
      name: pageName,
      children: pageChildren,
    })
  }

  const imageBlobs = collectImageBlobs(decoded.blobs)

  return {
    document: {
      version: '1',
      name: fileName,
      pages: penPages,
      children: [],
    },
    warnings,
    imageBlobs,
  }
}

/**
 * Get pages from a decoded .fig file.
 */
export function getFigmaPages(
  decoded: FigmaDecodedFile
): { id: string; name: string; childCount: number }[] {
  const tree = buildTree(decoded.nodeChanges)
  if (!tree) return []

  return tree.children
    .filter((c) => c.figma.type === 'CANVAS')
    .map((c) => ({
      id: guidToString(c.figma.guid!),
      name: c.figma.name ?? 'Page',
      childCount: c.children.length,
    }))
}

// --- Tree building ---

function guidToString(guid: FigmaGUID): string {
  return `${guid.sessionID}:${guid.localID}`
}

function buildTree(nodeChanges: FigmaNodeChange[]): TreeNode | null {
  // Index nodes by GUID
  const nodeMap = new Map<string, TreeNode>()
  let root: TreeNode | null = null

  for (const nc of nodeChanges) {
    if (!nc.guid) continue
    if (nc.phase === 'REMOVED') continue
    const key = guidToString(nc.guid)
    nodeMap.set(key, { figma: nc, children: [] })
  }

  // Build parent-child relationships
  for (const nc of nodeChanges) {
    if (!nc.guid || nc.phase === 'REMOVED') continue
    const key = guidToString(nc.guid)
    const treeNode = nodeMap.get(key)
    if (!treeNode) continue

    if (nc.type === 'DOCUMENT') {
      root = treeNode
      continue
    }

    if (nc.parentIndex?.guid) {
      const parentKey = guidToString(nc.parentIndex.guid)
      const parent = nodeMap.get(parentKey)
      if (parent) {
        parent.children.push(treeNode)
      }
    }
  }

  // Sort children by position (fractional index string)
  if (root) {
    sortChildrenRecursive(root)
  }

  return root
}

function sortChildrenRecursive(node: TreeNode): void {
  node.children.sort((a, b) => {
    const posA = a.figma.parentIndex?.position ?? ''
    const posB = b.figma.parentIndex?.position ?? ''
    // Use raw code-point comparison, not localeCompare.
    // Figma fractional index strings use characters like $ (36), % (37), & (38)
    // where lower code points = further back in z-stack. localeCompare sorts
    // these symbols incorrectly, causing background elements to render on top.
    return posA < posB ? -1 : posA > posB ? 1 : 0
  })
  for (const child of node.children) {
    sortChildrenRecursive(child)
  }
}

// --- Component collection ---

function collectComponents(
  node: TreeNode,
  map: Map<string, string>,
  genId: () => string,
): void {
  if (node.figma.type === 'SYMBOL' && node.figma.guid) {
    const figmaId = guidToString(node.figma.guid)
    map.set(figmaId, genId())
  }
  for (const child of node.children) {
    collectComponents(child, map, genId)
  }
}

// --- Conversion ---

interface ConversionContext {
  componentMap: Map<string, string>
  warnings: string[]
  generateId: () => string
  blobs: (Uint8Array | string)[]
}

function convertChildren(
  parent: TreeNode,
  ctx: ConversionContext,
): PenNode[] {
  const parentStackMode = parent.figma.stackMode
  const result: PenNode[] = []

  for (const child of parent.children) {
    if (child.figma.visible === false) continue
    const node = convertNode(child, parentStackMode, ctx)
    if (node) result.push(node)
  }

  return result
}

function convertNode(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode | null {
  const figma = treeNode.figma
  if (!figma.type || SKIPPED_TYPES.has(figma.type)) return null

  switch (figma.type) {
    case 'FRAME':
    case 'SECTION':
      return convertFrame(treeNode, parentStackMode, ctx)

    case 'GROUP':
      return convertGroup(treeNode, parentStackMode, ctx)

    case 'SYMBOL':
      return convertComponent(treeNode, parentStackMode, ctx)

    case 'INSTANCE':
      return convertInstance(treeNode, parentStackMode, ctx)

    case 'RECTANGLE':
    case 'ROUNDED_RECTANGLE':
      return convertRectangle(treeNode, parentStackMode, ctx)

    case 'ELLIPSE':
      return convertEllipse(treeNode, parentStackMode, ctx)

    case 'LINE':
      return convertLine(treeNode, ctx)

    case 'VECTOR':
    case 'STAR':
    case 'REGULAR_POLYGON':
    case 'BOOLEAN_OPERATION':
      return convertVector(treeNode, parentStackMode, ctx)

    case 'TEXT':
      return convertText(treeNode, parentStackMode, ctx)

    default: {
      if (treeNode.children.length > 0) {
        return convertFrame(treeNode, parentStackMode, ctx)
      }
      ctx.warnings.push(`Skipped unsupported node type: ${figma.type} (${figma.name})`)
      return null
    }
  }
}

function extractPosition(figma: FigmaNodeChange): { x: number; y: number } {
  if (figma.transform) {
    return {
      x: Math.round(figma.transform.m02),
      y: Math.round(figma.transform.m12),
    }
  }
  return { x: 0, y: 0 }
}

function extractRotation(transform?: FigmaMatrix): number | undefined {
  if (!transform) return undefined
  const angle = Math.atan2(transform.m10, transform.m00) * (180 / Math.PI)
  const rounded = Math.round(angle)
  return rounded !== 0 ? rounded : undefined
}

function mapCornerRadius(
  figma: FigmaNodeChange
): number | [number, number, number, number] | undefined {
  if (figma.rectangleCornerRadiiIndependent) {
    const tl = figma.rectangleTopLeftCornerRadius ?? 0
    const tr = figma.rectangleTopRightCornerRadius ?? 0
    const br = figma.rectangleBottomRightCornerRadius ?? 0
    const bl = figma.rectangleBottomLeftCornerRadius ?? 0
    if (tl === tr && tr === br && br === bl) {
      return tl > 0 ? tl : undefined
    }
    return [tl, tr, br, bl]
  }
  if (figma.cornerRadius && figma.cornerRadius > 0) {
    return figma.cornerRadius
  }
  return undefined
}

function commonProps(
  figma: FigmaNodeChange,
  id: string,
): { id: string; name?: string; x: number; y: number; rotation?: number; opacity?: number; locked?: boolean } {
  const { x, y } = extractPosition(figma)
  return {
    id,
    name: figma.name || undefined,
    x,
    y,
    rotation: extractRotation(figma.transform),
    opacity: figma.opacity !== undefined && figma.opacity < 1 ? figma.opacity : undefined,
    locked: figma.locked || undefined,
  }
}

// --- Node converters ---

function convertFrame(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()
  const layout = mapFigmaLayout(figma)
  const children = convertChildren(treeNode, ctx)

  // Check for image-only fill
  if (hasOnlyImageFill(figma) && children.length === 0) {
    return {
      type: 'image',
      ...commonProps(figma, id),
      src: getImageFillUrl(figma),
      width: mapWidthSizing(figma, parentStackMode),
      height: mapHeightSizing(figma, parentStackMode),
      cornerRadius: mapCornerRadius(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  return {
    type: 'frame',
    ...commonProps(figma, id),
    width: mapWidthSizing(figma, parentStackMode),
    height: mapHeightSizing(figma, parentStackMode),
    ...layout,
    cornerRadius: mapCornerRadius(figma),
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
    children: children.length > 0 ? children : undefined,
  }
}

function convertGroup(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()
  const children = convertChildren(treeNode, ctx)

  return {
    type: 'group',
    ...commonProps(figma, id),
    width: mapWidthSizing(figma, parentStackMode),
    height: mapHeightSizing(figma, parentStackMode),
    children: children.length > 0 ? children : undefined,
  }
}

function convertComponent(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const figmaId = figma.guid ? guidToString(figma.guid) : ''
  const id = ctx.componentMap.get(figmaId) ?? ctx.generateId()
  const layout = mapFigmaLayout(figma)
  const children = convertChildren(treeNode, ctx)

  return {
    type: 'frame',
    ...commonProps(figma, id),
    reusable: true,
    width: mapWidthSizing(figma, parentStackMode),
    height: mapHeightSizing(figma, parentStackMode),
    ...layout,
    cornerRadius: mapCornerRadius(figma),
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
    children: children.length > 0 ? children : undefined,
  }
}

function convertInstance(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const componentGuid = figma.overriddenSymbolID ?? figma.symbolData?.symbolID
  const componentPenId = componentGuid
    ? ctx.componentMap.get(guidToString(componentGuid))
    : undefined

  // If we can't resolve the component, convert as a regular frame
  if (!componentPenId) {
    return convertFrame(treeNode, parentStackMode, ctx)
  }

  const id = ctx.generateId()
  return {
    type: 'ref',
    ...commonProps(figma, id),
    ref: componentPenId,
  }
}

function convertRectangle(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()

  if (hasOnlyImageFill(figma)) {
    return {
      type: 'image',
      ...commonProps(figma, id),
      src: getImageFillUrl(figma),
      width: mapWidthSizing(figma, parentStackMode),
      height: mapHeightSizing(figma, parentStackMode),
      cornerRadius: mapCornerRadius(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  return {
    type: 'rectangle',
    ...commonProps(figma, id),
    width: mapWidthSizing(figma, parentStackMode),
    height: mapHeightSizing(figma, parentStackMode),
    cornerRadius: mapCornerRadius(figma),
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  }
}

function convertEllipse(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()

  // Ellipse with image-only fill → convert to image node (e.g. circular avatar)
  if (hasOnlyImageFill(figma)) {
    return {
      type: 'image',
      ...commonProps(figma, id),
      src: getImageFillUrl(figma),
      width: mapWidthSizing(figma, parentStackMode),
      height: mapHeightSizing(figma, parentStackMode),
      cornerRadius: Math.round((figma.size?.x ?? 100) / 2),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  return {
    type: 'ellipse',
    ...commonProps(figma, id),
    width: mapWidthSizing(figma, parentStackMode),
    height: mapHeightSizing(figma, parentStackMode),
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  }
}

function convertLine(
  treeNode: TreeNode,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()
  const { x, y } = extractPosition(figma)
  const w = figma.size?.x ?? 100

  return {
    type: 'line',
    id,
    name: figma.name || undefined,
    x,
    y,
    x2: x + w,
    y2: y,
    rotation: extractRotation(figma.transform),
    opacity: figma.opacity !== undefined && figma.opacity < 1 ? figma.opacity : undefined,
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  }
}

function convertVector(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()
  const name = figma.name ?? ''

  // 1. Try matching node name to a known icon (Lucide/Feather library)
  const iconMatch = lookupIconByName(name)
  if (iconMatch) {
    return {
      type: 'path',
      ...commonProps(figma, id),
      d: iconMatch.d,
      iconId: iconMatch.iconId,
      width: mapWidthSizing(figma, parentStackMode),
      height: mapHeightSizing(figma, parentStackMode),
      fill: iconMatch.style === 'fill' ? mapFigmaFills(figma.fillPaints) : undefined,
      stroke: iconMatch.style === 'stroke'
        ? mapFigmaStroke(figma) ?? { thickness: 2, fill: [{ type: 'solid', color: figmaFillColor(figma) ?? '#000000' }] }
        : mapFigmaStroke(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  // 2. Try decoding binary path data from fillGeometry / strokeGeometry blobs
  const pathD = decodeFigmaVectorPath(figma, ctx.blobs)
  if (pathD) {
    return {
      type: 'path',
      ...commonProps(figma, id),
      d: pathD,
      width: mapWidthSizing(figma, parentStackMode),
      height: mapHeightSizing(figma, parentStackMode),
      fill: mapFigmaFills(figma.fillPaints),
      stroke: mapFigmaStroke(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  // 3. Fall back to rectangle if path decoding fails
  ctx.warnings.push(
    `Vector node "${figma.name}" converted as rectangle (path data not decodable)`
  )
  return {
    type: 'rectangle',
    ...commonProps(figma, id),
    width: mapWidthSizing(figma, parentStackMode),
    height: mapHeightSizing(figma, parentStackMode),
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  }
}

function convertText(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()
  const textProps = mapFigmaTextProps(figma)

  return {
    type: 'text',
    ...commonProps(figma, id),
    width: mapWidthSizing(figma, parentStackMode),
    height: mapHeightSizing(figma, parentStackMode),
    ...textProps,
    fill: mapFigmaFills(figma.fillPaints),
    effects: mapFigmaEffects(figma.effects),
  }
}

// --- Vector path decoding ---

/**
 * Decode Figma binary path blob to SVG path `d` string.
 * Binary format: sequence of commands, each starting with a command byte:
 *   0x00 = closePath (Z) — 0 floats
 *   0x01 = moveTo (M)    — 2 float32 LE (x, y)
 *   0x02 = lineTo (L)    — 2 float32 LE (x, y)
 *   0x04 = cubicTo (C)   — 6 float32 LE (cp1x, cp1y, cp2x, cp2y, x, y)
 *   0x03 = quadTo (Q)    — 4 float32 LE (cpx, cpy, x, y)
 */
function decodeFigmaPathBlob(blob: Uint8Array): string | null {
  if (blob.length < 9) return null // minimum: 1 cmd byte + 2 float32

  const buf = new ArrayBuffer(blob.byteLength)
  new Uint8Array(buf).set(blob)
  const view = new DataView(buf)

  const parts: string[] = []
  let offset = 0

  while (offset < blob.length) {
    const cmd = blob[offset]
    offset += 1

    switch (cmd) {
      case 0x00: // close
        parts.push('Z')
        break
      case 0x01: { // moveTo
        if (offset + 8 > blob.length) return joinParts(parts)
        const x = view.getFloat32(offset, true); offset += 4
        const y = view.getFloat32(offset, true); offset += 4
        parts.push(`M${r(x)} ${r(y)}`)
        break
      }
      case 0x02: { // lineTo
        if (offset + 8 > blob.length) return joinParts(parts)
        const x = view.getFloat32(offset, true); offset += 4
        const y = view.getFloat32(offset, true); offset += 4
        parts.push(`L${r(x)} ${r(y)}`)
        break
      }
      case 0x03: { // quadTo
        if (offset + 16 > blob.length) return joinParts(parts)
        const cpx = view.getFloat32(offset, true); offset += 4
        const cpy = view.getFloat32(offset, true); offset += 4
        const x   = view.getFloat32(offset, true); offset += 4
        const y   = view.getFloat32(offset, true); offset += 4
        parts.push(`Q${r(cpx)} ${r(cpy)} ${r(x)} ${r(y)}`)
        break
      }
      case 0x04: { // cubicTo
        if (offset + 24 > blob.length) return joinParts(parts)
        const cp1x = view.getFloat32(offset, true); offset += 4
        const cp1y = view.getFloat32(offset, true); offset += 4
        const cp2x = view.getFloat32(offset, true); offset += 4
        const cp2y = view.getFloat32(offset, true); offset += 4
        const x    = view.getFloat32(offset, true); offset += 4
        const y    = view.getFloat32(offset, true); offset += 4
        parts.push(`C${r(cp1x)} ${r(cp1y)} ${r(cp2x)} ${r(cp2y)} ${r(x)} ${r(y)}`)
        break
      }
      default:
        // Unknown command — stop decoding
        return joinParts(parts)
    }
  }

  return joinParts(parts)
}

/** Round to 2 decimal places for compact SVG path data. */
function r(n: number): string {
  return Math.abs(n) < 0.005 ? '0' : parseFloat(n.toFixed(2)).toString()
}

function joinParts(parts: string[]): string | null {
  return parts.length > 1 ? parts.join(' ') : null
}

/**
 * Try to decode vector path data from a Figma node's fill/stroke geometry blobs.
 * Scales coordinates from normalizedSize to actual node size if needed.
 */
function decodeFigmaVectorPath(
  figma: FigmaNodeChange,
  blobs: (Uint8Array | string)[],
): string | null {
  // Try fillGeometry first, then strokeGeometry
  const geometries = figma.fillGeometry ?? figma.strokeGeometry
  if (!geometries || geometries.length === 0) return null

  const pathParts: string[] = []

  for (const geom of geometries) {
    if (geom.commandsBlob == null) continue
    const blob = blobs[geom.commandsBlob]
    if (!blob || typeof blob === 'string') continue
    const decoded = decodeFigmaPathBlob(blob)
    if (decoded) pathParts.push(decoded)
  }

  if (pathParts.length === 0) return null

  const rawPath = pathParts.join(' ')

  // Scale from normalizedSize to actual node size if they differ
  const normSize = figma.vectorData?.normalizedSize
  const actualSize = figma.size
  if (normSize && actualSize) {
    const sx = actualSize.x / normSize.x
    const sy = actualSize.y / normSize.y
    if (Math.abs(sx - 1) > 0.01 || Math.abs(sy - 1) > 0.01) {
      return scaleSvgPath(rawPath, sx, sy)
    }
  }

  return rawPath
}

/** Scale all coordinates in an SVG path string. */
function scaleSvgPath(d: string, sx: number, sy: number): string {
  // Tokenize: commands and numbers
  const tokens = d.match(/[MLCQZmlcqz]|-?\d+\.?\d*/g)
  if (!tokens) return d

  const result: string[] = []
  let i = 0

  while (i < tokens.length) {
    const token = tokens[i]
    if (/^[MLCQZmlcqz]$/.test(token)) {
      result.push(token)
      i++
      const cmd = token.toUpperCase()
      const count = cmd === 'M' || cmd === 'L' ? 2 : cmd === 'Q' ? 4 : cmd === 'C' ? 6 : 0
      for (let j = 0; j < count && i < tokens.length; j++) {
        const val = parseFloat(tokens[i])
        result.push(r(j % 2 === 0 ? val * sx : val * sy))
        i++
      }
    } else {
      result.push(token)
      i++
    }
  }

  return result.join(' ')
}

/**
 * Extract the primary fill color from a Figma node's fillPaints.
 */
function figmaFillColor(figma: FigmaNodeChange): string | undefined {
  const paint = figma.fillPaints?.find((f) => f.visible !== false && f.type === 'SOLID')
  if (!paint?.color) return undefined
  const { r: cr, g: cg, b: cb } = paint.color
  const toHex = (v: number) => Math.round(v * 255).toString(16).padStart(2, '0')
  return `#${toHex(cr)}${toHex(cg)}${toHex(cb)}`
}

// --- Helpers ---

function hasOnlyImageFill(figma: FigmaNodeChange): boolean {
  if (!figma.fillPaints || figma.fillPaints.length === 0) return false
  const visible = figma.fillPaints.filter((f) => f.visible !== false)
  return visible.length === 1 && visible[0].type === 'IMAGE'
}

function hashToHex(hash: Uint8Array): string {
  return Array.from(hash).map(b => b.toString(16).padStart(2, '0')).join('')
}

function getImageFillUrl(figma: FigmaNodeChange): string {
  const paint = figma.fillPaints?.find((f) => f.type === 'IMAGE' && f.visible !== false)
  if (!paint?.image) return ''

  // Prefer hash-based reference (resolves from ZIP image files)
  if (paint.image.hash && paint.image.hash.length > 0) {
    return `__hash:${hashToHex(paint.image.hash)}`
  }

  // Fall back to blob index reference
  if (paint.image.dataBlob !== undefined && paint.image.dataBlob !== null) {
    return `__blob:${paint.image.dataBlob}`
  }

  return ''
}

function collectImageBlobs(blobs: (Uint8Array | string)[]): Map<number, Uint8Array> {
  const map = new Map<number, Uint8Array>()
  for (let i = 0; i < blobs.length; i++) {
    const blob = blobs[i]
    if (blob instanceof Uint8Array && blob.length > 0) {
      // Check if it looks like image data (PNG starts with 0x89 0x50)
      if (blob[0] === 0x89 && blob[1] === 0x50) {
        map.set(i, blob)
      }
    }
  }
  return map
}
