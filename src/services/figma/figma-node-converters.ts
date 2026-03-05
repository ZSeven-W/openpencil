import type { FigmaNodeChange, FigmaMatrix, FigmaImportLayoutMode } from './figma-types'
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

const SKIPPED_TYPES = new Set([
  'SLICE', 'CONNECTOR', 'SHAPE_WITH_TEXT', 'STICKY', 'STAMP',
  'HIGHLIGHT', 'WASHI_TAPE', 'CODE_BLOCK', 'MEDIA', 'WIDGET',
  'SECTION_OVERLAY', 'NONE',
])

export interface ConversionContext {
  componentMap: Map<string, string>
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

  return {
    type: 'ellipse',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
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
