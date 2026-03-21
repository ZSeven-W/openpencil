import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import { forcePageResync } from '@/canvas/canvas-sync-utils'
import type { PenNode, ImageNode } from '@/types/pen'

export function inferAspectRatio(
  node: PenNode,
): 'wide' | 'tall' | 'square' | undefined {
  const n = node as unknown as Record<string, unknown>
  const w = typeof n['width'] === 'number' ? (n['width'] as number) : 0
  const h = typeof n['height'] === 'number' ? (n['height'] as number) : 0
  if (!w || !h) return undefined
  const ratio = w / h
  if (ratio > 1.3) return 'wide'
  if (ratio < 0.77) return 'tall'
  return 'square'
}

export function collectImageNodes(rootId: string): ImageNode[] {
  const { getNodeById } = useDocumentStore.getState()
  const root = getNodeById(rootId)
  if (!root) return []

  const images: ImageNode[] = []
  const walk = (node: PenNode) => {
    if (node.type === 'image') images.push(node)
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) walk(child)
    }
  }
  walk(root)
  return images
}

// Only match the known phone placeholder prefix, not user-uploaded SVGs
const PHONE_PLACEHOLDER_PREFIX = 'data:image/svg+xml;charset=utf-8,%3Csvg'

function isPlaceholderSrc(src?: string): boolean {
  return !src || src.startsWith(PHONE_PLACEHOLDER_PREFIX)
}

// Module-level abort controller for cancellation
let currentAbort: AbortController | null = null

export async function scanAndFillImages(rootId: string): Promise<void> {
  // Cancel any previous scan
  currentAbort?.abort()
  const abort = new AbortController()
  currentAbort = abort

  const imageNodes = collectImageNodes(rootId)
  const needsFill = imageNodes.filter((n) => isPlaceholderSrc(n.src))

  if (needsFill.length === 0) return

  const { setImageSearchStatus } = useCanvasStore.getState()
  const { updateNode } = useDocumentStore.getState()
  const { openverseOAuth } = useAgentSettingsStore.getState()

  // Mark all as pending
  for (const node of needsFill) {
    setImageSearchStatus(node.id, 'pending')
  }

  for (const node of needsFill) {
    if (abort.signal.aborted) return

    // Prefer short search keywords; fall back to name
    const query = node.imageSearchQuery ?? node.name ?? 'placeholder'
    const aspect = inferAspectRatio(node)

    try {
      const res = await fetch('/api/ai/image-search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query,
          count: 1,
          aspectRatio: aspect,
          ...(openverseOAuth && {
            openverseClientId: openverseOAuth.clientId,
            openverseClientSecret: openverseOAuth.clientSecret,
          }),
        }),
        signal: abort.signal,
      })
      const data = await res.json()
      if (data.results?.length > 0) {
        updateNode(node.id, { src: data.results[0].thumbUrl })
        setImageSearchStatus(node.id, 'found')
      } else {
        setImageSearchStatus(node.id, 'failed')
      }
    } catch {
      if (!abort.signal.aborted) {
        setImageSearchStatus(node.id, 'failed')
      }
    }

    // Rate limit: 3s between requests to stay under Openverse 20/min burst
    if (!abort.signal.aborted) {
      await new Promise((r) => setTimeout(r, 3000))
    }
  }

  if (!abort.signal.aborted) {
    forcePageResync()
  }
}
