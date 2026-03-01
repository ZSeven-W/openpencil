import { nanoid } from 'nanoid'
import type { PenDocument, PenNode, PenPage } from '@/types/pen'
import { useHistoryStore } from '@/stores/history-store'
import { useCanvasStore } from '@/stores/canvas-store'

interface PageActions {
  addPage: () => string
  removePage: (pageId: string) => void
  renamePage: (pageId: string, name: string) => void
  reorderPage: (pageId: string, direction: 'left' | 'right') => void
  duplicatePage: (pageId: string) => string | null
}

export function createPageActions(
  set: (partial: Partial<{ document: PenDocument; isDirty: boolean }>) => void,
  get: () => { document: PenDocument },
): PageActions {
  return {
    addPage: () => {
      const state = get()
      useHistoryStore.getState().pushState(state.document)
      const doc = state.document
      const pages = doc.pages ?? []
      const pageNum = pages.length + 1
      const newPageId = nanoid()
      const newPage: PenPage = {
        id: newPageId,
        name: `Page ${pageNum}`,
        children: [
          {
            id: nanoid(),
            type: 'frame',
            name: 'Frame',
            x: 0,
            y: 0,
            width: 1200,
            height: 800,
            fill: [{ type: 'solid', color: '#FFFFFF' }],
            children: [],
          },
        ],
      }
      set({
        document: { ...doc, pages: [...pages, newPage] },
        isDirty: true,
      })
      useCanvasStore.getState().setActivePageId(newPageId)
      return newPageId
    },

    removePage: (pageId) => {
      const state = get()
      const doc = state.document
      if (!doc.pages || doc.pages.length <= 1) return // Can't delete last page
      useHistoryStore.getState().pushState(doc)
      const newPages = doc.pages.filter((p) => p.id !== pageId)
      set({ document: { ...doc, pages: newPages }, isDirty: true })
      // If we deleted the active page, switch to the first page
      const activePageId = useCanvasStore.getState().activePageId
      if (activePageId === pageId) {
        useCanvasStore.getState().setActivePageId(newPages[0].id)
      }
    },

    renamePage: (pageId, name) => {
      const state = get()
      const doc = state.document
      if (!doc.pages) return
      useHistoryStore.getState().pushState(doc)
      set({
        document: {
          ...doc,
          pages: doc.pages.map((p) => (p.id === pageId ? { ...p, name } : p)),
        },
        isDirty: true,
      })
    },

    reorderPage: (pageId, direction) => {
      const state = get()
      const doc = state.document
      if (!doc.pages) return
      const idx = doc.pages.findIndex((p) => p.id === pageId)
      if (idx === -1) return
      const newIdx = direction === 'left' ? idx - 1 : idx + 1
      if (newIdx < 0 || newIdx >= doc.pages.length) return
      useHistoryStore.getState().pushState(doc)
      const newPages = [...doc.pages]
      const [moved] = newPages.splice(idx, 1)
      newPages.splice(newIdx, 0, moved)
      set({ document: { ...doc, pages: newPages }, isDirty: true })
    },

    duplicatePage: (pageId) => {
      const state = get()
      const doc = state.document
      if (!doc.pages) return null
      const page = doc.pages.find((p) => p.id === pageId)
      if (!page) return null
      useHistoryStore.getState().pushState(doc)
      const newPageId = nanoid()
      // Deep-clone children with new IDs
      const cloneWithNewIds = (n: PenNode): PenNode => {
        const cloned = { ...n, id: nanoid() } as PenNode
        if ('children' in cloned && cloned.children) {
          cloned.children = cloned.children.map(cloneWithNewIds)
        }
        return cloned
      }
      const newPage: PenPage = {
        id: newPageId,
        name: `${page.name} copy`,
        children: page.children.map(cloneWithNewIds),
      }
      const idx = doc.pages.findIndex((p) => p.id === pageId)
      const newPages = [...doc.pages]
      newPages.splice(idx + 1, 0, newPage)
      set({ document: { ...doc, pages: newPages }, isDirty: true })
      useCanvasStore.getState().setActivePageId(newPageId)
      return newPageId
    },
  }
}
