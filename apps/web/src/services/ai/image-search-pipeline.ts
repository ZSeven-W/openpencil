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

// 只匹配我们自己生成的占位图前缀，不误伤用户上传的 SVG。
const PHONE_PLACEHOLDER_PREFIX = 'data:image/svg+xml;charset=utf-8,%3Csvg';

function isPlaceholderSrc(src?: string): boolean {
  return !src || src.startsWith(PHONE_PLACEHOLDER_PREFIX);
}

// ---------------------------------------------------------------------------
// 增量式、基于队列的图片搜索
// ---------------------------------------------------------------------------

interface QueuedImage {
  id: string;
  query: string;
  aspect: 'wide' | 'tall' | 'square' | undefined;
}

const imageSearchQueue: QueuedImage[] = [];
// 记录已经排队或处理过的节点 ID，避免重复搜索
const queuedNodeIds = new Set<string>();
let queueProcessing = false;
let queueAbort: AbortController | null = null;

/**
 * 把单个图片节点加入后台搜索队列。
 * 当流式生成过程中有图片节点落到画布上时，就会触发这里。
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

  // 如果处理器还没跑起来，就立即启动
  processQueue();
}

/** 重置队列状态。通常在开启新一轮生成时调用。 */
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

    // 再检查一次节点是否仍然是占位图，避免用户已经手动替换后还继续搜索。
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

    // 简单限速：请求间隔 3 秒，尽量压在 Openverse 20/min 爆发阈值以下。
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
// 批量扫描：兜底抓取前面漏掉的占位图
// ---------------------------------------------------------------------------

export async function scanAndFillImages(rootId: string): Promise<void> {
  const imageNodes = collectImageNodes(rootId);
  const needsFill = imageNodes.filter((n) => isPlaceholderSrc(n.src) && !queuedNodeIds.has(n.id));

  if (needsFill.length === 0) return;

  // 把所有还没填充的节点补进队列，剩下的交给队列处理器。
  for (const node of needsFill) {
    enqueueImageForSearch(node);
  }
}
