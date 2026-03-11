/**
 * Composition accessors — thin indirection layer to break the circular
 * dependency between timeline-store and document-store.
 *
 * timeline-store needs to read/write `document.composition` but
 * document-store imports timeline-store for track cleanup on node delete.
 * Both stores import from this module instead of from each other.
 */

import type { CompositionSettings } from '@/types/animation'
import type { PenDocument } from '@/types/pen'

// Late-bound reference to the document store.  Populated by
// document-store.ts at module-init time via `registerDocumentStore`.
let _getDocumentState: (() => { document: PenDocument }) | null = null
let _setDocumentState: ((updater: (s: { document: PenDocument }) => Partial<{ document: PenDocument }>) => void) | null = null

/**
 * Called once by document-store at module init to wire up the accessor
 * without creating an import cycle.
 */
export function registerDocumentStore(
  getState: () => { document: PenDocument },
  setState: (updater: (s: { document: PenDocument }) => Partial<{ document: PenDocument }>) => void,
) {
  _getDocumentState = getState
  _setDocumentState = setState
}

/** Read the current composition from the document store (if registered). */
export function getDocumentComposition(): CompositionSettings | undefined {
  if (!_getDocumentState) return undefined
  return _getDocumentState().document.composition
}

/**
 * Write composition settings to the document store (if registered).
 * Merges `settings` into the existing `document.composition`.
 */
export function setDocumentComposition(settings: Partial<CompositionSettings>): void {
  if (!_getDocumentState || !_setDocumentState) return
  const current = _getDocumentState().document.composition
  _setDocumentState((s) => ({
    document: {
      ...s.document,
      composition: { ...current, ...settings } as CompositionSettings,
    },
  }))
}
