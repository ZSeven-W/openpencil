import { create } from 'zustand'
import type { ChatMessage } from '@/services/ai/ai-types'

export type PanelCorner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'

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
  availableModels: AIModelInfo[]
  isLoadingModels: boolean
  panelCorner: PanelCorner
  isMinimized: boolean

  setModel: (model: string) => void
  setAvailableModels: (models: AIModelInfo[]) => void
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
}

export const useAIStore = create<AIState>((set) => ({
  messages: [],
  isStreaming: false,
  isPanelOpen: true,
  activeTab: 'chat',
  generatedCode: '',
  codeFormat: 'react-tailwind',
  model: 'claude-sonnet-4-5-20250929',
  availableModels: [],
  isLoadingModels: false,
  panelCorner: 'bottom-left',
  isMinimized: false,

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

  setModel: (model) => set({ model }),
  setAvailableModels: (availableModels) => set({ availableModels }),
  setLoadingModels: (isLoadingModels) => set({ isLoadingModels }),
  clearMessages: () => set({ messages: [] }),

  setPanelCorner: (panelCorner) => set({ panelCorner }),
  toggleMinimize: () => set((s) => ({ isMinimized: !s.isMinimized })),
}))
