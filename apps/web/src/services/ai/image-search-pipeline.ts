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

function extractQueryForNode(node: PenNode): string {
  const r = node as PenNode & {
    imageSearchQuery?: string;
    name?: string;
    children?: PenNode[];
  };
  if (typeof r.imageSearchQuery === 'string' && r.imageSearchQuery.length > 0) {
    return r.imageSearchQuery;
  }
  if (
    typeof r.name === 'string' &&
    r.name.length > 0 &&
    r.name !== 'Image Placeholder' &&
    r.name !== 'Placeholder Icon'
  ) {
    return r.name;
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
 * Enqueue an image target for background search. Accepts either a real
 * `image` node with a placeholder src OR a `frame` carrying
 * `role: 'image-placeholder'` (what `add_image_placeholder_v0/v1` emit).
 *
 * Called from insertStreamingNode for streamed image nodes, and from
 * `scanAndFillImages` for both shapes after a non-streaming insert (the
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
