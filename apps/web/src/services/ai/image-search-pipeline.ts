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

export function collectImageNodes(rootId: string): ImageNode[] {
  const { getNodeById } = useDocumentStore.getState();
  const root = getNodeById(rootId);
  if (!root) return [];

  const images: ImageNode[] = [];
  const walk = (node: PenNode) => {
    if (node.type === 'image') images.push(node);
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) walk(child);
    }
  };
  walk(root);
  return images;
}

// Only match the known phone placeholder prefix, not user-uploaded SVGs
const PHONE_PLACEHOLDER_PREFIX = 'data:image/svg+xml;charset=utf-8,%3Csvg';

function isPlaceholderSrc(src?: string): boolean {
  return !src || src.startsWith(PHONE_PLACEHOLDER_PREFIX);
}

// ---------------------------------------------------------------------------
// Incremental queue-based image search
// ---------------------------------------------------------------------------

interface QueuedImage {
  id: string;
  query: string;
  aspect: 'wide' | 'tall' | 'square' | undefined;
}

const imageSearchQueue: QueuedImage[] = [];
// Track IDs already queued or processed to avoid duplicates
const queuedNodeIds = new Set<string>();
let queueProcessing = false;
let queueAbort: AbortController | null = null;

/**
 * Enqueue a single image node for background search.
 * Called from insertStreamingNode as soon as an image node hits the canvas.
 */
export function enqueueImageForSearch(node: PenNode): void {
  if (node.type !== 'image') return;
  const imgNode = node as ImageNode;
  if (!isPlaceholderSrc(imgNode.src)) return;
  if (queuedNodeIds.has(node.id)) return;

  queuedNodeIds.add(node.id);
  const query = imgNode.imageSearchQuery ?? imgNode.name ?? 'placeholder';
  const aspect = inferAspectRatio(node);

  imageSearchQueue.push({ id: node.id, query, aspect });
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

    // Re-check that the node still has a placeholder (may have been filled by user)
    const currentNode = useDocumentStore.getState().getNodeById(item.id);
    if (
      !currentNode ||
      currentNode.type !== 'image' ||
      !isPlaceholderSrc((currentNode as ImageNode).src)
    ) {
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
        updateNode(item.id, { src: data.results[0].thumbUrl });
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
  const imageNodes = collectImageNodes(rootId);
  const needsFill = imageNodes.filter((n) => isPlaceholderSrc(n.src) && !queuedNodeIds.has(n.id));

  if (needsFill.length === 0) return;

  // Enqueue any remaining unfilled nodes — the queue processor handles the rest
  for (const node of needsFill) {
    enqueueImageForSearch(node);
  }
}
