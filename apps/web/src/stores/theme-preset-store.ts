import { create } from 'zustand'
import type { ThemePreset } from '@/types/theme-preset'
import type { VariableDefinition } from '@/types/variables'
import { appStorage } from '@/utils/app-storage'

const STORAGE_KEY = 'openpencil-theme-presets'

interface ThemePresetStoreState {
  presets: ThemePreset[]

  savePreset: (name: string, themes: Record<string, string[]>, variables: Record<string, VariableDefinition>) => void
  deletePreset: (id: string) => void
  renamePreset: (id: string, name: string) => void
  persist: () => void
  hydrate: () => void
}

function generateId(): string {
  return `preset-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

export const useThemePresetStore = create<ThemePresetStoreState>((set, get) => ({
  presets: [],

  savePreset: (name, themes, variables) => {
    const preset: ThemePreset = {
      id: generateId(),
      name,
      themes,
      variables,
      createdAt: Date.now(),
    }
    set((s) => ({ presets: [...s.presets, preset] }))
    get().persist()
  },

  deletePreset: (id) => {
    set((s) => ({ presets: s.presets.filter((p) => p.id !== id) }))
    get().persist()
  },

  renamePreset: (id, name) => {
    set((s) => ({
      presets: s.presets.map((p) => (p.id === id ? { ...p, name } : p)),
    }))
    get().persist()
  },

  persist: () => {
    try {
      const { presets } = get()
      appStorage.setItem(STORAGE_KEY, JSON.stringify(presets))
    } catch {
      // ignore — localStorage may be full
    }
  },

  hydrate: () => {
    try {
      const raw = appStorage.getItem(STORAGE_KEY)
      if (!raw) return
      const data = JSON.parse(raw)
      if (Array.isArray(data)) {
        set({ presets: data })
      }
    } catch {
      // ignore
    }
  },
}))
