/**
 * Undo bridge for timeline operations.
 *
 * Guarantees correct ordering:
 * 1. injectAnimationData() — serialize current timeline state into document
 * 2. startBatch(doc)       — capture pre-mutation snapshot
 * 3. fn()                  — execute mutations
 * 4. injectAnimationData() — serialize post-mutation state
 * 5. endBatch(currentDoc)  — push to history (with no-op detection)
 */

import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { injectAnimationData } from '@/animation/animation-persistence'

export function withTimelineUndoBatch(fn: () => void): void {
  // 1. Serialize current timeline state into document
  injectAnimationData()

  // 2. Capture pre-mutation snapshot
  const doc = useDocumentStore.getState().document
  useHistoryStore.getState().startBatch(doc)

  try {
    // 3. Execute mutations
    fn()

    // 4. Serialize post-mutation state
    injectAnimationData()

    // 5. Push to history with no-op detection
    const currentDoc = useDocumentStore.getState().document
    useHistoryStore.getState().endBatch(currentDoc)
  } catch (e) {
    // Ensure batch is closed even on error to prevent dangling startBatch state
    const currentDoc = useDocumentStore.getState().document
    useHistoryStore.getState().endBatch(currentDoc)
    throw e
  }
}
