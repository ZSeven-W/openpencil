import type { PenNode } from '@/types/pen'
import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import {
  pendingAnimationNodes,
  markNodesForAnimation,
  startNewAnimationBatch,
  resetAnimationState,
} from './design-animation'
import {
  clamp,
  toSizeNumber,
  createPhonePlaceholderDataUri,
  estimateNodeIntrinsicHeight,
} from './generation-utils'
import { applyIconPathResolution, applyNoEmojiIconHeuristic, resolveAsyncIcons, resolveAllPendingIcons } from './icon-resolver'
import {
  resolveNodeRole,
  resolveTreeRoles,
  resolveTreePostPass,
} from './role-resolver'
import type { RoleContext } from './role-resolver'
// Trigger side-effect registration of all role definitions
import './role-definitions'
import { extractJsonFromResponse } from './design-parser'

// ---------------------------------------------------------------------------
// Cross-phase ID remapping -- tracks replaceEmptyFrame mappings so that
// later phases recognise the root frame ID has been remapped to DEFAULT_FRAME_ID.
// ---------------------------------------------------------------------------

const generationRemappedIds = new Map<string, string>()
let generationContextHint = ''
/** Root frame width for the current generation (1200 desktop, 375 mobile) */
let generationCanvasWidth = 1200

export function resetGenerationRemapping(): void {
  generationRemappedIds.clear()
}

export function setGenerationContextHint(hint?: string): void {
  generationContextHint = hint?.trim() ?? ''
}

export function setGenerationCanvasWidth(width: number): void {
  generationCanvasWidth = width > 0 ? width : 1200
}

/** Expose the current canvas width for use by other modules (read-only). */
export function getGenerationCanvasWidth(): number {
  return generationCanvasWidth
}

/** Expose the current remapped IDs map for use by other modules (read-only). */
export function getGenerationRemappedIds(): Map<string, string> {
  return generationRemappedIds
}

// ---------------------------------------------------------------------------
// Insert a single streaming node into the canvas
// ---------------------------------------------------------------------------

/**
 * Insert a single streaming node into the canvas instantly.
 * Handles root frame replacement and parent ID remapping.
 * Note: tree-aware heuristics (button width, frame height, clipContent)
 * cannot run here because the node has no children yet during streaming.
 * Use applyPostStreamingTreeHeuristics() after all subtask nodes are inserted.
 */
/**
 * Normalize gradient stop offsets in all fills on a node (in-place).
 * Handles stops without an offset field by auto-distributing them evenly.
 * Also normalizes percentage-format offsets (>1) to the 0-1 range.
 */
function normalizeNodeFills(node: PenNode): void {
  const fills = 'fill' in node ? (node as { fill?: unknown }).fill : undefined
  if (!Array.isArray(fills)) return
  for (const fill of fills) {
    if (!fill || typeof fill !== 'object') continue
    const f = fill as { type?: string; stops?: unknown[] }
    if ((f.type === 'linear_gradient' || f.type === 'radial_gradient') && Array.isArray(f.stops)) {
      const n = f.stops.length
      f.stops = f.stops.map((s: unknown, i: number) => {
        const stop = s as Record<string, unknown>
        let offset = typeof stop.offset === 'number' && Number.isFinite(stop.offset)
          ? stop.offset
          : typeof stop.position === 'number' && Number.isFinite(stop.position)
            ? (stop.position as number)
            : null
        if (offset !== null && offset > 1) offset = offset / 100
        return {
          color: typeof stop.color === 'string' ? stop.color : '#000000',
          offset: offset !== null ? Math.max(0, Math.min(1, offset)) : i / Math.max(n - 1, 1),
        }
      })
    }
  }
}

export function insertStreamingNode(
  node: PenNode,
  parentId: string | null,
): void {
  const { addNode, getNodeById } = useDocumentStore.getState()
  normalizeNodeFills(node)

  // Ensure container nodes have children array for later child insertions
  if ((node.type === 'frame' || node.type === 'group') && !('children' in node)) {
    ;(node as PenNode & { children: PenNode[] }).children = []
  }

  // Resolve remapped parent IDs (e.g., root frame -> DEFAULT_FRAME_ID)
  const resolvedParent = parentId
    ? (generationRemappedIds.get(parentId) ?? parentId)
    : null

  const parentNode = resolvedParent
    ? getNodeById(resolvedParent)
    : null

  if (parentNode && hasActiveLayout(parentNode)) {
    if ('x' in node) delete (node as { x?: number }).x
    if ('y' in node) delete (node as { y?: number }).y
    // Text defaults inside layout frames:
    // - vertical layout: body text prefers fill width for wrapping
    // - horizontal layout: short labels should hug content to avoid squeezing siblings
    if (node.type === 'text') {
      const parentLayout = ('layout' in parentNode ? parentNode.layout : undefined)
      if (parentLayout === 'vertical') {
        if (typeof node.width === 'number') node.width = 'fill_container'
        if (!node.textGrowth) node.textGrowth = 'fixed-width'
      } else if (parentLayout === 'horizontal') {
        if (typeof node.width === 'string' && node.width.startsWith('fill_container')) {
          node.width = 'fit_content'
        }
        if (!node.textGrowth || node.textGrowth === 'fixed-width' || node.textGrowth === 'fixed-width-height') {
          node.textGrowth = 'auto'
        }
      } else if (!node.textGrowth) {
        node.textGrowth = 'fixed-width'
      }
      // Default lineHeight based on text role (heading vs body)
      if (!node.lineHeight) {
        const fs = node.fontSize ?? 16
        node.lineHeight = fs >= 28 ? 1.2 : 1.5
      }
    }
  }

  // Apply role-based defaults before legacy heuristics
  const roleCtx: RoleContext = {
    parentRole: parentNode?.role,
    parentLayout: parentNode && 'layout' in parentNode ? parentNode.layout : undefined,
    canvasWidth: generationCanvasWidth,
  }
  resolveNodeRole(node, roleCtx)

  applyGenerationHeuristics(node)

  // Skip AI-streamed children under phone placeholders. Placeholder internals are
  // normalized post-streaming (at most one centered label text is allowed).
  // Also skip if the parent node doesn't exist on canvas (was itself blocked).
  if (resolvedParent !== null && !parentNode) {
    return
  }
  if (parentNode && isInsidePhonePlaceholder(resolvedParent!, getNodeById)) {
    return
  }

  if (resolvedParent === null && isCanvasOnlyEmptyFrame() && node.type === 'frame') {
    // Root frame replaces the default empty frame -- no animation needed
    replaceEmptyFrame(node)
  } else {
    const effectiveParent = resolvedParent ?? DEFAULT_FRAME_ID
    // Verify parent exists, fall back to root frame
    const parent = getNodeById(effectiveParent)
    const insertParent = parent ? effectiveParent : DEFAULT_FRAME_ID

    // Frames with fills appear instantly (background context for children).
    // All other nodes fade in with staggered animation.
    const nodeFill = 'fill' in node ? node.fill : undefined
    const hasFill = Array.isArray(nodeFill)
      ? nodeFill.length > 0
      : (nodeFill != null && typeof nodeFill === 'object')
    const isBackgroundFrame = node.type === 'frame' && hasFill
    if (!isBackgroundFrame) {
      pendingAnimationNodes.add(node.id)
      startNewAnimationBatch()
    }

    addNode(insertParent, node)

    // When a frame is inserted into a horizontal layout, equalize sibling card widths
    // to prevent overflow when multiple cards are placed in the same row.
    if (node.type === 'frame') {
      equalizeHorizontalSiblings(insertParent)
    }

    // When a top-level section is added directly under the root frame,
    // progressively expand root height to fit the new content.
    if (insertParent === DEFAULT_FRAME_ID) {
      expandRootFrameHeight()
    }
  }
}

// ---------------------------------------------------------------------------
// Canvas apply/upsert operations
// ---------------------------------------------------------------------------

export function applyNodesToCanvas(nodes: PenNode[]): void {
  const { getFlatNodes } = useDocumentStore.getState()
  const existingIds = new Set(getFlatNodes().map((n) => n.id))
  const preparedNodes = sanitizeNodesForInsert(nodes, existingIds)

  // If canvas only has one empty frame, replace it with the generated content
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0])
    resolveAllPendingIcons().catch(console.warn)
    return
  }

  const { addNode, getNodeById } = useDocumentStore.getState()
  // Insert into the root frame if it exists, otherwise at document root
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  const parentId = rootFrame ? DEFAULT_FRAME_ID : null
  for (const node of preparedNodes) {
    addNode(parentId, node)
  }
  adjustRootFrameHeightToContent()
  resolveAllPendingIcons().catch(console.warn)
}

export function upsertNodesToCanvas(nodes: PenNode[]): number {
  const preparedNodes = sanitizeNodesForUpsert(nodes)

  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0])
    return 1
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState()
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  const parentId = rootFrame ? DEFAULT_FRAME_ID : null
  let count = 0

  for (const node of preparedNodes) {
    // Resolve remapped IDs (e.g., root frame that was mapped to DEFAULT_FRAME_ID in Phase 1)
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id
    const existing = getNodeById(resolvedId)
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode)
      updateNode(resolvedId, merged)
    } else {
      addNode(parentId, node)
    }
    count++
  }

  adjustRootFrameHeightToContent()
  return count
}

/** Same as upsertNodesToCanvas but skips sanitization (caller already did it). */
function upsertPreparedNodes(preparedNodes: PenNode[]): number {
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0])
    return 1
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState()
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  const parentId = rootFrame ? DEFAULT_FRAME_ID : null
  let count = 0

  for (const node of preparedNodes) {
    // Resolve remapped IDs (e.g., root frame that was mapped to DEFAULT_FRAME_ID in Phase 1)
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id
    const existing = getNodeById(resolvedId)
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode)
      updateNode(resolvedId, merged)
    } else {
      addNode(parentId, node)
    }
    count++
  }

  adjustRootFrameHeightToContent()
  return count
}

/**
 * Animate nodes onto the canvas with a staggered fade-in effect.
 * Synchronous -- nodes are inserted immediately, and canvas-sync
 * schedules fire-and-forget staggered opacity animations.
 */
export function animateNodesToCanvas(nodes: PenNode[]): void {
  resetGenerationRemapping()
  resetAnimationState()
  const prepared = sanitizeNodesForUpsert(nodes)
  startNewAnimationBatch()
  markNodesForAnimation(prepared)

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
  upsertPreparedNodes(prepared)
  useHistoryStore.getState().endBatch(useDocumentStore.getState().document)

  // Resolve any icons queued for async (brand logos etc.) after nodes are in the store
  resolveAllPendingIcons().catch(console.warn)
}

// ---------------------------------------------------------------------------
// Extract + apply convenience wrappers
// ---------------------------------------------------------------------------

/**
 * Extract PenNode JSON from AI response text and apply to canvas.
 * Returns the number of top-level elements added (0 if nothing found/applied).
 */
export function extractAndApplyDesign(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText)
  if (!nodes || nodes.length === 0) return 0

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
  try {
    applyNodesToCanvas(nodes)
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
  }
  return nodes.length
}

/**
 * Extract PenNode JSON from AI response text and apply updates/insertions to canvas.
 * Handles both new nodes and modifications (matching by ID).
 */
export function extractAndApplyDesignModification(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText)
  if (!nodes || nodes.length === 0) return 0

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState()
  let count = 0

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
  try {
    for (const node of nodes) {
      const existing = getNodeById(node.id)
      if (existing) {
        // Update existing node
        updateNode(node.id, node)
        count++
      } else {
        // It's a new node implied by the modification (e.g. "add a button")
        const rootFrame = getNodeById(DEFAULT_FRAME_ID)
        const parentId = rootFrame ? DEFAULT_FRAME_ID : null
        addNode(parentId, node)
        count++
      }
    }
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
  }
  return count
}

// ---------------------------------------------------------------------------
// Generation heuristics
// ---------------------------------------------------------------------------

/**
 * Lightweight post-parse cleanup applied to each node.
 * Handles icon path resolution, emoji removal, and image placeholder generation.
 * Layout/sizing heuristics are now handled by the role resolver.
 */
export function applyGenerationHeuristics(node: PenNode): void {
  applyIconPathResolution(node)
  applyNoEmojiIconHeuristic(node)
  applyImagePlaceholderHeuristic(node)

  if (!('children' in node) || !Array.isArray(node.children)) return
  for (const child of node.children) {
    applyGenerationHeuristics(child)
  }
}

/**
 * Post-streaming tree heuristics -- applies tree-aware fixes after all nodes
 * of a subtask have been inserted into the store.
 *
 * During streaming, nodes are inserted individually (no children), so tree-aware
 * heuristics like button width expansion, frame height expansion, and clipContent
 * detection fail silently. This function re-runs them on the completed subtree.
 */
export function applyPostStreamingTreeHeuristics(rootNodeId: string): void {
  const { getNodeById, updateNode } = useDocumentStore.getState()
  const rootNode = getNodeById(rootNodeId)
  if (!rootNode || rootNode.type !== 'frame') return
  if (!Array.isArray(rootNode.children) || rootNode.children.length === 0) return

  // Role-based tree resolution + cross-node post-pass
  resolveTreeRoles(rootNode, generationCanvasWidth)
  resolveTreePostPass(rootNode, generationCanvasWidth, getNodeById, updateNode)

  // Resolve pending icons asynchronously via Iconify API (fire-and-forget)
  resolveAsyncIcons(rootNodeId).catch(console.warn)
}

// ---------------------------------------------------------------------------
// Root frame height management
// ---------------------------------------------------------------------------

export function adjustRootFrameHeightToContent(): void {
  const { getNodeById, updateNode } = useDocumentStore.getState()
  const root = getNodeById(DEFAULT_FRAME_ID)
  if (!root || root.type !== 'frame') return
  if (!Array.isArray(root.children) || root.children.length === 0) return

  const requiredHeight = estimateNodeIntrinsicHeight(root)
  const targetHeight = Math.max(320, Math.round(requiredHeight))
  const currentHeight = toSizeNumber(root.height, 0)
  if (currentHeight <= 0) return
  if (Math.abs(currentHeight - targetHeight) < 8) return

  updateNode(DEFAULT_FRAME_ID, { height: targetHeight })
}

/**
 * Expand-only version of adjustRootFrameHeightToContent.
 * Used during streaming: only grows the root frame, never shrinks it.
 * This prevents visual jitter while sections are being progressively added.
 *
 * When a frame is inserted into a horizontal layout parent, check if sibling
 * frame children should be equalized to fill_container to prevent overflow.
 * This runs DURING streaming so cards distribute evenly as they arrive.
 */
export function expandRootFrameHeight(): void {
  const { getNodeById, updateNode } = useDocumentStore.getState()
  const root = getNodeById(DEFAULT_FRAME_ID)
  if (!root || root.type !== 'frame') return
  if (!Array.isArray(root.children) || root.children.length === 0) return

  // Mobile screens have fixed viewport dimensions -- don't auto-expand height.
  const rootWidth = toSizeNumber(root.width, 0)
  if (rootWidth > 0 && rootWidth <= 480) return

  const requiredHeight = estimateNodeIntrinsicHeight(root)
  const targetHeight = Math.max(320, Math.round(requiredHeight))
  const currentHeight = toSizeNumber(root.height, 0)
  // Only grow -- never shrink during progressive generation
  if (currentHeight > 0 && targetHeight <= currentHeight) return

  updateNode(DEFAULT_FRAME_ID, { height: targetHeight })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Check if the canvas only has the default empty frame (no children).
 */
function isCanvasOnlyEmptyFrame(): boolean {
  const { document, getNodeById } = useDocumentStore.getState()
  if (document.children.length !== 1) return false
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  if (!rootFrame) return false
  return !('children' in rootFrame) || !rootFrame.children || rootFrame.children.length === 0
}

/**
 * Replace the default empty frame with the generated frame node,
 * preserving the root frame ID so canvas sync continues to work.
 */
function replaceEmptyFrame(generatedFrame: PenNode): void {
  const { updateNode } = useDocumentStore.getState()
  // Record the remapping so subsequent phases can find this node by its original ID
  generationRemappedIds.set(generatedFrame.id, DEFAULT_FRAME_ID)
  // Keep root frame ID and position (x=0, y=0), take everything else from generated frame
  const { id: _id, x: _x, y: _y, ...rest } = generatedFrame
  updateNode(DEFAULT_FRAME_ID, rest)
}

function equalizeHorizontalSiblings(parentId: string): void {
  const { getNodeById, updateNode } = useDocumentStore.getState()
  const parent = getNodeById(parentId)
  if (!parent || parent.type !== 'frame') return
  if (parent.layout !== 'horizontal') return
  if (!Array.isArray(parent.children) || parent.children.length < 2) return

  // Skip if any card already uses fill_container -- the AI chose it deliberately
  const cardCandidates = parent.children.filter(
    (c) => c.type === 'frame'
      && c.role !== 'phone-mockup'
      && c.role !== 'divider'
      && c.role !== 'badge' && c.role !== 'pill' && c.role !== 'tag'
      && toSizeNumber('height' in c ? c.height : undefined, 0) > 88,
  )
  if (cardCandidates.some((c) => ('width' in c) && c.width === 'fill_container')) return

  const fixedFrames = cardCandidates.filter(
    (c) => 'width' in c && typeof c.width === 'number' && (c.width as number) > 0,
  )
  if (fixedFrames.length < 2) return

  // Only equalize when widths vary significantly (ratio < 0.6)
  const widths = fixedFrames.map((c) => toSizeNumber('width' in c ? c.width : undefined, 0))
  const maxW = Math.max(...widths)
  const minW = Math.min(...widths)
  if (maxW <= 0 || minW / maxW >= 0.6) return

  // Check if they look like a card row (similar heights)
  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0))
  const maxH = Math.max(...heights)
  const minH = Math.min(...heights)
  if (maxH <= 0 || minH / maxH <= 0.5) return

  // Convert to fill_container for even distribution and equal height
  for (const child of fixedFrames) {
    updateNode(child.id, { width: 'fill_container', height: 'fill_container' } as Partial<PenNode>)
  }
}

function applyImagePlaceholderHeuristic(node: PenNode): void {
  if (node.type !== 'image') return

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  const contextMarker = generationContextHint.toLowerCase()
  const contextualScreenshotHint = /(截图|screenshot|mockup|手机|app[-_\s]*screen)/.test(contextMarker)
  const screenshotLike = isScreenshotLikeMarker(marker)
    || (contextualScreenshotHint && /(preview|hero|showcase|phone|screen)/.test(marker))
  if (!screenshotLike) return

  const width = toSizeNumber(node.width, 360)
  const height = toSizeNumber(node.height, 720)
  // Detect dark/light from context hint (dark if mentions dark/terminal/cyber/night)
  const dark = !/(light|bright)/.test(generationContextHint.toLowerCase())
  node.src = createPhonePlaceholderDataUri(width, height, dark)
  if (node.cornerRadius === undefined) {
    node.cornerRadius = 24
  }
}

function isScreenshotLikeMarker(text: string): boolean {
  return /app[-_\s]*screen|screenshot|mockup|phone|mobile|device|截图|手机/.test(text)
}

// ---------------------------------------------------------------------------
// Node sanitization for insert/upsert
// ---------------------------------------------------------------------------

function sanitizeNodesForInsert(
  nodes: PenNode[],
  existingIds: Set<string>,
): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n))

  for (const node of cloned) {
    resolveTreeRoles(node, generationCanvasWidth)
    applyGenerationHeuristics(node)
    sanitizeLayoutChildPositions(node, false)
    sanitizeScreenFrameBounds(node)
  }

  const counters = new Map<string, number>()
  const used = new Set(existingIds)
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters)
  }

  return cloned
}

function sanitizeNodesForUpsert(nodes: PenNode[]): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n))

  for (const node of cloned) {
    resolveTreeRoles(node, generationCanvasWidth)
    applyGenerationHeuristics(node)
    sanitizeLayoutChildPositions(node, false)
    sanitizeScreenFrameBounds(node)
  }

  const counters = new Map<string, number>()
  const used = new Set<string>()
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters)
  }

  return cloned
}

function mergeNodeForProgressiveUpsert(
  existing: PenNode,
  incoming: PenNode,
): PenNode {
  const merged: PenNode = { ...existing, ...incoming } as PenNode
  const existingChildren = 'children' in existing && Array.isArray(existing.children)
    ? existing.children
    : undefined
  const incomingChildren = 'children' in incoming && Array.isArray(incoming.children)
    ? incoming.children
    : undefined

  if (!existingChildren && !incomingChildren) return merged
  if (!incomingChildren) {
    if ('children' in merged && Array.isArray(existingChildren)) {
      setNodeChildren(merged, existingChildren)
    }
    return merged
  }
  if (!existingChildren) {
    setNodeChildren(merged, incomingChildren)
    return merged
  }

  const existingById = new Map(existingChildren.map((c) => [c.id, c] as const))
  const incomingById = new Map(incomingChildren.map((c) => [c.id, c] as const))
  const mergedChildren: PenNode[] = []

  // 1. Existing children first (preserves already-built order)
  for (const ex of existingChildren) {
    const inc = incomingById.get(ex.id)
    mergedChildren.push(inc ? mergeNodeForProgressiveUpsert(ex, inc) : ex)
  }

  // 2. Append new incoming children (progressive sections added at end)
  for (const child of incomingChildren) {
    if (!existingById.has(child.id)) mergedChildren.push(child)
  }

  setNodeChildren(merged, mergedChildren)
  return merged
}

function setNodeChildren(node: PenNode, children: PenNode[]): void {
  ;(node as PenNode & { children?: PenNode[] }).children = children
}

function deepCloneNode(node: PenNode): PenNode {
  return JSON.parse(JSON.stringify(node)) as PenNode
}

/** Check if a node (by ID) is inside a Phone Placeholder frame (any ancestor). */
function isInsidePhonePlaceholder(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
): boolean {
  let current = getNodeById(nodeId)
  while (current) {
    if (current.name === 'Phone Placeholder') return true
    const parent = useDocumentStore.getState().getParentOf(current.id)
    if (!parent) break
    current = parent
  }
  return false
}

function hasActiveLayout(node: PenNode): boolean {
  if (!('layout' in node)) return false
  return node.layout === 'vertical' || node.layout === 'horizontal'
}

function sanitizeLayoutChildPositions(
  node: PenNode,
  parentHasLayout: boolean,
): void {
  if (parentHasLayout) {
    if ('x' in node) delete (node as { x?: number }).x
    if ('y' in node) delete (node as { y?: number }).y
  }

  if (!('children' in node) || !Array.isArray(node.children)) return

  const currentHasLayout = hasActiveLayout(node)
  for (const child of node.children) {
    sanitizeLayoutChildPositions(child, currentHasLayout)
  }
}

function sanitizeScreenFrameBounds(node: PenNode): void {
  if ('children' in node && Array.isArray(node.children)) {
    if (isScreenFrame(node)) {
      clampChildrenIntoScreen(node)
    }
    for (const child of node.children) {
      sanitizeScreenFrameBounds(child)
    }
  }
}

function isScreenFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  if (!('width' in node) || typeof node.width !== 'number') return false
  if (!('height' in node) || typeof node.height !== 'number') return false
  const w = node.width
  const h = node.height
  const isMobileLike = w >= 320 && w <= 480 && h >= 640
  const isDesktopLike = w >= 900 && h >= 600
  return isMobileLike || isDesktopLike
}

function clampChildrenIntoScreen(frame: PenNode): void {
  if (!('children' in frame) || !Array.isArray(frame.children)) return
  if ('layout' in frame && frame.layout && frame.layout !== 'none') return
  if (!('width' in frame) || typeof frame.width !== 'number') return
  if (!('height' in frame) || typeof frame.height !== 'number') return

  const frameW = frame.width
  const frameH = frame.height
  const maxBleedX = frameW * 0.1
  const maxBleedY = frameH * 0.1

  for (const child of frame.children) {
    const childWidth = 'width' in child && typeof child.width === 'number' ? child.width : null
    const childHeight = 'height' in child && typeof child.height === 'number' ? child.height : null
    if (
      typeof child.x !== 'number' ||
      typeof child.y !== 'number' ||
      childWidth === null ||
      childHeight === null
    ) {
      continue
    }

    const minX = -maxBleedX
    const maxX = frameW - childWidth + maxBleedX
    const minY = -maxBleedY
    const maxY = frameH - childHeight + maxBleedY

    child.x = clamp(child.x, minX, maxX)
    child.y = clamp(child.y, minY, maxY)
  }
}

// ---------------------------------------------------------------------------
// Node ID uniqueness
// ---------------------------------------------------------------------------

function ensureUniqueNodeIds(
  node: PenNode,
  used: Set<string>,
  counters: Map<string, number>,
): void {
  const base = normalizeIdBase(node.id, node.type)
  let finalId = base

  if (used.has(finalId)) {
    finalId = makeUniqueId(base, used, counters)
  }

  if (finalId !== node.id) {
    node.id = finalId
  }

  used.add(finalId)

  if (!('children' in node) || !Array.isArray(node.children)) return
  for (const child of node.children) {
    ensureUniqueNodeIds(child, used, counters)
  }
}

function normalizeIdBase(id: string | undefined, type: PenNode['type']): string {
  const trimmed = id?.trim()
  return trimmed && trimmed.length > 0 ? trimmed : `${type}-node`
}

function makeUniqueId(
  base: string,
  used: Set<string>,
  counters: Map<string, number>,
): string {
  let next = counters.get(base) ?? 2
  let candidate = `${base}-${next}`
  while (used.has(candidate)) {
    next += 1
    candidate = `${base}-${next}`
  }
  counters.set(base, next + 1)
  return candidate
}
