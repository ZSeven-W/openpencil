import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import { forcePageResync } from '@/canvas/canvas-sync-utils';
import type { PenNode, ImageNode } from '@/types/pen';

export function inferAspectRatio(node: PenNode): 'wide' | 'tall' | 'square' | undefined {
  const n = node as unknown as Record<string, unknown>;
  const w = typeof n['width'] === 'number' ? (n['width'] as number) : 0;
  const h = typeof n['height'] === 'number' ? (n['height'] as number) : 0;
  if (!w || !h) return undefined;
  const ratio = w / h;
  if (ratio > 1.3) return 'wide';
  if (ratio < 0.77) return 'tall';
  return 'square';
}

/**
 * `add_image_placeholder_v0` / `_v1` element tools emit a `frame` (not an
 * `image` node) carrying `role: 'image-placeholder'` plus an icon_font (and
 * optional label) child. The auto-search pipeline used to filter exclusively
 * on `type === 'image'`, so every placeholder produced via element tools or
 * a JSONL `<op_tool>{name:"add_image_placeholder_…"}` payload silently
 * bypassed the search hook — designs landed with all dashed-border icons
 * instead of real photos.
 */
export function isImagePlaceholderFrame(node: PenNode | undefined | null): boolean {
  if (!node || node.type !== 'frame') return false;
  return (node as PenNode & { role?: string }).role === 'image-placeholder';
}

/**
 * Heuristic detector for "looks like an image area" frames the model
 * emitted WITHOUT the canonical `role: 'image-placeholder'`. Common
 * pattern: restaurant / product / event card whose top is a wide solid
 * coloured frame named "Image" / "Photo" / "Cover" / "Hero" / "Thumbnail"
 * — the model uses raw frame nodes instead of `add_image_placeholder_v1`,
 * so the strict isImagePlaceholderFrame check misses them and the auto-
 * search pipeline never fires. Result: cards land on canvas with empty
 * coloured rectangles where photos should be (the "Bella Italia" card
 * bug from the 2026-05-09 user report).
 *
 * Conservative match policy:
 *   - frame node, has a name matching IMAGE_AREA_NAME_RE
 *   - has a single solid (non-image) fill — not a gradient, not already
 *     filled with type:'image'
 *   - has 0 or 1 children (an icon-only "broken image" child is OK; a
 *     content-rich frame is NOT a placeholder)
 *   - aspect ratio: width is a finite number >= 80 AND height >= 60
 *     (filters out 1px borders, tiny color swatches, dividers)
 *
 * False-positive risk: a real solid-colour decorative block named
 * "Hero" or "Cover" gets searched and overwritten with a photo. Mitigated
 * by the name regex requiring an image / photo / cover / hero / thumbnail
 * keyword — purely decorative blocks are rarely named this way. Test
 * coverage in image-search-pipeline.test.ts protects the boundary.
 */
const IMAGE_AREA_NAME_RE = /\b(image|photo|cover|hero|thumbnail|thumb|picture|banner|poster)\b/i;

export function isImageAreaFrameByHeuristic(node: PenNode | undefined | null): boolean {
  if (!node || node.type !== 'frame') return false;
  // Already a canonical placeholder — handled by the strict path; skip
  // here to avoid double-counting.
  if ((node as PenNode & { role?: string }).role === 'image-placeholder') return false;
  const name = (node as PenNode & { name?: string }).name;
  if (typeof name !== 'string' || !IMAGE_AREA_NAME_RE.test(name)) return false;
  // Width/height shape — skip non-image-shaped blocks.
  const w = (node as PenNode & { width?: unknown }).width;
  const h = (node as PenNode & { height?: unknown }).height;
  if (typeof w !== 'number' || w < 80) return false;
  if (typeof h !== 'number' || h < 60) return false;
  // Single solid (non-image) fill only.
  const fill = (node as PenNode & { fill?: unknown }).fill;
  if (!Array.isArray(fill) || fill.length !== 1) return false;
  const first = fill[0] as { type?: unknown } | undefined;
  if (first?.type === 'image') return false; // already filled
  if (first?.type !== 'solid') return false; // gradient = decorative, skip
  // Children check — at most one (icon hint) child. A frame with full
  // sub-content isn't a placeholder.
  const children = (node as PenNode & { children?: unknown[] }).children;
  if (Array.isArray(children) && children.length > 1) return false;
  return true;
}

/**
 * True when the frame is a placeholder AND still carries the gray
 * solid fill (i.e. has not yet been filled by a previous scan). The
 * filled-frame flag we leave on the node is the fill itself: once a
 * scan succeeds it becomes `[{type:'image', url, mode:'crop'}]`. We
 * INTENTIONALLY don't strip `role: 'image-placeholder'` on fill — the
 * role is what makes "this frame is meant to hold a photo" semantics
 * survive into history, codegen, and downstream tooling — but we do
 * gate every search-pipeline read on the fill check so a future scan
 * (e.g. from a follow-up generation that resets queuedNodeIds and
 * re-walks the tree) doesn't re-fetch and replace an already-good photo.
 */
export function isUnfilledImagePlaceholderFrame(node: PenNode | undefined | null): boolean {
  if (!isImagePlaceholderFrame(node)) return false;
  const fill = (node as PenNode & { fill?: unknown }).fill;
  if (!Array.isArray(fill) || fill.length === 0) return true;
  const first = fill[0] as { type?: unknown } | undefined;
  // Already painted with an image fill = filled, skip.
  return first?.type !== 'image';
}

interface ImageSearchTarget {
  node: PenNode;
  kind: 'image' | 'placeholder-frame';
}

export function collectImageSearchTargets(rootId: string): ImageSearchTarget[] {
  const { getNodeById } = useDocumentStore.getState();
  const root = getNodeById(rootId);
  if (!root) return [];

  const targets: ImageSearchTarget[] = [];
  const walk = (node: PenNode) => {
    if (node.type === 'image') {
      if (isPlaceholderSrc((node as ImageNode).src)) targets.push({ node, kind: 'image' });
    } else if (isImagePlaceholderFrame(node)) {
      // Only enqueue if not already painted with an image fill — see
      // `isUnfilledImagePlaceholderFrame` for the rationale on keeping the
      // role around after fill. Either way, don't descend into children:
      // unfilled placeholder children (icon_font + label) get wiped on
      // fill, and filled placeholders don't have children anymore.
      if (isUnfilledImagePlaceholderFrame(node)) {
        targets.push({ node, kind: 'placeholder-frame' });
      }
      return;
    } else if (isImageAreaFrameByHeuristic(node)) {
      // Heuristic match — model emitted a "looks like an image area"
      // frame without the canonical role. Treat as placeholder-frame so
      // the search pipeline replaces its solid fill with a real photo.
      targets.push({ node, kind: 'placeholder-frame' });
      return;
    }
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) walk(child);
    }
  };
  walk(root);
  return targets;
}

// Only match the known phone placeholder prefix, not user-uploaded SVGs
const PHONE_PLACEHOLDER_PREFIX = 'data:image/svg+xml;charset=utf-8,%3Csvg';

function isPlaceholderSrc(src?: string): boolean {
  return !src || src.startsWith(PHONE_PLACEHOLDER_PREFIX);
}

/**
 * Names so generic that they tell the photo search API nothing useful.
 * For these, prefer mining context (parent frame name, sibling text)
 * over returning the name itself.
 */
const GENERIC_PLACEHOLDER_NAMES = new Set([
  'image',
  'photo',
  'cover',
  'hero',
  'thumbnail',
  'thumb',
  'picture',
  'banner',
  'poster',
  'image placeholder',
  'placeholder icon',
  'placeholder',
  'card image',
  'card photo',
  'product image',
  'item image',
]);

function isGenericPlaceholderName(name: string): boolean {
  return GENERIC_PLACEHOLDER_NAMES.has(name.trim().toLowerCase());
}

/**
 * Walk up to find a parent frame whose name carries product / restaurant
 * / event semantic. The image-search API hits much more useful results
 * with "Bella Italia" / "Margherita Pizza" than with the generic
 * "Image" name the model gave the placeholder. Bounded to 3 hops so a
 * deep page bg doesn't end up as the query.
 */
export function findParentSemanticName(nodeId: string, maxHops = 3): string | null {
  const { document: doc } = useDocumentStore.getState();
  // Build a parent map by walking the doc tree once. Cheap for typical
  // designs (< few hundred nodes) and avoids passing parent through
  // every collectImageSearchTargets / enqueue call site.
  const parentOf = new Map<string, PenNode>();
  const walk = (n: PenNode): void => {
    if ('children' in n && Array.isArray(n.children)) {
      for (const c of n.children) {
        parentOf.set(c.id, n);
        walk(c);
      }
    }
  };
  const roots = doc.pages?.flatMap((p) => p.children ?? []) ?? doc.children ?? [];
  for (const r of roots) walk(r);
  let cur = parentOf.get(nodeId);
  let hops = 0;
  while (cur && hops < maxHops) {
    const name = (cur as PenNode & { name?: string }).name;
    if (typeof name === 'string' && name.length > 0 && !isGenericPlaceholderName(name)) {
      // Filter common layout words so we don't end up searching for
      // "Card" / "Wrapper" — neither yields useful photos.
      const lower = name.toLowerCase();
      if (
        !/\b(card|wrapper|container|section|frame|root|page|stack|row|column|content)\b/.test(lower)
      ) {
        return name;
      }
    }
    cur = parentOf.get(cur.id);
    hops++;
  }
  return null;
}

export function extractQueryForNode(node: PenNode): string {
  const r = node as PenNode & {
    imageSearchQuery?: string;
    name?: string;
    children?: PenNode[];
  };
  if (typeof r.imageSearchQuery === 'string' && r.imageSearchQuery.length > 0) {
    return r.imageSearchQuery;
  }
  // For placeholder frames, mine the optional label child for a hint
  // (e.g. "Hero image" / "Upload cover" — set by the caller).
  if (isImagePlaceholderFrame(node) && Array.isArray(r.children)) {
    for (const ch of r.children) {
      if (
        ch.type === 'text' &&
        (ch as PenNode & { role?: string }).role === 'image-placeholder-label'
      ) {
        const content = (ch as PenNode & { content?: string }).content;
        if (typeof content === 'string' && content.length > 0) return content;
      }
    }
  }
  // If the node's name is too generic to make a useful photo query
  // (literal "Image" / "Photo" / "Cover" — common with the heuristic
  // detector), walk up to find a semantic parent name (e.g. "Bella
  // Italia" or "Margherita Pizza" from the food-app card scenario).
  if (typeof r.name === 'string' && r.name.length > 0 && !isGenericPlaceholderName(r.name)) {
    return r.name;
  }
  const parentName = findParentSemanticName(node.id);
  if (parentName) return parentName;
  return r.name ?? 'placeholder';
}

// ---------------------------------------------------------------------------
// Incremental queue-based image search
// ---------------------------------------------------------------------------

interface QueuedImage {
  id: string;
  query: string;
  aspect: 'wide' | 'tall' | 'square' | undefined;
  kind: 'image' | 'placeholder-frame';
}

const imageSearchQueue: QueuedImage[] = [];
// Track IDs already queued or processed to avoid duplicates
const queuedNodeIds = new Set<string>();
let queueProcessing = false;
let queueAbort: AbortController | null = null;

/**
 * Enqueue an image target for background search. Accepts:
 *   - real `image` node with a placeholder src
 *   - frame carrying `role: 'image-placeholder'` (canonical, from
 *     add_image_placeholder_v0/v1)
 *   - frame matching the isImageAreaFrameByHeuristic predicate (a
 *     non-canonical placeholder the model emitted as a plain colored
 *     "Image" / "Photo" / "Cover" / "Hero" frame — see the comment on
 *     that function for the full match policy)
 *
 * Called from insertStreamingNode for streamed image nodes, and from
 * `scanAndFillImages` for all shapes after a non-streaming insert (the
 * orchestrator-tail and per-subtask scans). Streaming intentionally
 * skips placeholder frames because their icon/label children stream in
 * separately — enqueueing the frame mid-stream and replacing children
 * before the icon arrives would race with the late-arriving child.
 */
export function enqueueImageForSearch(node: PenNode): void {
  let kind: QueuedImage['kind'];
  if (node.type === 'image') {
    if (!isPlaceholderSrc((node as ImageNode).src)) return;
    kind = 'image';
  } else if (isUnfilledImagePlaceholderFrame(node)) {
    kind = 'placeholder-frame';
  } else if (isImageAreaFrameByHeuristic(node)) {
    kind = 'placeholder-frame';
  } else {
    return;
  }
  if (queuedNodeIds.has(node.id)) return;

  queuedNodeIds.add(node.id);
  const query = extractQueryForNode(node);
  const aspect = inferAspectRatio(node);

  imageSearchQueue.push({ id: node.id, query, aspect, kind });
  useCanvasStore.getState().setImageSearchStatus(node.id, 'pending');

  // Start processing if not already running
  processQueue();
}

/**
 * Reset the queue state. Call when a new generation starts.
 */
export function resetImageSearchQueue(): void {
  queueAbort?.abort();
  queueAbort = null;
  imageSearchQueue.length = 0;
  queuedNodeIds.clear();
  queueProcessing = false;
}

async function processQueue(): Promise<void> {
  if (queueProcessing) return;
  queueProcessing = true;

  if (!queueAbort) queueAbort = new AbortController();
  const abort = queueAbort;

  const { updateNode } = useDocumentStore.getState();
  const { setImageSearchStatus } = useCanvasStore.getState();
  const { openverseOAuth } = useAgentSettingsStore.getState();

  while (imageSearchQueue.length > 0) {
    if (abort.signal.aborted) break;
    const item = imageSearchQueue.shift()!;

    // Re-check that the node still needs filling. The user (or another
    // pass) may have already filled the image / replaced the placeholder
    // since we enqueued.
    const currentNode = useDocumentStore.getState().getNodeById(item.id);
    let stillNeedsFill = false;
    if (currentNode) {
      if (item.kind === 'image' && currentNode.type === 'image') {
        stillNeedsFill = isPlaceholderSrc((currentNode as ImageNode).src);
      } else if (item.kind === 'placeholder-frame') {
        // Strict re-check: the frame must STILL be a placeholder AND not
        // already filled with an image. Without the fill check, a
        // follow-up generation that resets `queuedNodeIds` and re-runs
        // `scanAndFillImages` on the same tree would re-search the
        // already-good photo and overwrite it with whatever the new
        // search returns.
        stillNeedsFill = isUnfilledImagePlaceholderFrame(currentNode);
      }
    }
    if (!stillNeedsFill) {
      queuedNodeIds.delete(item.id);
      continue;
    }

    try {
      const res = await fetch('/api/ai/image-search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: item.query,
          count: 1,
          aspectRatio: item.aspect,
          ...(openverseOAuth && {
            openverseClientId: openverseOAuth.clientId,
            openverseClientSecret: openverseOAuth.clientSecret,
          }),
        }),
        signal: abort.signal,
      });
      const data = await res.json();
      if (data.results?.length > 0) {
        const thumbUrl = data.results[0].thumbUrl;
        if (item.kind === 'placeholder-frame') {
          // Repaint the placeholder frame as a real image surface: drop
          // the gray slate-100 fill in favor of an image fill, and clear
          // the icon_font + label children so the photo isn't covered by
          // the placeholder visual.
          updateNode(item.id, {
            fill: [{ type: 'image', url: thumbUrl, mode: 'crop' }],
            children: [],
          } as Partial<PenNode>);
        } else {
          updateNode(item.id, { src: thumbUrl });
        }
        setImageSearchStatus(item.id, 'found');
      } else {
        setImageSearchStatus(item.id, 'failed');
      }
    } catch {
      if (!abort.signal.aborted) {
        setImageSearchStatus(item.id, 'failed');
      }
    }

    // Rate limit: 3s between requests to stay under Openverse 20/min burst
    if (!abort.signal.aborted && imageSearchQueue.length > 0) {
      await new Promise((r) => setTimeout(r, 3000));
    }
  }

  queueProcessing = false;
  if (!abort.signal.aborted) {
    forcePageResync();
  }
}

// ---------------------------------------------------------------------------
// Batch scan — final sweep to catch any missed placeholder images
// ---------------------------------------------------------------------------

export async function scanAndFillImages(rootId: string): Promise<void> {
  const targets = collectImageSearchTargets(rootId);
  const needsFill = targets.filter((t) => !queuedNodeIds.has(t.node.id));

  if (needsFill.length === 0) return;

  // Enqueue any remaining unfilled nodes — the queue processor handles the rest
  for (const target of needsFill) {
    enqueueImageForSearch(target.node);
  }
}
