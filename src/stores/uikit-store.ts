import { create } from 'zustand'
import type { UIKit, ComponentCategory } from '@/types/uikit'
import { getBuiltInKits } from '@/uikit/built-in-registry'

const STORAGE_KEY = 'openpencil-uikits'

interface PersistedState {
  importedKits: UIKit[]
  browserOpen?: boolean
}

interface UIKitStoreState {
  /** All loaded kits (built-in + imported) */
  kits: UIKit[]
  /** Whether the browser panel is open */
  browserOpen: boolean
  /** Current search query */
  searchQuery: string
  /** Active category filter (null = all) */
  activeCategory: ComponentCategory | null
  /** Active kit filter (null = all) */
  activeKitId: string | null

  toggleBrowser: () => void
  setBrowserOpen: (open: boolean) => void
  setSearchQuery: (query: string) => void
  setActiveCategory: (category: ComponentCategory | null) => void
  setActiveKitId: (kitId: string | null) => void
  importKit: (kit: UIKit) => void
  removeKit: (kitId: string) => void
  persist: () => void
  hydrate: () => void
}

export const useUIKitStore = create<UIKitStoreState>((set, get) => ({
  kits: getBuiltInKits(),
  browserOpen: false,
  searchQuery: '',
  activeCategory: null,
  activeKitId: null,

  toggleBrowser: () => {
    const next = !get().browserOpen
    set({ browserOpen: next })
    get().persist()
  },
  setBrowserOpen: (open) => {
    set({ browserOpen: open })
    get().persist()
  },
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setActiveCategory: (activeCategory) => set({ activeCategory }),
  setActiveKitId: (activeKitId) => set({ activeKitId }),

  importKit: (kit) => {
    set((s) => ({ kits: [...s.kits, kit] }))
    get().persist()
  },

  removeKit: (kitId) => {
    const { activeKitId } = get()
    set((s) => ({
      kits: s.kits.filter((k) => k.id !== kitId || k.builtIn),
      // Reset filter if the deleted kit was selected
      activeKitId: activeKitId === kitId ? null : activeKitId,
    }))
    get().persist()
  },

  persist: () => {
    try {
      const { kits, browserOpen } = get()
      const imported = kits.filter((k) => !k.builtIn)
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ importedKits: imported, browserOpen }))
    } catch {
      // ignore — localStorage may be full
    }
  },

  hydrate: () => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (!raw) return
      const data = JSON.parse(raw) as Partial<PersistedState>
      if (data.importedKits && Array.isArray(data.importedKits)) {
        const builtIn = getBuiltInKits()
        const builtInIds = new Set(builtIn.map((k) => k.id))
        // Filter out any imported kits that clash with built-in IDs
        const imported = data.importedKits.filter((k) => !builtInIds.has(k.id))
        set({ kits: [...builtIn, ...imported] })
      }
      if (typeof data.browserOpen === 'boolean') set({ browserOpen: data.browserOpen })
    } catch {
      // ignore
    }
  },
}))
