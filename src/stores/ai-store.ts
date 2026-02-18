import { create } from 'zustand'
import type { ChatMessage } from '@/services/ai/ai-types'

export type PanelCorner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'

interface AIState {
  messages: ChatMessage[]
  isStreaming: boolean
  isPanelOpen: boolean
  activeTab: 'chat' | 'code'
  generatedCode: string
  codeFormat: 'react-tailwind' | 'html-css' | 'react-inline'
  panelCorner: PanelCorner
  isMinimized: boolean

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
  panelCorner: 'bottom-right',
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

  clearMessages: () => set({ messages: [] }),

  setPanelCorner: (panelCorner) => set({ panelCorner }),
  toggleMinimize: () => set((s) => ({ isMinimized: !s.isMinimized })),
}))
