import type { PenDocument } from '@/types/pen'
import { useTimelineStore } from '@/stores/timeline-store'
import { useDocumentStore } from '@/stores/document-store'

/**
 * Inject animation data into the document before saving.
 * Call this before any save operation.
 */
export function injectAnimationData(): void {
  const timelineData = useTimelineStore.getState().getTimelineData()
  const hasAnimation =
    Object.keys(timelineData.tracks).length > 0

  useDocumentStore.setState((s) => ({
    document: {
      ...s.document,
      animation: hasAnimation ? timelineData : undefined,
    },
  }))
}

/**
 * Extract animation data from a loaded document and load it into the timeline store.
 */
export function extractAnimationData(doc: PenDocument): void {
  if (doc.animation) {
    useTimelineStore.getState().loadTimelineData(doc.animation)
  } else {
    useTimelineStore.getState().clearAllTracks()
  }
}

/**
 * Remove animation tracks for nodes that no longer exist in the document.
 */
export function reconcileAnimationWithDocument(): void {
  const doc = useDocumentStore.getState().document
  const nodeIds = new Set<string>()

  function collectIds(nodes: PenDocument['children']) {
    for (const node of nodes) {
      nodeIds.add(node.id)
      if ('children' in node && node.children) {
        collectIds(node.children)
      }
    }
  }

  // Collect IDs from all pages
  if (doc.pages) {
    for (const page of doc.pages) {
      collectIds(page.children)
    }
  }
  collectIds(doc.children)

  useTimelineStore.getState().reconcile(nodeIds)
}
