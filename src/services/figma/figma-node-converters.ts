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
import { decodeFigmaVectorPath, computeSvgPathBounds } from './figma-vector-decoder'
import { lookupIconByName } from '@/services/ai/icon-resolver'
import type { TreeNode } from './figma-tree-builder'
import { guidToString } from './figma-tree-builder'

/** Scale tree children's transforms and sizes to fit a different parent size.
 *  Also scales strokeWeight proportionally so strokes don't appear
 *  disproportionately thick when an instance is smaller than its symbol. */
function scaleTreeChildren(children: TreeNode[], sx: number, sy: number): TreeNode[] {
  if (Math.abs(sx - 1) < 0.001 && Math.abs(sy - 1) < 0.001) return children
  const strokeScale = Math.min(sx, sy)
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
    // Scale stroke weight so lines stay visually proportional
    if (figma.strokeWeight !== undefined && strokeScale < 0.99) {
      figma.strokeWeight = Math.round(figma.strokeWeight * strokeScale * 100) / 100
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
  if (!figma.transform) return { x: 0, y: 0 }

  const m = figma.transform

  // Detect rotation or flip: any non-identity 2×2 sub-matrix means
  // m02/m12 is NOT the top-left corner of the bounding box.
  const hasRotation = Math.abs(m.m01) > 0.001 || Math.abs(m.m10) > 0.001
  const hasFlip = m.m00 < -0.001 || m.m11 < -0.001

  if ((hasRotation || hasFlip) && figma.size) {
    // Figma's m02/m12 gives where local origin (0,0) maps in parent space.
    // For rotated/flipped nodes this differs from the pre-transform top-left
    // that OpenPencil needs.  Compute the object center (invariant under
    // rotation/flip) and derive the pre-transform top-left from it.
    const w = figma.size.x
    const h = figma.size.y
    const cx = m.m00 * w / 2 + m.m01 * h / 2 + m.m02
    const cy = m.m10 * w / 2 + m.m11 * h / 2 + m.m12
    return {
      x: Math.round((cx - w / 2) * 100) / 100,
      y: Math.round((cy - h / 2) * 100) / 100,
    }
  }

  return {
    x: Math.round(m.m02 * 100) / 100,
    y: Math.round(m.m12 * 100) / 100,
  }
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
    if (blob instanceof Uint8Array && blob.length > 8) {
      // Detect image magic bytes: PNG, JPEG, GIF, WebP
      const isPng = blob[0] === 0x89 && blob[1] === 0x50
      const isJpeg = blob[0] === 0xFF && blob[1] === 0xD8
      const isGif = blob[0] === 0x47 && blob[1] === 0x49
      const isWebp = blob[0] === 0x52 && blob[1] === 0x49
      if (isPng || isJpeg || isGif || isWebp) {
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
    // Skip fully transparent nodes — their children are also invisible and
    // the Skia renderer does not propagate parent opacity to descendants.
    if (child.figma.opacity !== undefined && child.figma.opacity <= 0) continue
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

  // In preserve mode, only apply auto-layout properties for frames that actually
  // have stackMode set.  Frames without stackMode use absolute x,y positioning.
  // For auto-layout frames, children order must be reversed because the tree
  // builder sorts descending (for z-stacking) but layout needs ascending (flow order).
  const hasAutoLayout = figma.stackMode && figma.stackMode !== 'NONE'
  const layout = ctx.layoutMode === 'preserve'
    ? (hasAutoLayout ? mapFigmaLayout(figma) : (figma.frameMaskDisabled !== true ? { clipContent: true } : {}))
    : mapFigmaLayout(figma)

  // Reverse children order for auto-layout frames in preserve mode:
  // tree builder sorts descending by position (z-stacking), but auto-layout
  // needs ascending order (first child = start of layout flow).
  const orderedChildren = (hasAutoLayout && ctx.layoutMode === 'preserve' && children.length > 1)
    ? [...children].reverse()
    : children

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
    children: orderedChildren.length > 0 ? orderedChildren : undefined,
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

  const hasAutoLayout = figma.stackMode && figma.stackMode !== 'NONE'
  const layout = ctx.layoutMode === 'preserve'
    ? (hasAutoLayout ? mapFigmaLayout(figma) : (figma.frameMaskDisabled !== true ? { clipContent: true } : {}))
    : mapFigmaLayout(figma)

  const orderedChildren = (hasAutoLayout && ctx.layoutMode === 'preserve' && children.length > 1)
    ? [...children].reverse()
    : children

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
    children: orderedChildren.length > 0 ? orderedChildren : undefined,
  }
}

function convertInstance(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma
  const componentGuid = figma.overriddenSymbolID ?? figma.symbolData?.symbolID

  // Check if this instance has visual overrides (fills, strokes, arcData, text) that must be inlined
  const hasVisualOverrides = figma.symbolData?.symbolOverrides?.some(
    (ov: any) => ov.fillPaints?.length > 0 || ov.strokePaints?.length > 0 || ov.arcData || ov.textData || ov.fontSize !== undefined,
  ) ?? false

  // Try to inline the master SYMBOL's children with overrides applied.
  // Always inline when the instance has no local children (meaning it needs
  // the symbol's content to render), or when it has visual overrides.
  if (componentGuid && (treeNode.children.length === 0 || hasVisualOverrides)) {
    const symbolNode = ctx.symbolTree.get(guidToString(componentGuid))
    if (symbolNode && symbolNode.children.length > 0) {
      const children = applyInstanceOverrides(
        symbolNode,
        figma.symbolData?.symbolOverrides,
        figma.derivedSymbolData,
        figma.size,
        ctx.symbolTree,
      )
      return convertFrame(
        { figma: treeNode.figma, children },
        parentStackMode,
        ctx,
      )
    }
  }

  const componentPenId = componentGuid
    ? ctx.componentMap.get(guidToString(componentGuid))
    : undefined

  if (componentPenId) {
    const id = ctx.generateId()
    return {
      type: 'ref',
      ...commonProps(figma, id),
      ref: componentPenId,
    }
  }

  return convertFrame(treeNode, parentStackMode, ctx)
}

/**
 * Apply INSTANCE overrides (fills, arcData) and derived data (sizes, transforms)
 * to SYMBOL children when inlining them into an instance.
 *
 * Figma's derivedSymbolData entries use virtual GUIDs that don't exist in the
 * document's nodeChanges.  When the SYMBOL contains no nested INSTANCE nodes,
 * the entries correspond 1:1 (by index) to a pre-order DFS of the SYMBOL tree
 * sorted by ascending localID.  When nested INSTANCEs are present, the derived
 * data also includes entries for expanded children of those nested instances,
 * making derived.length > flatSymbol.length.
 *
 * Strategy:
 * 1. Filter derived to length-1 guidPaths (direct symbol nodes, excluding nested).
 * 2. If filtered count == flatSymbol count → direct index matching (proven correct).
 * 3. Otherwise → expanded DFS sequential block: walk the SYMBOL tree in localID
 *    order, recursively expanding INSTANCE children, and assign sequential slot
 *    IDs starting from the first derived entry's localID.  Each symbol node's
 *    slot ID is used as its guidPath key for derived/override lookups.
 */
function applyInstanceOverrides(
  symbolNode: TreeNode,
  overrides: FigmaSymbolOverride[] | undefined,
  derived: FigmaDerivedSymbolDataEntry[] | undefined,
  instanceSize: { x: number; y: number } | undefined,
  _symbolTree: Map<string, TreeNode>,
): TreeNode[] {
  // If no derived data and no overrides, fall back to simple scaling
  if ((!derived || derived.length === 0) && (!overrides || overrides.length === 0)) {
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
  const safeDerived = derived ?? []
  for (const d of safeDerived) {
    if (d.guidPath?.guids?.length) {
      derivedMap.set(guidPathKey(d.guidPath.guids), d)
    }
  }

  // Flatten SYMBOL tree in pre-order DFS with children sorted by ascending GUID localID
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

  // Filter derived to length-1 guidPaths only (excludes nested instance entries)
  const len1Derived = safeDerived.filter(d => d.guidPath?.guids?.length === 1)

  // Extract base session/localID from the first derived entry
  const firstGuids = len1Derived[0]?.guidPath?.guids
  const sessionID = firstGuids?.[0]?.sessionID
  const firstLocalID = firstGuids?.[0]?.localID

  // Resolve overrides and derived data to actual symbol tree nodes.
  // nodeOverride/nodeDerived are keyed by node GUID string.
  const nodeOverride = new Map<string, FigmaSymbolOverride>()
  const nodeDerived = new Map<string, FigmaDerivedSymbolDataEntry>()
  // Virtual pathKey → actual node GUID map, populated by the chosen strategy.
  // Used to resolve first GUIDs in multi-guid paths for nested instance propagation.
  const pkToNodeGuid = new Map<string, string>()

  /** Resolve a pathKey's override/derived entries to a target node GUID. */
  function resolveToNode(pathKey: string, nodeGuid: string) {
    const d = derivedMap.get(pathKey)
    if (d) nodeDerived.set(nodeGuid, d)
    const ov = overrideMap.get(pathKey)
    if (ov) nodeOverride.set(nodeGuid, ov)
  }

  // Build GUID→nodeGuid map for direct lookup
  const guidToNodeMap = new Map<string, string>()
  for (const node of flatSymbol) {
    if (node.figma.guid) guidToNodeMap.set(guidToString(node.figma.guid), guidToString(node.figma.guid))
  }

  // Strategy 0: Direct GUID matching — when derived guidPath GUIDs are actual
  // symbol node GUIDs (not virtual).  Check if most derived entries match.
  let directMatches = 0
  for (const d of len1Derived) {
    const pk = d.guidPath?.guids?.[0]
    if (pk && guidToNodeMap.has(guidToString(pk))) directMatches++
  }

  if (directMatches > len1Derived.length * 0.5 || len1Derived.length === 0) {
    // Most derived entries have actual GUIDs — use direct matching.
    // Also handles the override-only case (derived=0, overrides>0) where
    // override GUIDs are actual node GUIDs (e.g. icon instances that only
    // override stroke colors without changing sizes).
    for (const d of len1Derived) {
      const pk = d.guidPath?.guids?.[0]
      if (!pk) continue
      const pkStr = guidToString(pk)
      if (guidToNodeMap.has(pkStr)) {
        resolveToNode(pkStr, pkStr)
        pkToNodeGuid.set(pkStr, pkStr)
      }
    }
    // Also resolve overrides that use actual GUIDs
    for (const [pk] of overrideMap) {
      if (pk.includes('/')) continue
      if (guidToNodeMap.has(pk)) {
        const ov = overrideMap.get(pk)
        if (ov) nodeOverride.set(pk, ov)
      }
    }
  } else if (len1Derived.length === flatSymbol.length) {
    // Strategy 1: exact count match — index mapping (for virtual GUIDs)
    for (let i = 0; i < flatSymbol.length; i++) {
      const node = flatSymbol[i]
      const d = len1Derived[i]
      if (node.figma.guid && d.guidPath?.guids?.length) {
        const actualGuid = guidToString(node.figma.guid)
        resolveToNode(guidPathKey(d.guidPath.guids), actualGuid)
        pkToNodeGuid.set(guidToString(d.guidPath.guids[0]), actualGuid)
      }
    }
  } else if (firstLocalID !== undefined && sessionID !== undefined) {
    // Strategy 2: full DFS + expanded DFS.
    // Figma assigns sequential virtual localIDs to all nodes (including
    // INSTANCE nodes) in a pre-order DFS starting from the symbol's children
    // (excluding the symbol root itself).  The expanded DFS handles overflow
    // entries at higher localIDs caused by inline expansion of nested instances.

    const childSorted = [...symbolNode.children].sort((a, b) =>
      (a.figma.guid?.localID ?? 0) - (b.figma.guid?.localID ?? 0),
    )

    // --- Full DFS from children: pathKey → nodeGuid ---
    // Includes ALL node types (including INSTANCE).  Starts from the symbol's
    // children, NOT the root, because derived data doesn't include the root.
    const fullPkToNode = new Map<string, string>()
    let fullIdx = 0
    function walkFull(node: TreeNode) {
      if (node.figma.guid) {
        fullPkToNode.set(
          `${sessionID}:${firstLocalID! + fullIdx}`,
          guidToString(node.figma.guid),
        )
      }
      fullIdx++
      const sorted = [...node.children].sort((a, b) =>
        (a.figma.guid?.localID ?? 0) - (b.figma.guid?.localID ?? 0),
      )
      for (const c of sorted) walkFull(c)
    }
    for (const c of childSorted) walkFull(c)
    // Build root-inclusive DFS to detect overrides targeting the symbol root.
    // Root-targeted overrides belong to the instance frame (handled by caller)
    // and must be skipped when applying to children.
    const rootGuid = symbolNode.figma.guid ? guidToString(symbolNode.figma.guid) : ''
    const rootPkToNode = new Map<string, string>()
    let rootIdx = 0
    function walkRoot(node: TreeNode) {
      if (node.figma.guid) {
        rootPkToNode.set(
          `${sessionID}:${firstLocalID! + rootIdx}`,
          guidToString(node.figma.guid),
        )
      }
      rootIdx++
      const sorted = [...node.children].sort((a, b) =>
        (a.figma.guid?.localID ?? 0) - (b.figma.guid?.localID ?? 0),
      )
      for (const c of sorted) walkRoot(c)
    }
    walkRoot(symbolNode)

    // Populate pkToNodeGuid from the full DFS mapping for nested resolution
    for (const [pk, ng] of fullPkToNode) {
      pkToNodeGuid.set(pk, ng)
    }

    // Resolve derived data via children-only DFS
    for (const [pk, d] of derivedMap) {
      if (pk.includes('/')) continue // multi-guid → nested instance, skip
      const ng = fullPkToNode.get(pk)
      if (ng) nodeDerived.set(ng, d)
    }
    // Resolve overrides via children-only DFS, but skip overrides that
    // target the symbol root in root-inclusive DFS.  Nested INSTANCE nodes
    // inside SYMBOLs carry pre-computed overrides where slot 0 = root;
    // applying those to the first child (slot 0 in children-only) is wrong.
    for (const [pk, ov] of overrideMap) {
      if (pk.includes('/')) continue
      // Check if this override targets the root in root-inclusive numbering
      if (rootPkToNode.get(pk) === rootGuid) continue
      const ng = fullPkToNode.get(pk)
      if (ng) nodeOverride.set(ng, ov)
    }
  } else {
    // Fallback: direct index mapping with all derived
    for (let i = 0; i < Math.min(flatSymbol.length, safeDerived.length); i++) {
      const node = flatSymbol[i]
      const d = safeDerived[i]
      if (node.figma.guid && d.guidPath?.guids?.length) {
        const actualGuid = guidToString(node.figma.guid)
        resolveToNode(guidPathKey(d.guidPath.guids), actualGuid)
        if (d.guidPath.guids.length === 1) {
          pkToNodeGuid.set(guidToString(d.guidPath.guids[0]), actualGuid)
        }
      }
    }
  }

  // Build nested maps for multi-guid entries (nested instance overrides + derived data).
  // Use pkToNodeGuid to resolve virtual first GUIDs to actual node GUIDs so that
  // applyToNode (which uses actual GUIDs) can look them up correctly.
  const nestedOverrideMap = new Map<string, FigmaSymbolOverride[]>()
  const nestedDerivedMap = new Map<string, FigmaDerivedSymbolDataEntry[]>()

  for (const [pk, ov] of overrideMap) {
    if (!pk.includes('/')) continue
    const parts = pk.split('/')
    const instanceGuid = pkToNodeGuid.get(parts[0]) ?? parts[0]
    const childGuids = ov.guidPath?.guids?.slice(1)
    if (childGuids?.length) {
      const childOv = { ...ov, guidPath: { guids: childGuids } }
      const existing = nestedOverrideMap.get(instanceGuid) ?? []
      existing.push(childOv)
      nestedOverrideMap.set(instanceGuid, existing)
    }
  }

  for (const [pk, d] of derivedMap) {
    if (!pk.includes('/')) continue
    const parts = pk.split('/')
    const instanceGuid = pkToNodeGuid.get(parts[0]) ?? parts[0]
    const childGuids = d.guidPath?.guids?.slice(1)
    if (childGuids?.length) {
      const childD = { ...d, guidPath: { guids: childGuids } }
      const existing = nestedDerivedMap.get(instanceGuid) ?? []
      existing.push(childD)
      nestedDerivedMap.set(instanceGuid, existing)
    }
  }

  // Recursively apply resolved overrides and derived data to each node
  function applyToNode(node: TreeNode): TreeNode {
    const nodeKey = node.figma.guid ? guidToString(node.figma.guid) : ''
    const d = nodeDerived.get(nodeKey)
    const ov = nodeOverride.get(nodeKey)
    const nestedOvs = nestedOverrideMap.get(nodeKey)
    const nestedDer = nestedDerivedMap.get(nodeKey)

    if (!d && !ov && !nestedOvs && !nestedDer) {
      return { figma: { ...node.figma }, children: node.children.map(applyToNode) }
    }

    const figma = { ...node.figma }

    // Apply derived data (pre-computed sizes and transforms for this instance)
    if (d) {
      // Scale strokeWeight proportionally when derived size is smaller than
      // the symbol's original size, so strokes don't appear too thick.
      if (d.size && node.figma.size && figma.strokeWeight !== undefined) {
        const sx = d.size.x / node.figma.size.x
        const sy = d.size.y / node.figma.size.y
        const strokeScale = Math.min(sx, sy)
        if (strokeScale < 0.99) {
          figma.strokeWeight = Math.round(figma.strokeWeight * strokeScale * 100) / 100
        }
      }
      if (d.size) figma.size = d.size
      if (d.transform) figma.transform = d.transform
      if (d.fontSize !== undefined) figma.fontSize = d.fontSize
      if (d.derivedTextData?.characters !== undefined) figma.textData = d.derivedTextData
    }

    // Apply all override properties from the symbolOverride entry
    if (ov) {
      const skipKeys = new Set([
        'guidPath', 'guid', 'parentIndex', 'type', 'phase',
        'symbolData', 'derivedSymbolData', 'componentKey',
        'variableConsumptionMap', 'parameterConsumptionMap',
        'prototypeInteractions', 'styleIdForFill', 'styleIdForStrokeFill',
        'styleIdForText', 'overrideLevel', 'componentPropAssignments',
        'proportionsConstrained', 'fontVersion',
      ])
      for (const key of Object.keys(ov)) {
        if (skipKeys.has(key)) continue
        const value = (ov as Record<string, unknown>)[key]
        if (value !== undefined) {
          ;(figma as Record<string, unknown>)[key] = value
        }
      }
    }

    // Propagate multi-guid overrides and derived data to nested INSTANCE nodes.
    // When the outer instance has entries like "instanceGuid/childGuid/...",
    // inject the child-scoped data into the nested instance's symbolData
    // and derivedSymbolData so convertInstance can apply them recursively.
    if ((nestedOvs || nestedDer) && (figma.type === 'INSTANCE' || figma.symbolData)) {
      if (nestedOvs) {
        const existingOverrides = figma.symbolData?.symbolOverrides ?? []
        figma.symbolData = {
          ...figma.symbolData,
          symbolOverrides: [...existingOverrides, ...nestedOvs],
        }
      }
      if (nestedDer) {
        // Replace (not merge) existing derived data with nested data from the
        // outer instance.  INSTANCE nodes inside SYMBOLs carry pre-computed
        // derivedSymbolData, but the outer instance's nested entries supersede
        // them.  Merging would inflate the entry count and cause Strategy 1
        // (index matching) to trigger incorrectly.
        figma.derivedSymbolData = nestedDer
      }
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
    // Only adjust position for flip when there's no rotation component.
    // When rotation is present, extractPosition already computed the correct
    // center-based position that accounts for both rotation and flip.
    const hasRot = figma.transform && (Math.abs(figma.transform.m01) > 0.001 || Math.abs(figma.transform.m10) > 0.001)
    if (props.flipX) {
      arcProps.startAngle = normalizeAngle(180 - start - sweep)
      arcProps.sweepAngle = sweep
      if (!hasRot) {
        const w = figma.size?.x ?? 0
        props.x = Math.round((props.x - w) * 100) / 100
      }
      delete props.flipX
    }
    if (props.flipY) {
      arcProps.startAngle = normalizeAngle(360 - start - sweep)
      arcProps.sweepAngle = sweep
      if (!hasRot) {
        const h = figma.size?.y ?? 0
        props.y = Math.round((props.y - h) * 100) / 100
      }
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

/** Convert Figma arcData (radians, endAngle) to PenNode arc props (degrees, sweepAngle).
 *
 * When endingAngle < startingAngle, Figma draws the arc counter-clockwise from
 * startingAngle to endingAngle.  The equivalent clockwise arc goes from
 * endingAngle to startingAngle, so we swap start/end and use their difference
 * as the sweep.  This is critical for donut chart segments where overlapping
 * arcs with z-order create the visual pie slices.
 */
function mapFigmaArcData(arc: { startingAngle?: number; endingAngle?: number; innerRadius?: number }): {
  startAngle?: number
  sweepAngle?: number
  innerRadius?: number
} {
  const startRad = arc.startingAngle ?? 0
  const endRad = arc.endingAngle ?? Math.PI * 2
  const inner = arc.innerRadius ?? 0

  let actualStartRad: number
  let sweepRad: number

  if (endRad >= startRad) {
    // Normal case: clockwise from start to end
    actualStartRad = startRad
    sweepRad = endRad - startRad
  } else {
    // Inverted case: endAngle < startAngle means counter-clockwise from start
    // to end.  Convert to equivalent clockwise arc: start at endAngle, sweep
    // forward to startAngle.
    actualStartRad = endRad
    sweepRad = startRad - endRad
  }

  const startDeg = (actualStartRad * 180) / Math.PI
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
    const iconW = resolveWidth(figma, parentStackMode, ctx)
    const iconH = resolveHeight(figma, parentStackMode, ctx)

    // Lucide/Feather icon paths use a 24×24 viewbox.  When the icon is
    // rendered smaller, scale strokeWeight proportionally so lines don't
    // appear disproportionately thick.
    const iconSize = Math.min(
      typeof iconW === 'number' ? iconW : 24,
      typeof iconH === 'number' ? iconH : 24,
    )
    const iconScale = iconSize / 24

    let stroke = iconMatch.style === 'stroke'
      ? mapFigmaStroke(figma) ?? { thickness: 1.5, fill: [{ type: 'solid', color: figmaFillColor(figma) ?? '#000000' }], cap: 'round' as const, join: 'round' as const }
      : mapFigmaStroke(figma)

    if (stroke && iconScale < 0.99) {
      const rawThickness = typeof stroke.thickness === 'number' ? stroke.thickness : 1.5
      stroke = { ...stroke, thickness: Math.round(rawThickness * iconScale * 100) / 100 }
    }

    return {
      type: 'path',
      ...commonProps(figma, id),
      d: iconMatch.d,
      iconId: iconMatch.iconId,
      width: iconW,
      height: iconH,
      fill: iconMatch.style === 'fill' ? mapFigmaFills(figma.fillPaints) : undefined,
      stroke,
      effects: mapFigmaEffects(figma.effects),
    }
  }

  const pathD = decodeFigmaVectorPath(figma, ctx.blobs)
  if (pathD) {
    const props = commonProps(figma, id)
    let width: SizingBehavior = resolveWidth(figma, parentStackMode, ctx)
    let height: SizingBehavior = resolveHeight(figma, parentStackMode, ctx)

    // When Figma reports zero width or height for a vector node (common for
    // chevron/arrow icons centered around the origin), derive the actual
    // visual extent from the decoded path bounds and adjust position.
    const sizeX = figma.size?.x ?? 0
    const sizeY = figma.size?.y ?? 0
    if ((sizeX < 0.01 || sizeY < 0.01) && typeof width === 'number' && typeof height === 'number') {
      const bounds = computeSvgPathBounds(pathD)
      if (bounds) {
        const pathW = bounds.maxX - bounds.minX
        const pathH = bounds.maxY - bounds.minY
        if (sizeX < 0.01 && pathW > 0.01) {
          width = Math.round(pathW * 100) / 100
          props.x = Math.round((props.x + bounds.minX) * 100) / 100
        }
        if (sizeY < 0.01 && pathH > 0.01) {
          height = Math.round(pathH * 100) / 100
          props.y = Math.round((props.y + bounds.minY) * 100) / 100
        }
      }
    }

    // Figma's strokeGeometry is the EXPANDED stroke outline (not a centerline).
    // For stroke-only vectors, we must FILL this outline with the stroke color
    // instead of drawing another stroke on top (which would double the thickness).
    const hasVisibleFills = figma.fillPaints?.some((p: any) => p.visible !== false)
    const hasVisibleStrokes = figma.strokePaints?.some((p: any) => p.visible !== false)
    const isStrokeOnlyOutline = !hasVisibleFills && hasVisibleStrokes
      && !figma.fillGeometry?.length && figma.strokeGeometry?.length

    if (isStrokeOnlyOutline) {
      // Convert stroke paint to fill — the path IS the visual stroke
      const strokeAsFill = mapFigmaFills(figma.strokePaints!)
      return {
        type: 'path',
        ...props,
        d: pathD,
        width,
        height,
        fill: strokeAsFill,
        // No stroke — the outline shape already represents the stroke visual
        effects: mapFigmaEffects(figma.effects),
      }
    }

    return {
      type: 'path',
      ...props,
      d: pathD,
      width,
      height,
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
  const width = resolveWidth(figma, parentStackMode, ctx)

  // Reconcile textGrowth with the resolved width:
  // 1. Layout sizing string (fill_container, fit_content) — container dictates width,
  //    so text must use fixed-width mode (Textbox) for wrapping.
  // 2. textAutoResize missing (undefined in .fig binary) — Figma defaults to fixed
  //    dimensions; treat as fixed-width so text wraps at the stored width.
  //    Note: this does NOT affect WIDTH_AND_HEIGHT nodes (textGrowth: 'auto'),
  //    which correctly remain as IText (auto-width, no wrapping).
  if (textProps.textGrowth === undefined) {
    if (typeof width === 'string' || !figma.textAutoResize) {
      textProps.textGrowth = 'fixed-width'
    }
  } else if (textProps.textGrowth === 'auto' && typeof width === 'string') {
    textProps.textGrowth = 'fixed-width'
  }

  return {
    type: 'text',
    ...commonProps(figma, id),
    width,
    height: resolveHeight(figma, parentStackMode, ctx),
    ...textProps,
    fill: mapFigmaFills(figma.fillPaints),
    effects: mapFigmaEffects(figma.effects),
  }
}
