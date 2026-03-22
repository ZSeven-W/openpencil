import { create } from 'zustand'
import type { DesignMdSpec } from '@/types/design-md'
import { appStorage } from '@/utils/app-storage'

const STORAGE_PREFIX = 'openpencil-design-md:'
const CURRENT_KEY_STORAGE = 'openpencil-design-md-current-key'

/** Derive a storage key from a file identifier. Returns null for untitled documents. */
function fileKey(fileName: string | null, filePath: string | null): string | null {
  return filePath ?? fileName ?? null
}

interface DesignMdStoreState {
  /** Current design.md spec */
  designMd: DesignMdSpec | undefined
  /** Current file key for persistence (null = untitled, skip persistence) */
  _fileKey: string | null

  setDesignMd: (spec: DesignMdSpec | undefined) => void
  /** Sync store to a document — restores persisted designMd or clears if none. */
  syncToDocument: (fileName: string | null, filePath: string | null) => void
  /** Called on new document — clears designMd. */
  clearForNewDocument: () => void
  hydrate: () => void
}

export const useDesignMdStore = create<DesignMdStoreState>((set, get) => ({
  designMd: undefined,
  _fileKey: null,

  setDesignMd: (spec) => {
    set({ designMd: spec })
    const key = get()._fileKey
    if (!key) return // untitled — skip persistence
    try {
      if (spec) {
        appStorage.setItem(STORAGE_PREFIX + key, JSON.stringify(spec))
      } else {
        appStorage.removeItem(STORAGE_PREFIX + key)
      }
    } catch { /* ignore */ }
  },

  syncToDocument: (fileName, filePath) => {
    const key = fileKey(fileName, filePath)
    set({ _fileKey: key })

    if (!key) {
      set({ designMd: undefined })
      return
    }

    // Restore persisted designMd for this file
    try {
      const raw = appStorage.getItem(STORAGE_PREFIX + key)
      if (raw) {
        const data = JSON.parse(raw) as DesignMdSpec
        if (data && typeof data === 'object' && typeof data.raw === 'string') {
          set({ designMd: data })
          return
        }
      }
    } catch { /* ignore */ }

    set({ designMd: undefined })
  },

  clearForNewDocument: () => {
    set({ designMd: undefined, _fileKey: null })
  },

  hydrate: () => {
    try {
      const lastKey = appStorage.getItem(CURRENT_KEY_STORAGE)
      if (!lastKey) return
      set({ _fileKey: lastKey })
      const raw = appStorage.getItem(STORAGE_PREFIX + lastKey)
      if (!raw) return
      const data = JSON.parse(raw) as DesignMdSpec
      if (data && typeof data === 'object' && typeof data.raw === 'string') {
        set({ designMd: data })
      }
    } catch { /* ignore */ }
  },
}))

// Persist the current file key whenever state changes
let _prevFileKey: string | null = null
useDesignMdStore.subscribe((state) => {
  if (state._fileKey !== _prevFileKey) {
    _prevFileKey = state._fileKey
    try {
      if (state._fileKey) {
        appStorage.setItem(CURRENT_KEY_STORAGE, state._fileKey)
      } else {
        appStorage.removeItem(CURRENT_KEY_STORAGE)
      }
    } catch { /* ignore */ }
  }
})
