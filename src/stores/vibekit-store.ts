import { create } from 'zustand'
import type { VibeKit } from '@/types/vibekit'
import { appStorage } from '@/utils/app-storage'

const STORAGE_KEY = 'jeans-vibekits'

interface VibeKitStoreState {
  /** Currently active kit ID (null = no kit applied yet / onboarding needed) */
  activeKitId: string | null
  /** All saved kits keyed by ID */
  kits: Record<string, VibeKit>

  saveKit: (kit: VibeKit) => void
  removeKit: (kitId: string) => void
  setActiveKit: (kitId: string) => void
  getActiveKit: () => VibeKit | null
  persist: () => void
  hydrate: () => void
}

export const useVibeKitStore = create<VibeKitStoreState>((set, get) => ({
  activeKitId: null,
  kits: {},

  saveKit: (kit) => {
    set((s) => ({ kits: { ...s.kits, [kit.id]: kit } }))
    get().persist()
  },

  removeKit: (kitId) => {
    const { activeKitId } = get()
    set((s) => {
      const { [kitId]: _, ...rest } = s.kits
      return {
        kits: rest,
        activeKitId: activeKitId === kitId ? null : activeKitId,
      }
    })
    get().persist()
  },

  setActiveKit: (kitId) => {
    set({ activeKitId: kitId })
    get().persist()
  },

  getActiveKit: () => {
    const { activeKitId, kits } = get()
    if (!activeKitId) return null
    return kits[activeKitId] ?? null
  },

  persist: () => {
    try {
      const { activeKitId, kits } = get()
      appStorage.setItem(STORAGE_KEY, JSON.stringify({ activeKitId, kits }))
    } catch {
      // ignore — localStorage may be full
    }
  },

  hydrate: () => {
    try {
      const raw = appStorage.getItem(STORAGE_KEY)
      if (!raw) return
      const data = JSON.parse(raw)
      if (data && typeof data === 'object') {
        if (data.kits && typeof data.kits === 'object') {
          set({ kits: data.kits })
        }
        if (typeof data.activeKitId === 'string' || data.activeKitId === null) {
          set({ activeKitId: data.activeKitId })
        }
      }
    } catch {
      // ignore
    }
  },
}))
