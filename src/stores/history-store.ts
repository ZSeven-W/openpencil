import { create } from 'zustand'
import type { PenDocument, PenNode } from '@/types/pen'

const MAX_HISTORY = 300

function areDocumentsEqual(a: PenDocument, b: PenDocument): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

interface HistoryStoreState {
  undoStack: PenDocument[]
  redoStack: PenDocument[]
  batchDepth: number
  batchBaseState: PenDocument | null

  pushState: (doc: PenDocument) => void
  undo: (currentDoc: PenDocument) => PenDocument | null
  redo: (currentDoc: PenDocument) => PenDocument | null
  canUndo: () => boolean
  canRedo: () => boolean
  clear: () => void
  startBatch: (doc: PenDocument) => void
  endBatch: (currentDoc?: PenDocument) => void

  // Legacy API compatibility (used by some canvas event handlers)
  beginBatch: (currentChildren: PenNode[]) => void
  cancelBatch: () => void
}

export const useHistoryStore = create<HistoryStoreState>(
  (set, get) => ({
    undoStack: [],
    redoStack: [],
    batchDepth: 0,
    batchBaseState: null,

    pushState: (doc) => {
      const { batchDepth } = get()
      if (batchDepth > 0) return

      set((s) => {
        const last = s.undoStack[s.undoStack.length - 1]
        if (last && areDocumentsEqual(last, doc)) {
          return { redoStack: [] }
        }

        return {
          undoStack: [...s.undoStack.slice(-(MAX_HISTORY - 1)), structuredClone(doc)],
          redoStack: [],
        }
      })
    },

    undo: (currentDoc) => {
      const { undoStack } = get()
      if (undoStack.length === 0) return null

      const previous = undoStack[undoStack.length - 1]
      const currentClone = structuredClone(currentDoc)
      set((s) => ({
        undoStack: s.undoStack.slice(0, -1),
        redoStack: [...s.redoStack, currentClone],
      }))
      return structuredClone(previous)
    },

    redo: (currentDoc) => {
      const { redoStack } = get()
      if (redoStack.length === 0) return null

      const next = redoStack[redoStack.length - 1]
      const currentClone = structuredClone(currentDoc)
      set((s) => ({
        redoStack: s.redoStack.slice(0, -1),
        undoStack: [...s.undoStack, currentClone],
      }))
      return structuredClone(next)
    },

    canUndo: () => get().undoStack.length > 0,
    canRedo: () => get().redoStack.length > 0,

    clear: () => set({ undoStack: [], redoStack: [], batchDepth: 0, batchBaseState: null }),

    startBatch: (doc) => {
      const { batchDepth } = get()
      if (batchDepth === 0) {
        set({ batchBaseState: structuredClone(doc), batchDepth: 1 })
      } else {
        set({ batchDepth: batchDepth + 1 })
      }
    },

    endBatch: (currentDoc) => {
      const { batchDepth, batchBaseState } = get()
      if (batchDepth <= 0) return

      if (batchDepth === 1 && batchBaseState) {
        const hasNoChanges = currentDoc
          ? areDocumentsEqual(batchBaseState, currentDoc)
          : false

        if (hasNoChanges) {
          set({ batchDepth: 0, batchBaseState: null })
          return
        }

        set((s) => ({
          undoStack: [...s.undoStack.slice(-(MAX_HISTORY - 1)), batchBaseState],
          redoStack: [],
          batchDepth: 0,
          batchBaseState: null,
        }))
      } else {
        set({ batchDepth: batchDepth - 1 })
      }
    },

    // Legacy compatibility: beginBatch wraps startBatch by constructing a doc from children
    beginBatch: (currentChildren) => {
      const doc: PenDocument = { version: '1.0.0', children: currentChildren }
      get().startBatch(doc)
    },

    cancelBatch: () => {
      set({ batchDepth: 0, batchBaseState: null })
    },
  }),
)
