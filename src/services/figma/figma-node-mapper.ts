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
    return posA.localeCompare(posB)
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

  // Try to get path data from fillGeometry blobs
  // Since blob data is binary, we can't easily extract SVG paths
  // Fall back to a rectangle placeholder
  ctx.warnings.push(
    `Vector node "${figma.name}" converted as rectangle (binary path data not extractable from .fig format)`
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

// --- Helpers ---

function hasOnlyImageFill(figma: FigmaNodeChange): boolean {
  if (!figma.fillPaints || figma.fillPaints.length === 0) return false
  const visible = figma.fillPaints.filter((f) => f.visible !== false)
  return visible.length === 1 && visible[0].type === 'IMAGE'
}

function getImageFillUrl(figma: FigmaNodeChange): string {
  const paint = figma.fillPaints?.find((f) => f.type === 'IMAGE' && f.visible !== false)
  if (!paint?.image?.dataBlob && paint?.image?.dataBlob !== 0) return ''
  return `__blob:${paint.image.dataBlob}`
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
