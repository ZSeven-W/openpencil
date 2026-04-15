import type { PenNode } from '@/types/pen';
import { useDocumentStore, DEFAULT_FRAME_ID, getActivePageChildren } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { useHistoryStore } from '@/stores/history-store';
import {
  pendingAnimationNodes,
  markNodesForAnimation,
  startNewAnimationBatch,
  resetAnimationState,
} from './design-animation';
import {
  toSizeNumber,
  createPhonePlaceholderDataUri,
  estimateNodeIntrinsicHeight,
} from './generation-utils';
import { defaultLineHeight } from '@/canvas/canvas-text-measure';
import {
  normalizeTreeLayout,
  unwrapFakePhoneMockups,
  stripRedundantSectionFills,
  normalizeStrokeFillSchema,
} from '@/canvas/canvas-layout-engine';
import { forcePageResync } from '@/canvas/canvas-sync-utils';
import {
  applyIconPathResolution,
  applyNoEmojiIconHeuristic,
  resolveAsyncIcons,
  resolveAllPendingIcons,
} from './icon-resolver';
import {
  resolveNodeRole,
  resolveTreeRoles,
  resolveTreePostPass,
  detectThemeFromNode,
} from './role-resolver';
import type { RoleContext } from './role-resolver';
import { rewriteLlmAntiPatterns } from './sanitize-llm-anti-patterns';
// Trigger side-effect registration of all role definitions
import './role-definitions';
import { extractJsonFromResponse } from './design-parser';
import {
  scanAndFillImages,
  enqueueImageForSearch,
  resetImageSearchQueue,
} from './image-search-pipeline';
import {
  deepCloneNode,
  mergeNodeForProgressiveUpsert,
  ensureUniqueNodeIds,
  sanitizeLayoutChildPositions,
  sanitizeScreenFrameBounds,
  hasActiveLayout,
  isBadgeOverlayNode,
} from './design-node-sanitization';

// ---------------------------------------------------------------------------
// Cross-phase ID remapping -- tracks replaceEmptyFrame mappings so that
// later phases recognise the root frame ID has been remapped to DEFAULT_FRAME_ID.
// ---------------------------------------------------------------------------

const generationRemappedIds = new Map<string, string>();
let generationContextHint = '';
/** Root frame width for the current generation (1200 desktop, 375 mobile) */
let generationCanvasWidth = 1200;
/** Root frame ID for the current generation — may differ from DEFAULT_FRAME_ID
 *  when canvas already has content and new content is placed beside it. */
let generationRootFrameId: string = DEFAULT_FRAME_ID;
/** Node IDs that existed on canvas before the current generation started.
 *  Used by upsert sanitization to avoid ID collisions with pre-existing content. */
let preExistingNodeIds = new Set<string>();

/**
 * Return the id of the first top-level frame on the ACTIVE page, or null
 * if the page has no frame children yet. This is the correct "page root
 * frame id" for multi-page documents — DEFAULT_FRAME_ID ("root-frame")
 * only applies to Page 1 because addPage() assigns a fresh nanoid to
 * every subsequent page's initial frame. Use this helper anywhere you
 * previously wrote `getNodeById(DEFAULT_FRAME_ID)` with the intent of
 * locating "the current page's root frame".
 */
function getActivePagePrimaryFrameId(): string | null {
  const doc = useDocumentStore.getState().document;
  const activePageId = useCanvasStore.getState().activePageId;
  const children = getActivePageChildren(doc, activePageId);
  for (const child of children) {
    if (child.type === 'frame') return child.id;
  }
  return null;
}

export function resetGenerationRemapping(): void {
  generationRemappedIds.clear();
  // Fall back to DEFAULT_FRAME_ID only when the active page has no frame yet
  // (e.g. first load, legacy docs). In the multi-page case this is the
  // nanoid from addPage().
  generationRootFrameId = getActivePagePrimaryFrameId() ?? DEFAULT_FRAME_ID;
  // Snapshot all existing node IDs so upsert can avoid collisions
  preExistingNodeIds = new Set(
    useDocumentStore
      .getState()
      .getFlatNodes()
      .map((n) => n.id),
  );
  // Reset incremental image search queue for the new generation
  resetImageSearchQueue();
}

export function setGenerationContextHint(hint?: string): void {
  generationContextHint = hint?.trim() ?? '';
}

export function setGenerationCanvasWidth(width: number): void {
  generationCanvasWidth = width > 0 ? width : 1200;
}

/** Expose the current canvas width for use by other modules (read-only). */
export function getGenerationCanvasWidth(): number {
  return generationCanvasWidth;
}

/** Expose the root frame ID for the current generation (read-only). */
export function getGenerationRootFrameId(): string {
  return generationRootFrameId;
}

/** Override the root frame ID — used by append-mode to reuse an existing page frame. */
export function setGenerationRootFrameId(id: string): void {
  generationRootFrameId = id;
}

/** Expose the current remapped IDs map for use by other modules (read-only). */
export function getGenerationRemappedIds(): Map<string, string> {
  return generationRemappedIds;
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
  const fills = 'fill' in node ? (node as { fill?: unknown }).fill : undefined;

  // Convert string shorthand (e.g. "#000000") to PenFill array
  if (typeof fills === 'string') {
    (node as unknown as Record<string, unknown>).fill = [{ type: 'solid', color: fills }];
    return;
  }

  if (!Array.isArray(fills)) return;

  // Convert any string elements in the array to solid fill objects
  for (let i = 0; i < fills.length; i++) {
    if (typeof fills[i] === 'string') {
      fills[i] = { type: 'solid', color: fills[i] };
    }
  }

  for (const fill of fills) {
    if (!fill || typeof fill !== 'object') continue;
    const f = fill as { type?: string; stops?: unknown[] };
    if ((f.type === 'linear_gradient' || f.type === 'radial_gradient') && Array.isArray(f.stops)) {
      const n = f.stops.length;
      f.stops = f.stops.map((s: unknown, i: number) => {
        const stop = s as Record<string, unknown>;
        let offset =
          typeof stop.offset === 'number' && Number.isFinite(stop.offset)
            ? stop.offset
            : typeof stop.position === 'number' && Number.isFinite(stop.position)
              ? (stop.position as number)
              : null;
        if (offset !== null && offset > 1) offset = offset / 100;
        return {
          color: typeof stop.color === 'string' ? stop.color : '#000000',
          offset: offset !== null ? Math.max(0, Math.min(1, offset)) : i / Math.max(n - 1, 1),
        };
      });
    }
  }
}

export function insertStreamingNode(node: PenNode, parentId: string | null): void {
  const { addNode, getNodeById } = useDocumentStore.getState();
  normalizeNodeFills(node);

  // Ensure unique node IDs to avoid collisions with pre-existing canvas content.
  // The upsert path already does this in sanitizeNodesForUpsert, but the streaming
  // path was missing it — causing duplicate Fabric objects when two generations
  // produce nodes with the same IDs (e.g. "header-title" in both FoodHome and Settings).
  const streamCounters = new Map<string, number>();
  const streamRemaps = new Map<string, string>();
  ensureUniqueNodeIds(node, preExistingNodeIds, streamCounters, streamRemaps);
  // Track the newly inserted IDs so subsequent streaming nodes don't collide either
  const trackNewIds = (n: PenNode) => {
    preExistingNodeIds.add(n.id);
    if ('children' in n && Array.isArray(n.children)) {
      for (const child of n.children) trackNewIds(child);
    }
  };
  trackNewIds(node);
  // Merge any remappings into the generation-wide remap table
  for (const [from, to] of streamRemaps) {
    generationRemappedIds.set(from, to);
  }

  // Ensure container nodes have children array for later child insertions
  if ((node.type === 'frame' || node.type === 'group') && !('children' in node)) {
    (node as PenNode & { children: PenNode[] }).children = [];
  }

  // Resolve remapped parent IDs (e.g., root frame -> DEFAULT_FRAME_ID)
  const resolvedParent = parentId ? (generationRemappedIds.get(parentId) ?? parentId) : null;

  const parentNode = resolvedParent ? getNodeById(resolvedParent) : null;

  if (parentNode && hasActiveLayout(parentNode) && !isBadgeOverlayNode(node)) {
    if ('x' in node) delete (node as { x?: number }).x;
    if ('y' in node) delete (node as { y?: number }).y;
    // Text defaults inside layout frames:
    // - vertical layout: body text prefers fill width for wrapping
    // - horizontal layout: short labels should hug content to avoid squeezing siblings
    if (node.type === 'text') {
      const parentLayout = 'layout' in parentNode ? parentNode.layout : undefined;
      const content = 'content' in node ? ((node.content as string) ?? '') : '';
      const isLongText = content.length > 15;

      if (parentLayout === 'vertical') {
        // Only force fill_container + fixed-width on LONG text that needs wrapping.
        // Short labels/titles/numbers should hug content width (auto).
        if (isLongText) {
          if (typeof node.width === 'number') node.width = 'fill_container';
          if (!node.textGrowth) node.textGrowth = 'fixed-width';
        } else {
          // Short text in vertical layout: fix pixel width but don't force wrapping
          if (typeof node.width === 'number') node.width = 'fill_container';
        }
      } else if (parentLayout === 'horizontal') {
        if (
          typeof node.width === 'string' &&
          node.width.startsWith('fill_container') &&
          !isLongText
        ) {
          node.width = 'fit_content';
        }
        if (
          !isLongText &&
          (!node.textGrowth ||
            node.textGrowth === 'fixed-width' ||
            node.textGrowth === 'fixed-width-height')
        ) {
          node.textGrowth = 'auto';
        }
      }
      // Respect AI's explicit textGrowth setting; don't override if already set.

      // Strip explicit pixel height on text nodes — always let the engine auto-size.
      // AI models often output height values that cause text clipping/overlap.
      if (typeof node.height === 'number' && node.textGrowth !== 'fixed-width-height') {
        delete (node as { height?: unknown }).height;
      }
      // Default lineHeight based on text role (heading vs body)
      if (!node.lineHeight) {
        node.lineHeight = defaultLineHeight(node.fontSize ?? 16);
      }
    }
  }

  // Apply role-based defaults before legacy heuristics.
  //
  // Theme detection for the streaming path: `detectActiveDocumentTheme`
  // walks the live page root via `getActivePagePrimaryFrameId()`. For
  // streaming, the page root frame is always committed to the store
  // BEFORE any of its children (it's the first node emitted by the
  // LLM and hits `replaceEmptyFrame` / `addNode` earlier in this same
  // function for the root case). By the time a child streaming node
  // reaches role resolution, the root is already in place and its fill
  // is readable — so the theme lookup is always accurate for children.
  //
  // For the root node itself, `detectActiveDocumentTheme` sees the
  // still-stale empty default root in the store (bad) UNLESS we also
  // check the incoming `node` — which we do by passing `[node]` as
  // the input-forest hint. If `node` has a solid fill (the LLM-supplied
  // dark page bg), input-first detection wins. Falls back to the store
  // cleanly for everything else.
  const roleCtx: RoleContext = {
    parentRole: parentNode?.role,
    parentLayout: parentNode && 'layout' in parentNode ? parentNode.layout : undefined,
    canvasWidth: generationCanvasWidth,
    theme: detectActiveDocumentTheme([node]),
  };
  resolveNodeRole(node, roleCtx);

  applyGenerationHeuristics(node);

  // Recursively remove x/y from children inside layout containers so the
  // layout engine can position them correctly during canvas sync.
  const parentHasLayout = parentNode ? hasActiveLayout(parentNode) : false;
  sanitizeLayoutChildPositions(node, parentHasLayout);

  // Skip AI-streamed children under phone placeholders. Placeholder internals are
  // normalized post-streaming (at most one centered label text is allowed).
  // Also skip if the parent node doesn't exist on canvas (was itself blocked).
  if (resolvedParent !== null && !parentNode) {
    return;
  }
  if (parentNode && isInsidePhonePlaceholder(resolvedParent!, getNodeById)) {
    return;
  }

  if (resolvedParent === null && node.type === 'frame') {
    if (isCanvasOnlyEmptyFrame()) {
      // Root frame replaces the active page's empty frame -- no animation
      // needed. replaceEmptyFrame returns the real target id (nanoid for
      // pages 2+, DEFAULT_FRAME_ID for page 1) so we can track it as the
      // generation root.
      const targetId = replaceEmptyFrame(node);
      if (targetId) generationRootFrameId = targetId;
    } else {
      // Canvas already has content — add as new top-level frame beside existing ones
      const { document: doc } = useDocumentStore.getState();
      const activePageId = useCanvasStore.getState().activePageId;
      const pageChildren = getActivePageChildren(doc, activePageId);
      let maxRight = 0;
      for (const child of pageChildren) {
        const cx = child.x ?? 0;
        const cw = 'width' in child && typeof child.width === 'number' ? child.width : 0;
        maxRight = Math.max(maxRight, cx + cw);
      }
      node.x = maxRight + 100;
      node.y = 0;
      generationRootFrameId = node.id;
      addNode(null, node);
    }
  } else {
    const effectiveParent = resolvedParent ?? generationRootFrameId;
    // Verify parent exists, fall back to generation root frame
    const parent = getNodeById(effectiveParent);
    const insertParent = parent ? effectiveParent : generationRootFrameId;

    // Frames with fills appear instantly (background context for children).
    // All other nodes fade in with staggered animation.
    const nodeFill = 'fill' in node ? node.fill : undefined;
    const hasFill = Array.isArray(nodeFill)
      ? nodeFill.length > 0
      : nodeFill != null && typeof nodeFill === 'object';
    const isBackgroundFrame = node.type === 'frame' && hasFill;
    if (!isBackgroundFrame) {
      pendingAnimationNodes.add(node.id);
      startNewAnimationBatch();
    }

    // Badge/overlay nodes prepend (index 0) so they render on top (earlier = higher z-order).
    // All other nodes append to preserve auto-layout generation order.
    addNode(insertParent, node, isBadgeOverlayNode(node) ? 0 : Infinity);

    // When a frame is inserted into a horizontal layout, equalize sibling card widths
    // to prevent overflow when multiple cards are placed in the same row.
    if (node.type === 'frame') {
      equalizeHorizontalSiblings(insertParent);
    }

    // When a top-level section is added directly under the generation root frame,
    // progressively expand root height to fit the new content.
    if (insertParent === generationRootFrameId) {
      expandRootFrameHeight();
    }
  }

  // Immediately enqueue image nodes for background search as they arrive
  if (node.type === 'image') {
    enqueueImageForSearch(node);
  }
}

// ---------------------------------------------------------------------------
// Canvas apply/upsert operations
// ---------------------------------------------------------------------------

/**
 * Run the page-root post-pass cleanups after a non-streaming apply path
 * inserts its nodes into the store. This mirrors part of what
 * applyPostStreamingTreeHeuristics does for the streaming path:
 * specifically, strip redundant "safe-dark" section fills that sub-agents
 * or external MCP callers hedge with on section roots (they hide the real
 * page background and break theming).
 *
 * OpenPencil documents are multi-page — only the FIRST page uses the
 * constant DEFAULT_FRAME_ID for its root frame. Pages added later via
 * addPage() receive a fresh nanoid, so we cannot look the page root up by
 * a well-known id. Instead we pull the active page's top-level children
 * and run stripRedundantSectionFills on every top-level frame we find.
 * In the common case that's a single page-root frame; edge cases with
 * multiple top-level frames on one page (comparison mockups, etc.) are
 * handled by iterating. Publishes once if any frame was modified.
 */
function finalizePageRootAfterApply(): void {
  const doc = useDocumentStore.getState().document;
  const activePageId = useCanvasStore.getState().activePageId;
  const topLevel = getActivePageChildren(doc, activePageId);
  if (!topLevel || topLevel.length === 0) return;

  let anyChanged = false;
  for (const node of topLevel) {
    if (node.type !== 'frame') continue;
    if (stripRedundantSectionFills(node)) {
      anyChanged = true;
    }
  }
  if (anyChanged) forcePageResync();
}

export function applyNodesToCanvas(nodes: PenNode[]): void {
  const { getFlatNodes } = useDocumentStore.getState();
  const existingIds = new Set(getFlatNodes().map((n) => n.id));
  const preparedNodes = sanitizeNodesForInsert(nodes, existingIds);

  // If canvas only has one empty frame, replace it with the generated content
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0]);
    finalizePageRootAfterApply();
    resolveAllPendingIcons().catch(console.warn);
    // Use the active page's primary frame id, NOT generationRootFrameId.
    // The latter is module-level state owned by the streaming path and
    // is stale here (module init value or leftover from a previous
    // streaming generation — on Page 2+ it would point at nothing on the
    // current page).
    const rootId = getActivePagePrimaryFrameId();
    if (rootId) scanAndFillImages(rootId).catch(() => {});
    return;
  }

  const { addNode } = useDocumentStore.getState();
  // Insert into the active page's root frame if it exists, otherwise at
  // document root. `getActivePagePrimaryFrameId` replaces the old
  // DEFAULT_FRAME_ID lookup which only worked on Page 1.
  const parentId = getActivePagePrimaryFrameId();
  for (const node of preparedNodes) {
    addNode(parentId, node, Infinity);
  }
  adjustRootFrameHeightToContent();
  finalizePageRootAfterApply();
  resolveAllPendingIcons().catch(console.warn);
  const rootId = getActivePagePrimaryFrameId();
  if (rootId) scanAndFillImages(rootId).catch(() => {});
}

export function upsertNodesToCanvas(nodes: PenNode[]): number {
  const preparedNodes = sanitizeNodesForUpsert(nodes);

  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0]);
    finalizePageRootAfterApply();
    return 1;
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState();
  const parentId = getActivePagePrimaryFrameId();
  let count = 0;

  for (const node of preparedNodes) {
    // Resolve remapped IDs (e.g., root frame that was mapped to DEFAULT_FRAME_ID in Phase 1)
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id;
    const existing = getNodeById(resolvedId);
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node;
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode);
      updateNode(resolvedId, merged);
    } else {
      addNode(parentId, node, Infinity);
    }
    count++;
  }

  adjustRootFrameHeightToContent();
  finalizePageRootAfterApply();
  // Use the active page's primary frame id, not the streaming path's
  // generationRootFrameId (which is stale for non-streaming applies —
  // see applyNodesToCanvas for the full explanation).
  const rootId = getActivePagePrimaryFrameId();
  if (rootId) scanAndFillImages(rootId).catch(() => {});
  return count;
}

/** Same as upsertNodesToCanvas but skips sanitization (caller already did it). */
function upsertPreparedNodes(preparedNodes: PenNode[]): number {
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0]);
    finalizePageRootAfterApply();
    return 1;
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState();
  const parentId = getActivePagePrimaryFrameId();
  let count = 0;

  for (const node of preparedNodes) {
    // Resolve remapped IDs (e.g., root frame that was mapped to DEFAULT_FRAME_ID in Phase 1)
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id;
    const existing = getNodeById(resolvedId);
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node;
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode);
      updateNode(resolvedId, merged);
    } else {
      addNode(parentId, node, Infinity);
    }
    count++;
  }

  adjustRootFrameHeightToContent();
  finalizePageRootAfterApply();
  return count;
}

/**
 * Animate nodes onto the canvas with a staggered fade-in effect.
 * Synchronous -- nodes are inserted immediately, and canvas-sync
 * schedules fire-and-forget staggered opacity animations.
 */
export function animateNodesToCanvas(nodes: PenNode[]): void {
  resetGenerationRemapping();
  resetAnimationState();
  const prepared = sanitizeNodesForUpsert(nodes);
  startNewAnimationBatch();
  markNodesForAnimation(prepared);

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
  upsertPreparedNodes(prepared);
  useHistoryStore.getState().endBatch(useDocumentStore.getState().document);

  // Resolve any icons queued for async (brand logos etc.) after nodes are in the store
  resolveAllPendingIcons().catch(console.warn);
  // Scan images on the active page root. generationRootFrameId is refreshed
  // by resetGenerationRemapping above, but going straight through
  // getActivePagePrimaryFrameId keeps the source of truth consistent with
  // the other non-streaming apply paths and doesn't rely on a specific
  // ordering between reset and this call.
  const rootId = getActivePagePrimaryFrameId();
  if (rootId) scanAndFillImages(rootId).catch(() => {});
}

// ---------------------------------------------------------------------------
// Extract + apply convenience wrappers
// ---------------------------------------------------------------------------

/**
 * Extract PenNode JSON from AI response text and apply to canvas.
 * Returns the number of top-level elements added (0 if nothing found/applied).
 */
export function extractAndApplyDesign(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText);
  if (!nodes || nodes.length === 0) return 0;

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
  try {
    applyNodesToCanvas(nodes);
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
  }
  return nodes.length;
}

/**
 * Extract PenNode JSON from AI response text and apply updates/insertions to canvas.
 * Handles both new nodes and modifications (matching by ID).
 */
export function extractAndApplyDesignModification(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText);
  if (!nodes || nodes.length === 0) return 0;

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState();
  let count = 0;

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
  try {
    for (const node of nodes) {
      const existing = getNodeById(node.id);
      if (existing) {
        // Update existing node
        updateNode(node.id, node);
        count++;
      } else {
        // It's a new node implied by the modification (e.g. "add a button").
        // Parent it to the active page's root frame, whichever page we're
        // on — not just the Page 1 constant.
        const parentId = getActivePagePrimaryFrameId();
        addNode(parentId, node);
        count++;
      }
    }
    finalizePageRootAfterApply();
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
  }
  return count;
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
  // Skip pre-injected chrome (e.g. iPhone status bar) — its path data is
  // hardcoded from the Pencil demo and must not be overwritten by icon resolver.
  if ('role' in node && (node as { role?: string }).role === 'status-bar') return;

  // Default icon_font nodes to lucide family when unspecified
  if (node.type === 'icon_font' && !node.iconFontFamily) {
    node.iconFontFamily = 'lucide';
  }

  applyIconPathResolution(node);
  applyNoEmojiIconHeuristic(node);
  // Re-run icon resolution on nodes converted from emoji text → path by the
  // heuristic above. applyNoEmojiIconHeuristic sets a circle fallback path;
  // the icon resolver can often match the name (e.g. "Pizza Emoji Path" → pizza).
  if (node.type === 'path') {
    applyIconPathResolution(node);
  }
  applyImagePlaceholderHeuristic(node);

  if (!('children' in node) || !Array.isArray(node.children)) return;
  for (const child of node.children) {
    applyGenerationHeuristics(child);
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
  const { getNodeById, updateNode } = useDocumentStore.getState();
  const rootNode = getNodeById(rootNodeId);
  if (!rootNode || rootNode.type !== 'frame') return;
  if (!Array.isArray(rootNode.children) || rootNode.children.length === 0) return;

  // Schema-level normalization runs first: unwrap array-wrapped strokes,
  // migrate fill-shaped stroke objects to proper PenStroke, drop illegal
  // "none"/"transparent" CSS keyword fills. Sub-agents break these
  // constraints constantly and downstream passes assume valid shapes.
  normalizeStrokeFillSchema(rootNode);

  // Earliest pass: strip fake phone mockup wrappers that weaker sub-agents
  // generate when they misread the prompt's phone mockup guidance. Must run
  // BEFORE resolveTreeRoles, otherwise the role resolver may write defaults
  // (layout, fill) onto the wrapper and the children inside it that we then
  // throw away.
  // Return value is intentionally ignored — see the publish step at the end:
  // we always publish, so the boolean would only be informational.
  unwrapFakePhoneMockups(rootNode);

  // Role-based tree resolution + cross-node post-pass.
  // Runs FIRST so role defaults (e.g. navbar → horizontal, button → horizontal)
  // can populate missing `layout` fields with semantically correct values.
  resolveTreeRoles(rootNode, generationCanvasWidth);
  resolveTreePostPass(rootNode, generationCanvasWidth, getNodeById, updateNode);

  // Re-fetch the root from the store before running any subsequent pass.
  // resolveTreePostPass calls `updateNode` in several places (height
  // expansion, clipContent). Each call routes through `updateNodeInTree`,
  // which shallow-clones every ancestor along the update path. Our original
  // `rootNode` reference now points to a detached tree: further mutations on
  // it would silently disappear for nodes that lived on those update paths.
  // Always read a fresh reference for mutation passes that follow updateNode.
  const freshRoot = useDocumentStore.getState().getNodeById(rootNodeId);
  if (!freshRoot || freshRoot.type !== 'frame') return;

  // Normalize layout as a final safety net: fills in `layout` for frames the
  // role resolver did not touch (unknown roles, plain containers) and strips
  // stale x/y from children of any auto-layout frame. MUST run AFTER role
  // resolution — otherwise the 'vertical' fallback here freezes wrong layouts
  // before role defaults can override them.
  normalizeTreeLayout(freshRoot);

  // Strip redundant section-level fills. Weaker sub-agents hedge by
  // hardcoding a "safe dark" hex (e.g. #0A0A0A) on every section root they
  // emit, which then completely covers the page root's intended background
  // and breaks theme switching. This pass drops those redundant fills while
  // preserving cards/buttons/badges. Must run AFTER role resolution so we
  // can tell section containers apart from card/button/chip components by
  // their resolved role.
  //
  // IMPORTANT: stripRedundantSectionFills must ONLY be called on the true
  // page root frame. Calling it on an arbitrary sub-agent root (or any
  // non-root nested frame) is wrong — the nested frame's direct children
  // are components, not "sections", and stripping their fills would
  // clobber intended visual styling (e.g. a card's own dark header).
  //
  // The page root is:
  //   - `parentOfRoot` when the sub-agent's root was inserted as a child of
  //     an existing page frame (the common case for a multi-section plan)
  //   - `freshRoot` itself when the sub-agent's root IS the page frame
  //     (replaceEmptyFrame remap, or a single-sub-agent page)
  // Pick exactly one — never both.
  const parentOfRoot = useDocumentStore.getState().getParentOf(rootNodeId);
  const pageRoot = parentOfRoot && parentOfRoot.type === 'frame' ? parentOfRoot : freshRoot;
  stripRedundantSectionFills(pageRoot);

  // Publish point. unwrap, resolveTreeRoles, and normalizeTreeLayout all
  // mutate store-owned nodes in place; resolveTreePostPass mostly goes
  // through updateNode but also has direct-mutation branches. Without an
  // explicit publish, Zustand subscribers (canvas sync, MCP push) only fire
  // if some later code path happens to call updateNode — and that path is
  // skipped on sub-agent retry / no-op cases.
  //
  // We use forcePageResync (not a hand-rolled shallow doc spread) because
  // canvas-document-sync subscribes to the active page's children array
  // identity, not to the document object itself. A naive `{ ...document }`
  // spread would NOT change `pages[0].children` and the canvas would never
  // re-sync — see canvas-sync-utils.ts header comment for the trap.
  // forcePageResync also bypasses mutateWithHistory so we don't push an
  // undo entry for what is a deterministic post-streaming cleanup.
  forcePageResync();

  // Resolve pending icons asynchronously via Iconify API (fire-and-forget)
  resolveAsyncIcons(rootNodeId).catch(console.warn);
}

// ---------------------------------------------------------------------------
// Root frame height management
// ---------------------------------------------------------------------------

export function adjustRootFrameHeightToContent(frameId?: string): void {
  const { getNodeById, updateNode, getParentOf } = useDocumentStore.getState();
  // Prefer the explicitly-passed frame, then the active page's primary
  // frame (the correct default for non-streaming apply paths, which is
  // where this function is called from), and finally the streaming
  // path's generationRootFrameId as a last resort.
  const rootId = frameId ?? getActivePagePrimaryFrameId() ?? generationRootFrameId;
  if (!rootId) return;
  const root = getNodeById(rootId);
  if (!root || root.type !== 'frame') return;
  if (!Array.isArray(root.children) || root.children.length === 0) return;

  const measurableRoot = { ...root, height: 0 } as typeof root;
  const requiredHeight = estimateNodeIntrinsicHeight(measurableRoot);
  const minimumHeight = getParentOf(rootId) ? 0 : 320;
  const targetHeight = Math.max(minimumHeight, Math.round(requiredHeight));
  const currentHeight = toSizeNumber(root.height, 0);
  if (Math.abs(currentHeight - targetHeight) < 8) return;

  updateNode(rootId, { height: targetHeight });
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
export function expandRootFrameHeight(frameId?: string): void {
  const { getNodeById, updateNode, getParentOf } = useDocumentStore.getState();
  const rootId = frameId ?? generationRootFrameId;
  const root = getNodeById(rootId);
  if (!root || root.type !== 'frame') return;
  if (!Array.isArray(root.children) || root.children.length === 0) return;

  const measurableRoot = { ...root, height: 0 } as typeof root;
  const requiredHeight = estimateNodeIntrinsicHeight(measurableRoot);
  const minimumHeight = getParentOf(rootId) ? 0 : 320;
  const targetHeight = Math.max(minimumHeight, Math.round(requiredHeight));
  const currentHeight = toSizeNumber(root.height, 0);
  // Only grow -- never shrink during progressive generation
  if (currentHeight > 0 && targetHeight <= currentHeight) return;

  updateNode(rootId, { height: targetHeight });
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Check if the active page has exactly one top-level frame and that frame
 * has no children yet. Used to decide whether an incoming batch/streaming
 * insert should REPLACE the empty boilerplate frame created by addPage()
 * vs. append new content beside it.
 *
 * Previously this hardcoded DEFAULT_FRAME_ID, which broke on every page
 * after the first: addPage() gives new pages a nanoid-based root frame id,
 * so the check was `false` on Page 2+ and the replace branch never fired.
 * The check now looks at the actual top-level frame of the active page,
 * whatever its id happens to be.
 */
function isCanvasOnlyEmptyFrame(): boolean {
  const { document } = useDocumentStore.getState();
  const activePageId = useCanvasStore.getState().activePageId;
  const pageChildren = getActivePageChildren(document, activePageId);
  if (pageChildren.length !== 1) return false;
  const only = pageChildren[0];
  if (only.type !== 'frame') return false;
  return !('children' in only) || !only.children || only.children.length === 0;
}

/**
 * Replace the active page's empty root frame with the generated frame
 * node, preserving the existing frame id so canvas sync continues to
 * work. Returns the id of the frame that was updated, or null if the
 * active page has no frame to replace (caller should have gated this
 * call on isCanvasOnlyEmptyFrame).
 *
 * Previously this hardcoded DEFAULT_FRAME_ID as the update target, which
 * meant calling replaceEmptyFrame on Page 2+ would silently modify
 * Page 1's root frame instead of the page the user was actually editing.
 */
function replaceEmptyFrame(generatedFrame: PenNode): string | null {
  const targetId = getActivePagePrimaryFrameId();
  if (!targetId) return null;
  const { updateNode } = useDocumentStore.getState();
  // Record the remapping so subsequent phases can find this node by its original ID
  generationRemappedIds.set(generatedFrame.id, targetId);
  // Keep root frame ID and position (x=0, y=0), take everything else from generated frame
  const { id: _id, x: _x, y: _y, ...rest } = generatedFrame;
  updateNode(targetId, rest);
  return targetId;
}

function equalizeHorizontalSiblings(parentId: string): void {
  const { getNodeById, updateNode } = useDocumentStore.getState();
  const parent = getNodeById(parentId);
  if (!parent || parent.type !== 'frame') return;
  if (parent.layout !== 'horizontal') return;
  if (!Array.isArray(parent.children) || parent.children.length < 2) return;

  // Skip if any card already uses fill_container -- the AI chose it deliberately
  const cardCandidates = parent.children.filter(
    (c) =>
      c.type === 'frame' &&
      c.role !== 'phone-mockup' &&
      c.role !== 'divider' &&
      c.role !== 'badge' &&
      c.role !== 'pill' &&
      c.role !== 'tag' &&
      toSizeNumber('height' in c ? c.height : undefined, 0) > 88,
  );
  if (cardCandidates.some((c) => 'width' in c && c.width === 'fill_container')) return;

  const fixedFrames = cardCandidates.filter(
    (c) => 'width' in c && typeof c.width === 'number' && (c.width as number) > 0,
  );
  if (fixedFrames.length < 2) return;

  // Only equalize when widths vary significantly (ratio < 0.6)
  const widths = fixedFrames.map((c) => toSizeNumber('width' in c ? c.width : undefined, 0));
  const maxW = Math.max(...widths);
  const minW = Math.min(...widths);
  if (maxW <= 0 || minW / maxW >= 0.6) return;

  // Check if they look like a card row (similar heights)
  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0));
  const maxH = Math.max(...heights);
  const minH = Math.min(...heights);
  if (maxH <= 0 || minH / maxH <= 0.5) return;

  // Convert to fill_container for even distribution and equal height
  for (const child of fixedFrames) {
    updateNode(child.id, { width: 'fill_container', height: 'fill_container' } as Partial<PenNode>);
  }
}

function applyImagePlaceholderHeuristic(node: PenNode): void {
  if (node.type !== 'image') return;

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase();
  const contextMarker = generationContextHint.toLowerCase();
  const contextualScreenshotHint = /(截图|screenshot|mockup|手机|app[-_\s]*screen)/.test(
    contextMarker,
  );
  const screenshotLike =
    isScreenshotLikeMarker(marker) ||
    (contextualScreenshotHint && /(preview|hero|showcase|phone|screen)/.test(marker));
  if (!screenshotLike) return;

  const width = toSizeNumber(node.width, 360);
  const height = toSizeNumber(node.height, 720);
  // Detect dark/light from context hint (dark if mentions dark/terminal/cyber/night)
  const dark = !/(light|bright)/.test(generationContextHint.toLowerCase());
  node.src = createPhonePlaceholderDataUri(width, height, dark);
  if (node.cornerRadius === undefined) {
    node.cornerRadius = 24;
  }
}

function isScreenshotLikeMarker(text: string): boolean {
  return /app[-_\s]*screen|screenshot|mockup|phone|mobile|device|截图|手机/.test(text);
}

// ---------------------------------------------------------------------------
// Node sanitization for insert/upsert
// ---------------------------------------------------------------------------

/**
 * Resolve the theme that should drive role defaults for an incoming
 * batch of nodes.
 *
 * Detection order — INPUT NODES FIRST, then live store:
 *
 *   1. Walk the incoming `nodes` array top-down. The first frame at
 *      depth 0 (outermost) with a solid-color fill wins. If none of
 *      the outermost nodes has a fill, walk one level deeper, and so
 *      on. The first hit is the theme source.
 *
 *      Why input first: in a fresh generation the LLM emits the new
 *      page root (e.g. fill #0A0A0A) inside `nodes`, but the LIVE
 *      store still holds the previous empty default root. Reading
 *      the store would return 'light' from that empty default, and
 *      the LLM-supplied dark page would get white card defaults
 *      injected into every child before the new root reaches the
 *      store. Reading the input first guarantees the cards see the
 *      same theme as the page they belong to.
 *
 *   2. Fall back to the LIVE active-page primary frame in the store
 *      via `getActivePagePrimaryFrameId()`. This handles partial
 *      inserts (e.g. dropping a single navbar into an existing dark
 *      page where `nodes` doesn't carry the page root).
 *
 *      Always reads via `getActivePagePrimaryFrameId()` rather than
 *      the cached `generationRootFrameId` module variable. The cache
 *      is set by `resetGenerationRemapping()` at the start of an
 *      orchestrator generation flow but is stale or default for
 *      direct MCP call paths (`insert_node`, `batch_design`,
 *      `upsertNodesToCanvas` from non-streaming code) that bypass
 *      that initialization. The same precedent exists at line ~464
 *      of this file: `upsertNodesToCanvas` already reads
 *      `getActivePagePrimaryFrameId()` for the same reason.
 *
 *   3. Returns `undefined` when neither source has a usable fill
 *      (brand-new document, partial insert into empty page) —
 *      callers should treat that the same as the default light theme.
 */
function detectActiveDocumentTheme(nodes?: PenNode[]): 'dark' | 'light' | undefined {
  if (nodes && nodes.length > 0) {
    const fromInput = detectThemeFromNodeForest(nodes);
    if (fromInput) return fromInput;
  }

  const primaryFrameId = getActivePagePrimaryFrameId();
  if (!primaryFrameId) return undefined;
  const pageRoot = useDocumentStore.getState().getNodeById(primaryFrameId);
  if (!pageRoot) return undefined;
  return detectThemeFromNode(pageRoot);
}

/**
 * BFS over a forest of nodes, returning the theme detected from the
 * first frame with a usable solid fill. Returns `undefined` if no
 * frame in the entire forest carries a fill we can read.
 *
 * BFS (not DFS) so the OUTERMOST frames are visited first — the page
 * root and top-level sections are the most authoritative theme
 * source. A small white card nested deep inside a dark page must not
 * out-vote the page root.
 */
function detectThemeFromNodeForest(nodes: PenNode[]): 'dark' | 'light' | undefined {
  const queue: PenNode[] = [...nodes];
  while (queue.length > 0) {
    const node = queue.shift()!;
    if (node.type === 'frame') {
      const theme = readThemeFromNodeFill(node);
      if (theme) return theme;
    }
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) queue.push(child);
    }
  }
  return undefined;
}

/**
 * Read theme from a single node's fill if it has a parseable solid
 * color. Returns `undefined` for missing fill, empty fill, gradient,
 * variable ref, or any other unreadable shape — caller must keep
 * walking the tree.
 *
 * Mirrors `detectThemeFromNode` from role-resolver but returns
 * undefined (not 'light') when the fill is unreadable, so the caller
 * can distinguish "no fill found, keep looking" from "explicit light".
 */
function readThemeFromNodeFill(node: PenNode): 'dark' | 'light' | undefined {
  const fill = (node as { fill?: unknown }).fill;
  if (!Array.isArray(fill) || fill.length === 0) return undefined;
  const first = fill[0] as { type?: string; color?: string };
  if (first?.type !== 'solid' || typeof first.color !== 'string') return undefined;
  return detectThemeFromNode(node);
}

function sanitizeNodesForInsert(nodes: PenNode[], existingIds: Set<string>): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n));
  const activeTheme = detectActiveDocumentTheme(cloned);

  for (const node of cloned) {
    // Schema normalization first so later passes see valid stroke/fill
    // shapes (unwrap stroke arrays, migrate fill-shaped strokes, drop
    // CSS-keyword fill colors).
    normalizeStrokeFillSchema(node);
    // Strip fake phone mockup wrappers BEFORE role resolution so role
    // defaults aren't wasted on a wrapper we're about to discard.
    unwrapFakePhoneMockups(node);
    // Rewrite known LLM composition anti-patterns BEFORE role resolution
    // so the rewritten subtree still benefits from theme-aware defaults,
    // layout normalization, and post-pass fixes. Covers stacked-ellipse
    // progress rings rendering as overlapping top-left blobs, and
    // alternating bar/label siblings that break chart column layouts.
    rewriteLlmAntiPatterns(node);
    // Role resolution runs first so role defaults can populate `layout`
    // before normalizeTreeLayout's generic fallback would otherwise freeze
    // the wrong value (e.g. navbar → horizontal, not vertical fallback).
    //
    // `activeTheme` is detected from the LIVE page root (not from `node`)
    // because `node` here is an arbitrary subtree without its own fill —
    // a card or navbar that omitted fill expecting the dark page bg to
    // show through. Without this, theme detection would fall back to
    // 'light' and paint a white default on top of a dark page.
    resolveTreeRoles(
      node,
      generationCanvasWidth,
      undefined,
      undefined,
      undefined,
      false,
      activeTheme,
    );
    applyGenerationHeuristics(node);
    normalizeTreeLayout(node);
    // Intentionally NOT calling stripRedundantSectionFills here: `cloned`
    // is an arbitrary PenNode from MCP/batch APIs (could be a card, a
    // component, or a page). strip must only run on the true page root
    // frame, which this path cannot guarantee.
    sanitizeLayoutChildPositions(node, false);
    sanitizeScreenFrameBounds(node);
  }

  const counters = new Map<string, number>();
  const used = new Set(existingIds);
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters);
  }

  return cloned;
}

function sanitizeNodesForUpsert(nodes: PenNode[]): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n));
  const activeTheme = detectActiveDocumentTheme(cloned);

  for (const node of cloned) {
    // Schema normalization first so later passes see valid stroke/fill
    // shapes (unwrap stroke arrays, migrate fill-shaped strokes, drop
    // CSS-keyword fill colors).
    normalizeStrokeFillSchema(node);
    // Strip fake phone mockup wrappers BEFORE role resolution so role
    // defaults aren't wasted on a wrapper we're about to discard.
    unwrapFakePhoneMockups(node);
    // Rewrite known LLM composition anti-patterns BEFORE role resolution.
    // See sanitizeNodesForInsert for the full rationale.
    rewriteLlmAntiPatterns(node);
    // Role resolution runs first so role defaults can populate `layout`
    // before normalizeTreeLayout's generic fallback would otherwise freeze
    // the wrong value (e.g. navbar → horizontal, not vertical fallback).
    // See sanitizeNodesForInsert for the activeTheme rationale.
    resolveTreeRoles(
      node,
      generationCanvasWidth,
      undefined,
      undefined,
      undefined,
      false,
      activeTheme,
    );
    applyGenerationHeuristics(node);
    normalizeTreeLayout(node);
    // Intentionally NOT calling stripRedundantSectionFills here: `cloned`
    // is an arbitrary PenNode from MCP/batch APIs (could be a card, a
    // component, or a page). strip must only run on the true page root
    // frame, which this path cannot guarantee.
    sanitizeLayoutChildPositions(node, false);
    sanitizeScreenFrameBounds(node);
  }

  // Start with pre-existing node IDs to avoid collisions with content
  // that was on canvas before this generation started. IDs generated
  // within the current batch are also tracked so siblings stay unique.
  // Record remappings so progressive upsert can resolve renamed IDs.
  const counters = new Map<string, number>();
  const used = new Set(preExistingNodeIds);
  const newRemaps = new Map<string, string>();
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters, newRemaps);
  }

  // Merge new remappings into the generation-wide remap table
  for (const [from, to] of newRemaps) {
    generationRemappedIds.set(from, to);
  }

  return cloned;
}

/** Check if a node (by ID) is inside a Phone Placeholder frame (any ancestor). */
function isInsidePhonePlaceholder(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
): boolean {
  let current = getNodeById(nodeId);
  while (current) {
    if (current.name === 'Phone Placeholder') return true;
    const parent = useDocumentStore.getState().getParentOf(current.id);
    if (!parent) break;
    current = parent;
  }
  return false;
}
