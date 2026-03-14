import { useEffect } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, findNodeInTree, getActivePageChildren, setActivePageChildren, getAllChildren } from '@/stores/document-store'
import type { PenNode, ContainerProps } from '@/types/pen'
import {
  createFabricObject,
  type FabricObjectWithPenId,
  isDirectionalStroke,
  getDirectionalStrokeThicknesses,
  resolveStrokeColor,
} from './canvas-object-factory'
import { syncFabricObject } from './canvas-object-sync'
import { isFabricSyncLocked, setFabricSyncLock } from './canvas-sync-lock'
import { pendingAnimationNodes, getNextStaggerDelay } from '@/services/ai/design-animation'
import { removePreviewNode, removeAgentIndicator } from './agent-indicator'
import { resolveNodeForCanvas, getDefaultTheme } from '@/variables/resolve-variables'
import { COMPONENT_COLOR, INSTANCE_COLOR, SELECTION_BLUE } from './canvas-constants'
import {
  type Padding,
  resolvePadding,
  isNodeVisible,
  getNodeWidth,
  getNodeHeight,
  computeLayoutPositions,
  inferLayout,
} from './canvas-layout-engine'
import { parseSizing } from './canvas-text-measure'

// ---------------------------------------------------------------------------
// Clip info — tracks parent frame bounds for child clipping
// ---------------------------------------------------------------------------

interface ClipInfo {
  x: number
  y: number
  w: number
  h: number
  rx: number
}

// ---------------------------------------------------------------------------
// Render info — tracks parent offset & layout status for each node.
// Used by use-canvas-events to convert absolute ↔ relative positions.
// ---------------------------------------------------------------------------

export interface NodeRenderInfo {
  parentOffsetX: number
  parentOffsetY: number
  isLayoutChild: boolean
  /** The ID of the top-level root frame this node belongs to. */
  rootFrameId?: string
  /** Nesting depth: 0 = root frame, 1 = immediate child, etc. */
  depth: number
}

/** Rebuilt every sync cycle. Maps nodeId → parent offset + layout child status. */
export const nodeRenderInfo = new Map<string, NodeRenderInfo>()

/** Maps root-frame IDs to their absolute bounds. Rebuilt every sync cycle. */
export const rootFrameBounds = new Map<string, { x: number; y: number; w: number; h: number }>()

// ---------------------------------------------------------------------------
// Viewport culling — only create Fabric objects for visible root frames
// ---------------------------------------------------------------------------

/** Margin in scene-space pixels around the viewport for pre-rendering. */
const VIEWPORT_MARGIN = 200

interface ViewportRect {
  left: number
  top: number
  right: number
  bottom: number
}

/** Compute the viewport bounds in scene coordinates from Fabric's viewport transform. */
function getViewportBounds(canvas: fabric.Canvas): ViewportRect {
  const vpt = canvas.viewportTransform
  const zoom = vpt[0] || 1
  const panX = vpt[4] || 0
  const panY = vpt[5] || 0
  const w = canvas.getWidth()
  const h = canvas.getHeight()
  return {
    left: -panX / zoom,
    top: -panY / zoom,
    right: (-panX + w) / zoom,
    bottom: (-panY + h) / zoom,
  }
}

/** Check if a rectangle overlaps the viewport (with margin). */
function isRectInViewport(
  rect: { x: number; y: number; w: number; h: number },
  vp: ViewportRect,
  margin: number,
): boolean {
  return !(
    rect.x + rect.w < vp.left - margin
    || rect.x > vp.right + margin
    || rect.y + rect.h < vp.top - margin
    || rect.y > vp.bottom + margin
  )
}


/** Info for layout containers — used by drag-into-layout for hit detection. */
export interface LayoutContainerInfo {
  x: number; y: number; w: number; h: number
  layout: 'vertical' | 'horizontal'
  padding: Padding
  gap: number
}

/** Maps layout container IDs to their absolute bounds + layout info. Rebuilt every sync cycle. */
export const layoutContainerBounds = new Map<string, LayoutContainerInfo>()

// ---------------------------------------------------------------------------
// Resolve RefNodes — expand instances by looking up their referenced component
// ---------------------------------------------------------------------------

/** Give children unique IDs scoped to the instance, apply overrides from descendants. */
function remapInstanceChildIds(
  children: PenNode[],
  refId: string,
  overrides?: Record<string, Partial<PenNode>>,
): PenNode[] {
  return children.map((child) => {
    const virtualId = `${refId}__${child.id}`
    const ov = overrides?.[child.id] ?? {}
    const mapped = { ...child, ...ov, id: virtualId } as PenNode
    if ('children' in mapped && mapped.children) {
      ;(mapped as PenNode & { children: PenNode[] }).children =
        remapInstanceChildIds(mapped.children, refId, overrides)
    }
    return mapped
  })
}

/**
 * Recursively resolve all RefNodes in the tree by expanding them
 * with their referenced component's structure.
 */
function resolveRefs(
  nodes: PenNode[],
  rootNodes: PenNode[],
  visited = new Set<string>(),
): PenNode[] {
  return nodes.flatMap((node) => {
    if (node.type !== 'ref') {
      if ('children' in node && node.children) {
        return [
          {
            ...node,
            children: resolveRefs(node.children, rootNodes, visited),
          } as PenNode,
        ]
      }
      return [node]
    }

    // Resolve RefNode
    if (visited.has(node.ref)) return [] // circular reference guard
    const component = findNodeInTree(rootNodes, node.ref)
    if (!component) return []

    visited.add(node.ref)

    const refNode = node as PenNode & { descendants?: Record<string, Partial<PenNode>> }
    // Apply top-level visual overrides from descendants[componentId]
    const topOverrides = refNode.descendants?.[node.ref] ?? {}

    // Build resolved node: component base → overrides → RefNode's own properties
    const resolved: Record<string, unknown> = { ...component, ...topOverrides }
    // Apply all explicitly-defined RefNode properties (position, size, opacity, etc.)
    for (const [key, val] of Object.entries(node)) {
      if (key === 'type' || key === 'ref' || key === 'descendants' || key === 'children') continue
      if (val !== undefined) {
        resolved[key] = val
      }
    }
    // Use component's type (not 'ref') and ensure name fallback
    resolved.type = component.type
    if (!resolved.name) resolved.name = component.name
    // Clear the reusable flag — this is an instance, not the component
    delete resolved.reusable
    const resolvedNode = resolved as unknown as PenNode

    // Remap children IDs to avoid clashes with the original component
    if ('children' in resolvedNode && resolvedNode.children) {
      ;(resolvedNode as PenNode & { children: PenNode[] }).children =
        remapInstanceChildIds(
          resolvedNode.children,
          node.id,
          refNode.descendants,
        )
    }

    visited.delete(node.ref)
    return [resolvedNode]
  })
}

// ---------------------------------------------------------------------------
// Flatten document tree → absolute-positioned list for Fabric.js
// ---------------------------------------------------------------------------

function cornerRadiusVal(
  cr: number | [number, number, number, number] | undefined,
): number {
  if (cr === undefined) return 0
  if (typeof cr === 'number') return cr
  return cr[0]
}

/** Create thin rectangle PenNodes for directional border sides. */
function createDirectionalBorderNodes(
  parentId: string,
  x: number, y: number, w: number, h: number,
  sides: { top: number; right: number; bottom: number; left: number },
  color: string,
): PenNode[] {
  const nodes: PenNode[] = []
  const fill = [{ type: 'solid' as const, color }]
  if (sides.top > 0) {
    nodes.push({
      id: `${parentId}__border_top`, type: 'rectangle', name: '_border',
      x, y, width: w, height: sides.top, fill,
    } as unknown as PenNode)
  }
  if (sides.bottom > 0) {
    nodes.push({
      id: `${parentId}__border_bottom`, type: 'rectangle', name: '_border',
      x, y: y + h - sides.bottom, width: w, height: sides.bottom, fill,
    } as unknown as PenNode)
  }
  if (sides.left > 0) {
    nodes.push({
      id: `${parentId}__border_left`, type: 'rectangle', name: '_border',
      x, y, width: sides.left, height: h, fill,
    } as unknown as PenNode)
  }
  if (sides.right > 0) {
    nodes.push({
      id: `${parentId}__border_right`, type: 'rectangle', name: '_border',
      x: x + w - sides.right, y, width: sides.right, height: h, fill,
    } as unknown as PenNode)
  }
  return nodes
}

function flattenNodes(
  nodes: PenNode[],
  offsetX = 0,
  offsetY = 0,
  parentAvailW?: number,
  parentAvailH?: number,
  clipCtx?: ClipInfo,
  clipMap?: Map<string, ClipInfo>,
  isLayoutChild = false,
  depth = 0,
  rootFrameId?: string,
): PenNode[] {
  const result: PenNode[] = []
  // Iterate children in REVERSE so that children[0] (top of layer panel)
  // is added to the canvas LAST → renders in front. This matches the
  // standard design tool convention: top of layer panel = frontmost element.
  for (let i = nodes.length - 1; i >= 0; i--) {
    const node = nodes[i]
    if (!isNodeVisible(node)) continue

    // Track which root frame this node belongs to
    const currentRootId = depth === 0 ? node.id : rootFrameId

    // Store render info for position conversion in canvas events
    nodeRenderInfo.set(node.id, {
      parentOffsetX: offsetX,
      parentOffsetY: offsetY,
      isLayoutChild,
      rootFrameId: currentRootId,
      depth,
    })

    // Resolve fill_container / fit_content string sizes into pixel values
    let resolved = node
    if (parentAvailW !== undefined || parentAvailH !== undefined) {
      let changed = false
      const r: Record<string, unknown> = { ...node }
      if ('width' in node && typeof node.width !== 'number') {
        const s = parseSizing(node.width)
        if (s === 'fill' && parentAvailW) {
          r.width = parentAvailW
          changed = true
        } else if (s === 'fit') {
          r.width = getNodeWidth(node, parentAvailW)
          changed = true
        }
      }
      if ('height' in node && typeof node.height !== 'number') {
        const s = parseSizing(node.height)
        if (s === 'fill' && parentAvailH) {
          r.height = parentAvailH
          changed = true
        } else if (s === 'fit') {
          r.height = getNodeHeight(node, parentAvailH, parentAvailW)
          changed = true
        }
      }
      if (changed) resolved = r as unknown as PenNode
    }

    // For frames without explicit numeric height (especially root frames),
    // compute height from children so the frame isn't collapsed to fallback.
    if (
      node.type === 'frame'
      && 'children' in node
      && node.children?.length
      && (!('height' in resolved) || typeof resolved.height !== 'number')
    ) {
      const computedH = getNodeHeight(resolved, parentAvailH, parentAvailW)
      if (computedH > 0) {
        resolved = { ...resolved, height: computedH } as unknown as PenNode
      }
    }

    // Apply parent offset to get absolute position for rendering
    const absoluteNode =
      offsetX !== 0 || offsetY !== 0
        ? {
            ...resolved,
            x: (resolved.x ?? 0) + offsetX,
            y: (resolved.y ?? 0) + offsetY,
          }
        : resolved

    // Store clip info from parent frame (if any)
    if (clipCtx && clipMap) {
      clipMap.set(node.id, clipCtx)
    }

    result.push(absoluteNode as PenNode)

    // Inject synthetic border rectangles for directional strokes.
    // Fabric.js doesn't support per-side strokes, so we render them as
    // thin rectangles positioned at the specified edges.
    if ('stroke' in node && isDirectionalStroke(node.stroke)) {
      const strokeColor = resolveStrokeColor(node.stroke)
      // Only render directional borders when the stroke has an explicit fill color.
      // Pencil uses fill-less directional strokes for internal spacing, not visible borders.
      if (strokeColor) {
        const absX = (absoluteNode as PenNode).x ?? 0
        const absY = (absoluteNode as PenNode).y ?? 0
        const nodeW = getNodeWidth(resolved, parentAvailW)
        const nodeH = getNodeHeight(resolved, parentAvailH, parentAvailW)
        const sides = getDirectionalStrokeThicknesses(node.stroke!)
        const borderNodes = createDirectionalBorderNodes(
          node.id, absX, absY, nodeW, nodeH, sides, strokeColor,
        )
        for (const bn of borderNodes) result.push(bn)
      }
    }

    const children = 'children' in node ? node.children : undefined
    if (children && children.length > 0) {
      const parentAbsX = (resolved.x ?? 0) + offsetX
      const parentAbsY = (resolved.y ?? 0) + offsetY

      // Compute available dimensions for children
      const nodeW = getNodeWidth(resolved, parentAvailW)
      const nodeH = getNodeHeight(resolved, parentAvailH, parentAvailW)
      const pad = resolvePadding(
        'padding' in resolved ? (resolved as any).padding : undefined,
      )
      const childAvailW = Math.max(0, nodeW - pad.left - pad.right)
      const childAvailH = Math.max(0, nodeH - pad.top - pad.bottom)

      // If the parent has an auto-layout, compute child positions first.
      // Infer horizontal layout when gap/justifyContent/alignItems are set
      // but layout is not — matches the inference in computeLayoutPositions.
      const layout = ('layout' in node ? (node as ContainerProps).layout : undefined)
        || inferLayout(node)
      const positioned =
        layout && layout !== 'none'
          ? computeLayoutPositions(resolved, children)
          : children

      // Compute clip context for children:
      // - Root frames (depth 0, type frame) always clip their children
      // - Non-root frames clip only when they have cornerRadius
      let childClip = clipCtx
      const crRaw = 'cornerRadius' in node ? cornerRadiusVal(node.cornerRadius) : 0
      const cr = Math.min(crRaw, nodeH / 2)
      const isRootFrame = node.type === 'frame' && depth === 0
      const hasClipContent = 'clipContent' in node && (node as ContainerProps).clipContent === true
      if (isRootFrame || cr > 0 || hasClipContent) {
        childClip = { x: parentAbsX, y: parentAbsY, w: nodeW, h: nodeH, rx: cr }
      }

      // Track root frame bounds for drag-out reparenting
      if (isRootFrame) {
        rootFrameBounds.set(node.id, { x: parentAbsX, y: parentAbsY, w: nodeW, h: nodeH })
      }

      // Track layout container bounds for drag-into detection
      if (layout && layout !== 'none') {
        const gap = 'gap' in node && typeof (node as any).gap === 'number' ? (node as any).gap : 0
        layoutContainerBounds.set(node.id, {
          x: parentAbsX, y: parentAbsY, w: nodeW, h: nodeH,
          layout: layout as 'vertical' | 'horizontal',
          padding: pad, gap,
        })
      }

      // Children inside layout containers are layout-controlled (position not manually editable)
      const childIsLayoutChild = !!(layout && layout !== 'none')

      result.push(
        ...flattenNodes(positioned, parentAbsX, parentAbsY, childAvailW, childAvailH, childClip, clipMap, childIsLayoutChild, depth + 1, currentRootId),
      )
    }
  }
  return result
}

/**
 * Rebuild nodeRenderInfo from the current document state.
 * Called after locked syncs (e.g. object:modified) so that subsequent
 * panel-driven property changes use fresh parent-offset data.
 */
export function rebuildNodeRenderInfo() {
  const state = useDocumentStore.getState()
  const activePageId = useCanvasStore.getState().activePageId
  const pageChildren = getActivePageChildren(state.document, activePageId)
  const allNodes = getAllChildren(state.document)
  nodeRenderInfo.clear()
  rootFrameBounds.clear()
  layoutContainerBounds.clear()
  const resolvedTree = resolveRefs(pageChildren, allNodes)
  flattenNodes(resolvedTree, 0, 0, undefined, undefined, undefined, new Map())
}

/**
 * Force-sync every Fabric object's position/size back to the document store.
 * Call this before saving to guarantee the file captures the latest canvas state,
 * even if a real-time sync was missed for any reason.
 */
/**
 * Collect position/size updates from all Fabric objects and apply them to the
 * document store in a **single** state write — no per-object `updateNode()`
 * calls and no undo-history pushes.  This makes save-before-export O(n)
 * instead of O(n²) and avoids flooding the history stack.
 */
export function syncCanvasPositionsToStore() {
  const canvas = useCanvasStore.getState().fabricCanvas
  if (!canvas) return

  // Ensure nodeRenderInfo is fresh
  rebuildNodeRenderInfo()

  // 1. Collect all updates keyed by node id
  const updateMap = new Map<string, Record<string, unknown>>()
  const objects = canvas.getObjects() as FabricObjectWithPenId[]

  for (const obj of objects) {
    if (!obj.penNodeId) continue

    const info = nodeRenderInfo.get(obj.penNodeId)
    const offsetX = info?.parentOffsetX ?? 0
    const offsetY = info?.parentOffsetY ?? 0
    const scaleX = obj.scaleX ?? 1
    const scaleY = obj.scaleY ?? 1

    const updates: Record<string, unknown> = {
      x: (obj.left ?? 0) - offsetX,
      y: (obj.top ?? 0) - offsetY,
      rotation: obj.angle ?? 0,
    }

    if (obj.width !== undefined) {
      updates.width = obj.width * scaleX
    }
    if (obj.height !== undefined) {
      updates.height = obj.height * scaleY
    }

    // Sync text content too
    if ('text' in obj && typeof (obj as any).text === 'string') {
      updates.content = (obj as any).text
    }

    updateMap.set(obj.penNodeId, updates)
  }

  if (updateMap.size === 0) return

  // 2. Apply all updates in a single tree walk + single store set
  function applyUpdates(nodes: PenNode[]): PenNode[] {
    return nodes.map((n) => {
      const upd = updateMap.get(n.id)
      const patched = upd ? ({ ...n, ...upd } as PenNode) : n
      if ('children' in patched && patched.children) {
        const newChildren = applyUpdates(patched.children)
        if (newChildren !== patched.children) {
          return { ...patched, children: newChildren } as PenNode
        }
      }
      return patched
    })
  }

  setFabricSyncLock(true)
  try {
    const state = useDocumentStore.getState()
    const activePageId = useCanvasStore.getState().activePageId
    const children = getActivePageChildren(state.document, activePageId)
    const updated = applyUpdates(children)
    const newDoc = setActivePageChildren(state.document, activePageId, updated)
    // Direct set — no history push, no per-node overhead
    useDocumentStore.setState({ document: newDoc })
  } finally {
    setFabricSyncLock(false)
  }
}

// ---------------------------------------------------------------------------
// Viewport-driven object materialization — instead of creating ALL Fabric
// objects and toggling `visible`, we only add objects to the canvas when
// they should be rendered.  Off-screen / deeply-nested objects are removed
// from the canvas and held in a lightweight pool.  This dramatically
// reduces Fabric's per-frame iteration count (e.g. from 2949 to ~200).
// ---------------------------------------------------------------------------

/** Pool of Fabric objects removed from the canvas but kept for quick re-add. */
const offscreenPool = new Map<string, FabricObjectWithPenId>()

/** Set of penNodeIds currently on the canvas. */
const onCanvasIds = new Set<string>()

/** Desired z-order for all flat nodes (built during sync). */
let expectedNodeOrder: string[] = []

/** Whether a progressive materialization is currently in progress. */
let materializationRafId: number | null = null

/** Determine whether a node should be on canvas given current viewport.
 *  Only viewport culling is applied: children of off-screen root frames are
 *  excluded. Depth/size LOD is no longer needed because the zoom cache
 *  (canvas-zoom-cache.ts) renders a pixel snapshot during zoom/pan, making
 *  per-object rendering cost irrelevant during navigation. */
function shouldNodeBeOnCanvas(
  nodeId: string,
  visibleRootIds: Set<string>,
): boolean {
  const info = nodeRenderInfo.get(nodeId)
  if (!info) return false

  const isRootFrame = !info.rootFrameId || info.rootFrameId === nodeId

  // Frame-level culling: skip children of off-screen root frames
  if (!isRootFrame && info.rootFrameId && !visibleRootIds.has(info.rootFrameId)) {
    return false
  }

  return true
}

function updateObjectVisibility(canvas: fabric.Canvas) {
  const vpBounds = getViewportBounds(canvas)

  // Determine which root frames overlap the viewport
  const visibleRootIds = new Set<string>()
  for (const [id, bounds] of rootFrameBounds) {
    if (isRectInViewport(bounds, vpBounds, VIEWPORT_MARGIN)) {
      visibleRootIds.add(id)
    }
  }

  // 1. Remove objects that should no longer be on canvas.
  //    Collect all removals first, then batch-process to avoid O(n²)
  //    from repeated canvas.remove() splice operations.
  const objects = canvas.getObjects() as FabricObjectWithPenId[]
  const toRemove: FabricObjectWithPenId[] = []
  for (const obj of objects) {
    if (!obj.penNodeId) continue
    if (!shouldNodeBeOnCanvas(obj.penNodeId, visibleRootIds)) {
      toRemove.push(obj)
    }
  }
  if (toRemove.length > 0) {
    // For large batches, manipulate the internal array directly for O(n)
    // instead of calling canvas.remove() per object which is O(n) each.
    const removeSet = new Set(toRemove)
    const internalObjects = (canvas as any)._objects as fabric.FabricObject[]
    if (internalObjects) {
      const kept = internalObjects.filter((o) => !removeSet.has(o as FabricObjectWithPenId))
      internalObjects.length = 0
      internalObjects.push(...kept)
    } else {
      // Fallback: individual remove
      for (const obj of toRemove) canvas.remove(obj)
    }
    for (const obj of toRemove) {
      offscreenPool.set(obj.penNodeId!, obj)
      onCanvasIds.delete(obj.penNodeId!)
      // Detach from canvas group reference
      ;(obj as any).canvas = undefined
      ;(obj as any).group = undefined
    }
  }

  // 2. Find objects that should be added to canvas
  const toAdd: FabricObjectWithPenId[] = []
  for (const nodeId of expectedNodeOrder) {
    if (onCanvasIds.has(nodeId)) continue
    const pooled = offscreenPool.get(nodeId)
    if (!pooled) continue
    if (shouldNodeBeOnCanvas(nodeId, visibleRootIds)) {
      toAdd.push(pooled)
    }
  }

  // 3. Progressive materialization: add objects in batches to avoid
  //    a single-frame spike when crossing LOD thresholds.
  if (toAdd.length > 0) {
    // Cancel any in-flight progressive add
    if (materializationRafId !== null) cancelAnimationFrame(materializationRafId)

    const BATCH_SIZE = 150
    let idx = 0

    function addBatch() {
      materializationRafId = null
      if (!canvas || idx >= toAdd.length) return
      const end = Math.min(idx + BATCH_SIZE, toAdd.length)
      for (let i = idx; i < end; i++) {
        const obj = toAdd[i]
        if (!obj.penNodeId) continue
        offscreenPool.delete(obj.penNodeId)
        onCanvasIds.add(obj.penNodeId)
        canvas.add(obj)
      }
      idx = end
      fixZOrder(canvas)
      canvas.requestRenderAll()

      if (idx < toAdd.length) {
        materializationRafId = requestAnimationFrame(addBatch)
      }
    }

    // If batch is small, do it synchronously (no jank)
    if (toAdd.length <= BATCH_SIZE) {
      addBatch()
    } else {
      // First batch immediately, rest progressive
      addBatch()
    }
    return
  }

  if (toRemove.length > 0) {
    canvas.requestRenderAll()
  }
}

/** Quickly fix z-order after adding objects from pool. */
function fixZOrder(canvas: fabric.Canvas) {
  const orderMap = new Map<string, number>()
  for (let i = 0; i < expectedNodeOrder.length; i++) {
    orderMap.set(expectedNodeOrder[i], i)
  }

  const objs = canvas.getObjects() as FabricObjectWithPenId[]
  // Check if already in correct order (common case — skip sort)
  let sorted = true
  let prevIdx = -1
  for (const o of objs) {
    if (!o.penNodeId) continue
    const idx = orderMap.get(o.penNodeId) ?? -1
    if (idx < prevIdx) { sorted = false; break }
    prevIdx = idx
  }
  if (sorted) return

  // Sort by desired order — uses Fabric's moveObjectTo
  const desired = objs
    .filter((o) => o.penNodeId)
    .sort((a, b) => (orderMap.get(a.penNodeId!) ?? 0) - (orderMap.get(b.penNodeId!) ?? 0))

  for (let i = 0; i < desired.length; i++) {
    const current = canvas.getObjects().indexOf(desired[i])
    if (current !== i) canvas.moveObjectTo(desired[i], i)
  }
}

export function useCanvasSync() {
  useEffect(() => {
    // Track the previous document reference so we only re-sync Fabric when
    // the document tree actually changes — not on every store update (e.g.
    // `isDirty`, `fileName`).  Without this guard, operations like
    // `markClean()` trigger a full re-sync that overwrites canvas-side
    // changes (drag positions, edited text) with stale store data if those
    // changes failed to write back to the store for any reason.
    let prevPageChildren = getActivePageChildren(
      useDocumentStore.getState().document,
      useCanvasStore.getState().activePageId,
    )
    let prevVariables = useDocumentStore.getState().document.variables
    let prevThemes = useDocumentStore.getState().document.themes
    let prevActivePageId = useCanvasStore.getState().activePageId

    // Subscribe to page switches and canvas initialization
    let prevFabricCanvas = useCanvasStore.getState().fabricCanvas
    const unsubCanvas = useCanvasStore.subscribe((cs) => {
      if (cs.activePageId !== prevActivePageId) {
        prevActivePageId = cs.activePageId
        // Force a full re-sync by resetting prevPageChildren
        prevPageChildren = null as unknown as PenNode[]
        // Trigger document subscription by creating a new reference.
        // Always call setState — even for empty pages — so old canvas
        // objects are cleared and the sync runs unconditionally.
        const { document: doc } = useDocumentStore.getState()
        const pageChildren = getActivePageChildren(doc, cs.activePageId)
        useDocumentStore.setState({
          document: setActivePageChildren(doc, cs.activePageId, [...pageChildren]),
        })
      }
      // When fabricCanvas transitions from null → ready, force a full re-sync
      // so that documents loaded before the canvas was ready get rendered.
      if (cs.fabricCanvas && !prevFabricCanvas) {
        prevPageChildren = null as unknown as PenNode[]
        const { document: doc } = useDocumentStore.getState()
        const pageChildren = getActivePageChildren(doc, cs.activePageId)
        if (pageChildren.length > 0) {
          useDocumentStore.setState({
            document: setActivePageChildren(doc, cs.activePageId, [...pageChildren]),
          })
        }
      }
      prevFabricCanvas = cs.fabricCanvas
    })

    const unsub = useDocumentStore.subscribe((state) => {
      const activePageId = useCanvasStore.getState().activePageId
      const pageChildren = getActivePageChildren(state.document, activePageId)

      const childrenChanged = pageChildren !== prevPageChildren
      const variablesChanged = state.document.variables !== prevVariables
      const themesChanged = state.document.themes !== prevThemes

      // When the sync lock is active, track references so that unrelated
      // store updates (e.g. markClean) don't trigger stale re-syncs.
      if (isFabricSyncLocked()) {
        prevPageChildren = pageChildren
        prevVariables = state.document.variables
        prevThemes = state.document.themes
        return
      }

      // Skip re-sync when only non-document fields changed (isDirty, fileName, etc.)
      if (!childrenChanged && !variablesChanged && !themesChanged) return

      const canvas = useCanvasStore.getState().fabricCanvas
      // Don't update prev references when canvas isn't ready — otherwise
      // the change is "consumed" without syncing and won't trigger again.
      if (!canvas) return

      prevPageChildren = pageChildren
      prevVariables = state.document.variables
      prevThemes = state.document.themes

      // Build variable resolution context
      const variables = state.document.variables ?? {}
      const activeTheme = getDefaultTheme(state.document.themes)

      // Use active page children for rendering, all children for ref resolution
      const allNodes = getAllChildren(state.document)

      const clipMap = new Map<string, ClipInfo>()
      nodeRenderInfo.clear()
      rootFrameBounds.clear()
      layoutContainerBounds.clear()
      // Preserve offscreenPool across syncs: existing Fabric objects are reused
      // via objMap lookup below, avoiding O(n) recreation on every document edit.
      // Stale entries (deleted nodes) are purged after the removal pass.
      onCanvasIds.clear()
      // Resolve RefNodes before flattening so instances render as their component
      const resolvedTree = resolveRefs(pageChildren, allNodes)
      const flatNodes = flattenNodes(
        resolvedTree, 0, 0, undefined, undefined, undefined, clipMap,
      ).map((node) => resolveNodeForCanvas(node, variables, { ...activeTheme, ...node.theme }))

      // Build expected z-order for materialization
      expectedNodeOrder = flatNodes.filter((n) => n.type !== 'ref').map((n) => n.id)

      const nodeMap = new Map(flatNodes.map((n) => [n.id, n]))
      const objects = canvas.getObjects() as FabricObjectWithPenId[]
      // Build objMap from BOTH canvas objects AND offscreen pool
      const objMap = new Map(
        objects
          .filter((o) => o.penNodeId)
          .map((o) => [o.penNodeId!, o]),
      )
      for (const [id, obj] of offscreenPool) {
        if (!objMap.has(id)) objMap.set(id, obj)
      }

      // Collect component and instance IDs for selection styling
      const reusableIds = new Set<string>()
      const instanceIds = new Set<string>()
      ;(function collectComponentIds(nodes: PenNode[]) {
        for (const n of nodes) {
          if ('reusable' in n && n.reusable === true) reusableIds.add(n.id)
          if (n.type === 'ref') instanceIds.add(n.id)
          if ('children' in n && n.children) collectComponentIds(n.children)
        }
      })(pageChildren)

      // Remove objects that no longer exist in the active set, and
      // deduplicate: when multiple Fabric objects share the same penNodeId
      // (e.g. from ID collisions across separate AI generations), keep only
      // the one tracked in objMap and remove the rest.
      for (const obj of objects) {
        if (!obj.penNodeId) continue
        if (!nodeMap.has(obj.penNodeId)) {
          canvas.remove(obj)
          onCanvasIds.delete(obj.penNodeId)
        } else if (objMap.get(obj.penNodeId) !== obj) {
          canvas.remove(obj)
        }
      }
      // Also purge pool entries for nodes no longer in the document
      for (const id of offscreenPool.keys()) {
        if (!nodeMap.has(id)) {
          offscreenPool.delete(id)
        }
      }

      // Rebuild onCanvasIds from surviving canvas objects
      for (const obj of canvas.getObjects() as FabricObjectWithPenId[]) {
        if (obj.penNodeId) onCanvasIds.add(obj.penNodeId)
      }

      // Add or update objects
      for (const node of flatNodes) {
        if (node.type === 'ref') continue // Skip unresolved refs

        let obj: FabricObjectWithPenId | undefined
        let existingObj = objMap.get(node.id)

        // Some object types require recreation when their underlying Fabric
        // class/shape data changes (e.g. IText↔Textbox).
        // Path `d` changes are handled in-place by syncFabricObject.
        let objectRecreated = false
        // Text nodes may need recreation when textGrowth mode or textAlign changes
        // (IText ↔ Textbox are different Fabric classes).
        // Must match isFixedWidthText() in canvas-object-factory.ts.
        if (existingObj && node.type === 'text') {
          const growth = node.textGrowth
          const needsTextbox = growth === 'fixed-width' || growth === 'fixed-width-height'
            || (node.textAlign != null && node.textAlign !== 'left')
          const isTextbox = existingObj instanceof fabric.Textbox
          if (needsTextbox !== isTextbox) {
            canvas.remove(existingObj)
            onCanvasIds.delete(node.id)
            offscreenPool.delete(node.id)
            existingObj = undefined
            objectRecreated = true
          }
        }

        if (existingObj) {
          // Skip objects inside an ActiveSelection — their left/top are
          // group-relative, not absolute.  Setting absolute values from
          // the store would move them to wrong positions (snap-back bug).
          if (existingObj.group instanceof fabric.ActiveSelection) {
            continue
          }
          syncFabricObject(existingObj, node)

          // Check if sync flagged this object for recreation (e.g. image
          // fit mode changed between tile ↔ non-tile, requiring a different
          // Fabric class).
          if ((existingObj as any).__needsRecreation) {
            canvas.remove(existingObj)
            onCanvasIds.delete(node.id)
            offscreenPool.delete(node.id)
            existingObj = undefined
            objectRecreated = true
          } else {
            obj = existingObj
          }
        }
        if (!existingObj) {
          const newObj = createFabricObject(node)
          if (newObj) {
            const shouldAnimate = pendingAnimationNodes.has(node.id)
            if (shouldAnimate) {
              const targetOpacity = newObj.opacity ?? 1
              // Sequential queue delay: includes BORDER_LEAD (border shows first)
              const totalDelay = getNextStaggerDelay(node.id)
              newObj.set({ opacity: 0 })
              canvas.add(newObj)
              onCanvasIds.add(node.id)
              setTimeout(() => {
                removePreviewNode(node.id)
                newObj.animate({ opacity: targetOpacity }, {
                  duration: 300,
                  easing: fabric.util.ease.easeOutCubic,
                  onChange: () => canvas.requestRenderAll(),
                  onComplete: () => {
                    pendingAnimationNodes.delete(node.id)
                    removeAgentIndicator(node.id)
                  },
                })
              }, totalDelay)
            } else {
              // Don't add to canvas yet — pool it for lazy materialization.
              // updateObjectVisibility() will add visible objects afterwards.
              offscreenPool.set(node.id, newObj)
            }
            // Restore Fabric selection when an object was recreated
            if (objectRecreated) {
              const { activeId } = useCanvasStore.getState().selection
              if (activeId === node.id) {
                canvas.add(newObj)
                onCanvasIds.add(node.id)
                offscreenPool.delete(node.id)
                canvas.setActiveObject(newObj)
              }
            }
            obj = newObj
          }
        }

        if (obj) {
          // Component/instance selection border styling
          if (reusableIds.has(node.id)) {
            obj.borderColor = COMPONENT_COLOR
            obj.cornerColor = COMPONENT_COLOR
            obj.borderDashArray = []
          } else if (instanceIds.has(node.id)) {
            obj.borderColor = INSTANCE_COLOR
            obj.cornerColor = INSTANCE_COLOR
            obj.borderDashArray = [4, 4]
          } else if (obj.borderColor === COMPONENT_COLOR || obj.borderColor === INSTANCE_COLOR) {
            obj.borderColor = SELECTION_BLUE
            obj.cornerColor = SELECTION_BLUE
            obj.borderDashArray = []
          }

          // Apply clip path from parent frame with clipContent / cornerRadius.
          // Skip if the object already has a self-contained clip (e.g. image
          // corner radius, absolutePositioned: false) — overwriting it with
          // the frame clip would erase the corner radius.
          const clip = clipMap.get(node.id)
          const hasOwnClip = obj.clipPath && !obj.clipPath.absolutePositioned
          if (clip && !hasOwnClip) {
            obj.clipPath = new fabric.Rect({
              left: clip.x,
              top: clip.y,
              width: clip.w,
              height: clip.h,
              rx: clip.rx,
              ry: clip.rx,
              originX: 'left',
              originY: 'top',
              absolutePositioned: true,
            })
            obj.dirty = true
          } else if (obj.clipPath && obj.clipPath.absolutePositioned) {
            obj.clipPath = undefined
            obj.dirty = true
          }
        }
      }

      // Materialize visible objects from pool and fix z-order.
      // updateObjectVisibility will add objects from the pool to the
      // canvas based on current viewport and LOD, then fixZOrder
      // ensures correct stacking.
      updateObjectVisibility(canvas)
      fixZOrder(canvas)
      canvas.requestRenderAll()
    })

    let visibilityRafId: number | null = null
    let prevViewportZoom = 0
    let prevViewportPanX = 0
    let prevViewportPanY = 0

    const unsubViewport = useCanvasStore.subscribe((cs) => {
      if (!cs.fabricCanvas) return
      // Only trigger visibility update when viewport actually changed
      // (zoom or pan), not on every store tick (tool switch, selection, etc.)
      const vpt = cs.fabricCanvas.viewportTransform
      const z = vpt[0] || 1
      const px = vpt[4] || 0
      const py = vpt[5] || 0
      if (z === prevViewportZoom && px === prevViewportPanX && py === prevViewportPanY) return
      prevViewportZoom = z
      prevViewportPanX = px
      prevViewportPanY = py

      // Batch with rAF so visibility updates don't fire on every zoom tick
      if (visibilityRafId !== null) cancelAnimationFrame(visibilityRafId)
      visibilityRafId = requestAnimationFrame(() => {
        visibilityRafId = null
        if (cs.fabricCanvas) updateObjectVisibility(cs.fabricCanvas)
      })
    })

    // Trigger initial sync for the already-existing document.
    // The subscription only fires on future changes, so force a
    // re-render by creating a new children reference.
    const { document: doc } = useDocumentStore.getState()
    const initActivePageId = useCanvasStore.getState().activePageId
    const initChildren = getActivePageChildren(doc, initActivePageId)
    if (initChildren.length > 0) {
      useDocumentStore.setState({
        document: setActivePageChildren(doc, initActivePageId, [...initChildren]),
      })
    }

    return () => {
      unsub()
      unsubCanvas()
      unsubViewport()
      if (visibilityRafId !== null) cancelAnimationFrame(visibilityRafId)
      if (materializationRafId !== null) cancelAnimationFrame(materializationRafId)
      offscreenPool.clear()
      onCanvasIds.clear()
      expectedNodeOrder = []
    }
  }, [])
}
