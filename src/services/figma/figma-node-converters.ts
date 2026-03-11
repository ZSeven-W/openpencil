import type {
  FigmaNodeChange, FigmaMatrix, FigmaImportLayoutMode,
  FigmaSymbolOverride, FigmaDerivedSymbolDataEntry, FigmaGUID,
} from './figma-types'
import type { PenNode, SizingBehavior, ImageFitMode } from '@/types/pen'
import { mapFigmaFills } from './figma-fill-mapper'
import { mapFigmaStroke } from './figma-stroke-mapper'
import { mapFigmaEffects } from './figma-effect-mapper'
import { mapFigmaLayout, mapWidthSizing, mapHeightSizing } from './figma-layout-mapper'
import { mapFigmaTextProps } from './figma-text-mapper'
import { decodeFigmaVectorPath } from './figma-vector-decoder'
import { lookupIconByName } from '@/services/ai/icon-resolver'
import type { TreeNode } from './figma-tree-builder'
import { guidToString } from './figma-tree-builder'

/** Scale tree children's transforms and sizes to fit a different parent size. */
function scaleTreeChildren(children: TreeNode[], sx: number, sy: number): TreeNode[] {
  if (Math.abs(sx - 1) < 0.001 && Math.abs(sy - 1) < 0.001) return children
  return children.map((child) => {
    const figma = { ...child.figma }
    if (figma.transform) {
      figma.transform = {
        ...figma.transform,
        m02: figma.transform.m02 * sx,
        m12: figma.transform.m12 * sy,
      }
    }
    if (figma.size) {
      figma.size = { x: figma.size.x * sx, y: figma.size.y * sy }
    }
    return {
      figma,
      children: scaleTreeChildren(child.children, sx, sy),
    }
  })
}

const SKIPPED_TYPES = new Set([
  'SLICE', 'CONNECTOR', 'SHAPE_WITH_TEXT', 'STICKY', 'STAMP',
  'HIGHLIGHT', 'WASHI_TAPE', 'CODE_BLOCK', 'MEDIA', 'WIDGET',
  'SECTION_OVERLAY', 'NONE',
])

export interface ConversionContext {
  componentMap: Map<string, string>
  /** SYMBOL TreeNodes keyed by figma GUID — includes internal canvases for instance inlining */
  symbolTree: Map<string, TreeNode>
  warnings: string[]
  generateId: () => string
  blobs: (Uint8Array | string)[]
  layoutMode: FigmaImportLayoutMode
}

// --- Size resolution ---

function resolveWidth(figma: FigmaNodeChange, parentStackMode: string | undefined, ctx: ConversionContext): SizingBehavior {
  if (ctx.layoutMode === 'preserve') return figma.size?.x ?? 100
  return mapWidthSizing(figma, parentStackMode)
}

function resolveHeight(figma: FigmaNodeChange, parentStackMode: string | undefined, ctx: ConversionContext): SizingBehavior {
  if (ctx.layoutMode === 'preserve') return figma.size?.y ?? 100
  return mapHeightSizing(figma, parentStackMode)
}

// --- Common property extraction ---

function extractPosition(figma: FigmaNodeChange): { x: number; y: number } {
  if (figma.transform) {
    return {
      x: Math.round(figma.transform.m02 * 100) / 100,
      y: Math.round(figma.transform.m12 * 100) / 100,
    }
  }
  return { x: 0, y: 0 }
}

function normalizeAngle(deg: number): number {
  let a = deg % 360
  if (a < 0) a += 360
  return Math.round(a * 100) / 100
}

function extractRotation(transform?: FigmaMatrix): number | undefined {
  if (!transform) return undefined
  // Use abs(m00) to ignore horizontal flip (which is handled separately as flipX)
  const angle = Math.atan2(transform.m10, Math.abs(transform.m00)) * (180 / Math.PI)
  const rounded = Math.round(angle)
  return rounded !== 0 ? rounded : undefined
}

function extractFlip(transform?: FigmaMatrix): { flipX?: boolean; flipY?: boolean } {
  if (!transform) return {}
  const result: { flipX?: boolean; flipY?: boolean } = {}
  // Determinant sign of the 2x2 rotation/scale sub-matrix detects reflection
  // m00*m11 - m01*m10 < 0 means a single-axis flip
  const det = transform.m00 * transform.m11 - transform.m01 * transform.m10
  if (det < -0.001) {
    // Check which axis is flipped by looking at the scale signs
    if (transform.m00 < 0) result.flipX = true
    else result.flipY = true
  }
  return result
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
): { id: string; name?: string; x: number; y: number; rotation?: number; opacity?: number; locked?: boolean; flipX?: boolean; flipY?: boolean } {
  const { x, y } = extractPosition(figma)
  const flip = extractFlip(figma.transform)
  return {
    id,
    name: figma.name || undefined,
    x,
    y,
    rotation: extractRotation(figma.transform),
    opacity: figma.opacity !== undefined && figma.opacity < 1 ? figma.opacity : undefined,
    locked: figma.locked || undefined,
    ...flip,
  }
}

// --- Image helpers ---

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

  if (paint.image.hash && paint.image.hash.length > 0) {
    return `__hash:${hashToHex(paint.image.hash)}`
  }

  if (paint.image.dataBlob !== undefined && paint.image.dataBlob !== null) {
    return `__blob:${paint.image.dataBlob}`
  }

  return ''
}

function getImageFitMode(figma: FigmaNodeChange): ImageFitMode | undefined {
  const paint = figma.fillPaints?.find(
    (f) => f.visible !== false && f.type === 'IMAGE',
  )
  if (!paint?.imageScaleMode) return undefined
  switch (paint.imageScaleMode) {
    case 'FIT': return 'fit'
    case 'FILL': return 'fill'
    case 'TILE': return 'tile'
    default: return undefined
  }
}

function figmaFillColor(figma: FigmaNodeChange): string | undefined {
  const paint = figma.fillPaints?.find((f) => f.visible !== false && f.type === 'SOLID')
  if (!paint?.color) return undefined
  const { r: cr, g: cg, b: cb } = paint.color
  const toHex = (v: number) => Math.round(v * 255).toString(16).padStart(2, '0')
  return `#${toHex(cr)}${toHex(cg)}${toHex(cb)}`
}

export function collectImageBlobs(blobs: (Uint8Array | string)[]): Map<number, Uint8Array> {
  const map = new Map<number, Uint8Array>()
  for (let i = 0; i < blobs.length; i++) {
    const blob = blobs[i]
    if (blob instanceof Uint8Array && blob.length > 0) {
      if (blob[0] === 0x89 && blob[1] === 0x50) {
        map.set(i, blob)
      }
    }
  }
  return map
}

// --- Children conversion ---

export function convertChildren(
  parent: TreeNode,
  ctx: ConversionContext,
): PenNode[] {
  const parentStackMode = ctx.layoutMode === 'preserve' ? undefined : parent.figma.stackMode
  const result: PenNode[] = []

  for (const child of parent.children) {
    if (child.figma.visible === false) continue
    const node = convertNode(child, parentStackMode, ctx)
    if (node) result.push(node)
  }

  return result
}

// --- Node conversion dispatcher ---

export function convertNode(
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

// --- Individual node converters ---

function convertFrame(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const id = ctx.generateId()
  const children = convertChildren(treeNode, ctx)

  if (hasOnlyImageFill(figma) && children.length === 0) {
    return {
      type: 'image',
      ...commonProps(figma, id),
      src: getImageFillUrl(figma),
      objectFit: getImageFitMode(figma),
      width: resolveWidth(figma, parentStackMode, ctx),
      height: resolveHeight(figma, parentStackMode, ctx),
      cornerRadius: mapCornerRadius(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  const layout = ctx.layoutMode === 'preserve'
    ? ((figma.frameMaskDisabled !== true || figma.stackMode) ? { clipContent: true } : {})
    : mapFigmaLayout(figma)

  return {
    type: 'frame',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
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
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
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
  const children = convertChildren(treeNode, ctx)

  const layout = ctx.layoutMode === 'preserve'
    ? ((figma.frameMaskDisabled !== true || figma.stackMode) ? { clipContent: true } : {})
    : mapFigmaLayout(figma)

  return {
    type: 'frame',
    ...commonProps(figma, id),
    reusable: true,
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
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

  if (!componentPenId) {
    // Instance's own tree node may have 0 children (Figma instances inherit from master).
    // Try to inline the master SYMBOL's children so the visual content is preserved.
    if (componentGuid && treeNode.children.length === 0) {
      const symbolNode = ctx.symbolTree.get(guidToString(componentGuid))
      if (symbolNode && symbolNode.children.length > 0) {
        const children = applyInstanceOverrides(
          symbolNode,
          figma.symbolData?.symbolOverrides,
          figma.derivedSymbolData,
          figma.size,
        )
        return convertFrame(
          { figma: treeNode.figma, children },
          parentStackMode,
          ctx,
        )
      }
    }
    return convertFrame(treeNode, parentStackMode, ctx)
  }

  const id = ctx.generateId()
  return {
    type: 'ref',
    ...commonProps(figma, id),
    ref: componentPenId,
  }
}

/**
 * Apply INSTANCE overrides (fills, arcData) and derived data (sizes, transforms)
 * to SYMBOL children when inlining them into an instance.
 */
function applyInstanceOverrides(
  symbolNode: TreeNode,
  overrides: FigmaSymbolOverride[] | undefined,
  derived: FigmaDerivedSymbolDataEntry[] | undefined,
  instanceSize: { x: number; y: number } | undefined,
): TreeNode[] {
  // If no derived data, fall back to simple scaling
  if (!derived || derived.length === 0) {
    if (instanceSize && symbolNode.figma.size) {
      const sx = instanceSize.x / symbolNode.figma.size.x
      const sy = instanceSize.y / symbolNode.figma.size.y
      return scaleTreeChildren(symbolNode.children, sx, sy)
    }
    return symbolNode.children
  }

  // Build override map keyed by guidPath string
  const overrideMap = new Map<string, FigmaSymbolOverride>()
  if (overrides) {
    for (const ov of overrides) {
      if (ov.guidPath?.guids?.length) {
        overrideMap.set(guidPathKey(ov.guidPath.guids), ov)
      }
    }
  }

  // Build derived map keyed by guidPath string
  const derivedMap = new Map<string, FigmaDerivedSymbolDataEntry>()
  for (const d of derived) {
    if (d.guidPath?.guids?.length) {
      derivedMap.set(guidPathKey(d.guidPath.guids), d)
    }
  }

  // Flatten SYMBOL tree in pre-order DFS with children sorted by ascending GUID localID.
  // derivedSymbolData entries follow creation order (ascending GUID), not the tree's
  // z-order (descending position), so we must match that order.
  const flatSymbol: TreeNode[] = []
  function flattenDFS(node: TreeNode) {
    flatSymbol.push(node)
    const sorted = [...node.children].sort((a, b) => {
      const aId = a.figma.guid?.localID ?? 0
      const bId = b.figma.guid?.localID ?? 0
      return aId - bId
    })
    for (const c of sorted) flattenDFS(c)
  }
  flattenDFS(symbolNode)

  // Map each SYMBOL node's GUID → guidPath key (from derived data, matched by index)
  const nodeGuidToPathKey = new Map<string, string>()
  for (let i = 0; i < Math.min(flatSymbol.length, derived.length); i++) {
    const node = flatSymbol[i]
    const d = derived[i]
    if (node.figma.guid && d.guidPath?.guids?.length) {
      nodeGuidToPathKey.set(
        guidToString(node.figma.guid),
        guidPathKey(d.guidPath.guids),
      )
    }
  }

  // Recursively apply overrides and derived data to each node
  function applyToNode(node: TreeNode): TreeNode {
    const nodeKey = node.figma.guid ? guidToString(node.figma.guid) : ''
    const pathKey = nodeGuidToPathKey.get(nodeKey)
    if (!pathKey) {
      return { figma: { ...node.figma }, children: node.children.map(applyToNode) }
    }

    const figma = { ...node.figma }

    // Apply derived data (pre-computed sizes and transforms for this instance)
    const d = derivedMap.get(pathKey)
    if (d) {
      if (d.size) figma.size = d.size
      if (d.transform) figma.transform = d.transform
      if (d.fontSize !== undefined) figma.fontSize = d.fontSize
      if (d.derivedTextData) figma.textData = d.derivedTextData
    }

    // Apply overrides (fills, arcData, text props customized by this instance)
    const ov = overrideMap.get(pathKey)
    if (ov) {
      if (ov.fillPaints) figma.fillPaints = ov.fillPaints
      if (ov.arcData) figma.arcData = ov.arcData
      if (ov.textData) figma.textData = ov.textData
      if (ov.fontSize !== undefined) figma.fontSize = ov.fontSize
      if (ov.fontName) figma.fontName = ov.fontName
      if (ov.lineHeight) figma.lineHeight = ov.lineHeight
      if (ov.letterSpacing) figma.letterSpacing = ov.letterSpacing
    }

    return { figma, children: node.children.map(applyToNode) }
  }

  return symbolNode.children.map(applyToNode)
}

function guidPathKey(guids: FigmaGUID[]): string {
  return guids.map((g) => guidToString(g)).join('/')
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
      objectFit: getImageFitMode(figma),
      width: resolveWidth(figma, parentStackMode, ctx),
      height: resolveHeight(figma, parentStackMode, ctx),
      cornerRadius: mapCornerRadius(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  return {
    type: 'rectangle',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
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

  if (hasOnlyImageFill(figma)) {
    return {
      type: 'image',
      ...commonProps(figma, id),
      src: getImageFillUrl(figma),
      objectFit: getImageFitMode(figma),
      width: resolveWidth(figma, parentStackMode, ctx),
      height: resolveHeight(figma, parentStackMode, ctx),
      cornerRadius: Math.round((figma.size?.x ?? 100) / 2),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  // Convert Figma arcData (radians) to PenNode arc properties (degrees)
  const arc = figma.arcData
  const arcProps = arc ? mapFigmaArcData(arc) : {}
  const props = commonProps(figma, id)

  // For arc ellipses, absorb flipX/flipY into the arc angles instead of
  // relying on canvas-level flip (SVG path flip doesn't work well in Fabric.js).
  // Also fix the position: when m00=-1 the x in transform is the right edge.
  if (arcProps.sweepAngle !== undefined || arcProps.startAngle !== undefined || arcProps.innerRadius !== undefined) {
    const start = arcProps.startAngle ?? 0
    const sweep = arcProps.sweepAngle ?? 360
    if (props.flipX) {
      arcProps.startAngle = normalizeAngle(180 - start - sweep)
      arcProps.sweepAngle = sweep
      const w = figma.size?.x ?? 0
      props.x = Math.round((props.x - w) * 100) / 100
      delete props.flipX
    }
    if (props.flipY) {
      arcProps.startAngle = normalizeAngle(360 - start - sweep)
      arcProps.sweepAngle = sweep
      const h = figma.size?.y ?? 0
      props.y = Math.round((props.y - h) * 100) / 100
      delete props.flipY
    }
  }

  return {
    type: 'ellipse',
    ...props,
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    ...arcProps,
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  }
}

/** Convert Figma arcData (radians, endAngle) to PenNode arc props (degrees, sweepAngle). */
function mapFigmaArcData(arc: { startingAngle?: number; endingAngle?: number; innerRadius?: number }): {
  startAngle?: number
  sweepAngle?: number
  innerRadius?: number
} {
  const startRad = arc.startingAngle ?? 0
  const endRad = arc.endingAngle ?? Math.PI * 2
  const inner = arc.innerRadius ?? 0

  let sweepRad = endRad - startRad
  while (sweepRad < 0) sweepRad += Math.PI * 2

  const startDeg = (startRad * 180) / Math.PI
  const sweepDeg = (sweepRad * 180) / Math.PI

  // Only emit props that differ from the full-circle defaults
  const result: { startAngle?: number; sweepAngle?: number; innerRadius?: number } = {}
  if (Math.abs(startDeg) > 0.1) result.startAngle = Math.round(startDeg * 100) / 100
  if (Math.abs(sweepDeg - 360) > 0.1) result.sweepAngle = Math.round(sweepDeg * 100) / 100
  if (inner > 0.001) result.innerRadius = Math.round(inner * 1000) / 1000
  return result
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

  const iconMatch = lookupIconByName(name)
  if (iconMatch) {
    return {
      type: 'path',
      ...commonProps(figma, id),
      d: iconMatch.d,
      iconId: iconMatch.iconId,
      width: resolveWidth(figma, parentStackMode, ctx),
      height: resolveHeight(figma, parentStackMode, ctx),
      fill: iconMatch.style === 'fill' ? mapFigmaFills(figma.fillPaints) : undefined,
      stroke: iconMatch.style === 'stroke'
        ? mapFigmaStroke(figma) ?? { thickness: 2, fill: [{ type: 'solid', color: figmaFillColor(figma) ?? '#000000' }] }
        : mapFigmaStroke(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  const pathD = decodeFigmaVectorPath(figma, ctx.blobs)
  if (pathD) {
    return {
      type: 'path',
      ...commonProps(figma, id),
      d: pathD,
      width: resolveWidth(figma, parentStackMode, ctx),
      height: resolveHeight(figma, parentStackMode, ctx),
      fill: mapFigmaFills(figma.fillPaints),
      stroke: mapFigmaStroke(figma),
      effects: mapFigmaEffects(figma.effects),
    }
  }

  ctx.warnings.push(
    `Vector node "${figma.name}" converted as rectangle (path data not decodable)`
  )
  return {
    type: 'rectangle',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
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
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    ...textProps,
    fill: mapFigmaFills(figma.fillPaints),
    effects: mapFigmaEffects(figma.effects),
  }
}
