import { create } from 'zustand'
import type { ChatMessage, ChatAttachment } from '@/services/ai/ai-types'
import type { ModelGroup } from '@/types/agent-settings'

export type PanelCorner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'

const DEFAULT_MODEL = 'claude-sonnet-4-5-20250929'
const MODEL_PREFERENCE_STORAGE_KEY = 'openpencil-ai-model-preference'

function readStoredModelPreference(): string | null {
  if (typeof window === 'undefined') return null
  try {
    const value = window.localStorage.getItem(MODEL_PREFERENCE_STORAGE_KEY)
    if (!value || value.trim().length === 0) return null
    return value
  } catch {
    return null
  }
}

function writeStoredModelPreference(model: string): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(MODEL_PREFERENCE_STORAGE_KEY, model)
  } catch {
    // Ignore storage failures (private mode, quota, etc.)
  }
}

// Keep SSR/CSR first render deterministic to avoid hydration mismatch.
// Real preference is loaded on mount via hydrateModelPreference().
const initialPreferredModel = DEFAULT_MODEL

export interface AIModelInfo {
  value: string
  displayName: string
  description: string
}

interface AIState {
  messages: ChatMessage[]
  isStreaming: boolean
  isPanelOpen: boolean
  activeTab: 'chat' | 'code'
  generatedCode: string
  codeFormat: 'react-tailwind' | 'html-css' | 'react-inline'
  model: string
  preferredModel: string
  availableModels: AIModelInfo[]
  modelGroups: ModelGroup[]
  isLoadingModels: boolean
  panelCorner: PanelCorner
  isMinimized: boolean
  chatTitle: string
  generationProgress: { current: number; total: number } | null
  pendingAttachments: ChatAttachment[]

  setChatTitle: (title: string) => void
  setGenerationProgress: (progress: { current: number; total: number } | null) => void

  hydrateModelPreference: () => void
  selectModel: (model: string) => void
  setModel: (model: string) => void
  setAvailableModels: (models: AIModelInfo[]) => void
  setModelGroups: (groups: ModelGroup[]) => void
  setLoadingModels: (v: boolean) => void
  addMessage: (msg: ChatMessage) => void
  updateLastMessage: (content: string) => void
  setStreaming: (v: boolean) => void
  togglePanel: () => void
  setPanelOpen: (open: boolean) => void
  setActiveTab: (tab: 'chat' | 'code') => void
  setGeneratedCode: (code: string) => void
  setCodeFormat: (f: 'react-tailwind' | 'html-css' | 'react-inline') => void
  clearMessages: () => void
  setPanelCorner: (corner: PanelCorner) => void
  toggleMinimize: () => void
  addPendingAttachment: (attachment: ChatAttachment) => void
  removePendingAttachment: (id: string) => void
  clearPendingAttachments: () => void
}

export const useAIStore = create<AIState>((set) => ({
  messages: [],
  isStreaming: false,
  isPanelOpen: true,
  activeTab: 'chat',
  generatedCode: '',
  codeFormat: 'react-tailwind',
  model: initialPreferredModel,
  preferredModel: initialPreferredModel,
  availableModels: [],
  modelGroups: [],
  isLoadingModels: false,
  panelCorner: 'bottom-left',
  isMinimized: false,
  chatTitle: 'New Chat',
  generationProgress: null,
  pendingAttachments: [],

  setChatTitle: (chatTitle) => set({ chatTitle }),
  setGenerationProgress: (generationProgress) => set({ generationProgress }),

  hydrateModelPreference: () => {
    const stored = readStoredModelPreference()
    if (!stored) return
    set({ model: stored, preferredModel: stored })
  },

  addMessage: (msg) =>
    set((s) => ({ messages: [...s.messages, msg] })),

  updateLastMessage: (content) =>
    set((s) => {
      const msgs = [...s.messages]
      const last = msgs[msgs.length - 1]
      if (last && last.role === 'assistant') {
        msgs[msgs.length - 1] = { ...last, content }
      }
      return { messages: msgs }
    }),

  setStreaming: (isStreaming) => set({ isStreaming }),

  togglePanel: () => set((s) => ({ isPanelOpen: !s.isPanelOpen })),

  setPanelOpen: (isPanelOpen) => set({ isPanelOpen }),

  setActiveTab: (activeTab) => set({ activeTab }),

  setGeneratedCode: (generatedCode) => set({ generatedCode }),

  setCodeFormat: (codeFormat) => set({ codeFormat }),

  selectModel: (model) => {
    writeStoredModelPreference(model)
    set({ model, preferredModel: model })
  },
  setModel: (model) => set({ model }),
  setAvailableModels: (availableModels) => set({ availableModels }),
  setModelGroups: (modelGroups) => set({ modelGroups }),
  setLoadingModels: (isLoadingModels) => set({ isLoadingModels }),
  clearMessages: () => set({ messages: [], chatTitle: 'New Chat' }),

  setPanelCorner: (panelCorner) => set({ panelCorner }),
  toggleMinimize: () => set((s) => ({ isMinimized: !s.isMinimized })),

  addPendingAttachment: (attachment) =>
    set((s) => ({ pendingAttachments: [...s.pendingAttachments, attachment] })),
  removePendingAttachment: (id) =>
    set((s) => ({ pendingAttachments: s.pendingAttachments.filter((a) => a.id !== id) })),
  clearPendingAttachments: () => set({ pendingAttachments: [] }),
}))
