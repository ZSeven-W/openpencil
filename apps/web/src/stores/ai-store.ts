import { create } from 'zustand';
import type { ChatMessage, ChatAttachment } from '@/services/ai/ai-types';
import type { ModelGroup } from '@/types/agent-settings';
import type { ToolCallBlockData } from '@/components/panels/tool-call-block';
import { appStorage } from '@/utils/app-storage';

export type PanelCorner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

const DEFAULT_MODEL = 'claude-sonnet-4-5-20250929';
const MODEL_PREFERENCE_STORAGE_KEY = 'openpencil-ai-model-preference';
const CONCURRENCY_STORAGE_KEY = 'openpencil-ai-concurrency';
const UI_PREFS_KEY = 'openpencil-ai-ui-preferences';

interface AIUIPrefs {
  isPanelOpen?: boolean;
  panelCorner?: PanelCorner;
  isMinimized?: boolean;
  codeFormat?: 'react-tailwind' | 'html-css' | 'react-inline';
  panelWidth?: number;
  panelHeight?: number;
}

function readUIPrefs(): AIUIPrefs {
  if (typeof window === 'undefined') return {};
  try {
    const raw = appStorage.getItem(UI_PREFS_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function writeUIPrefs(partial: AIUIPrefs): void {
  if (typeof window === 'undefined') return;
  try {
    const current = readUIPrefs();
    appStorage.setItem(UI_PREFS_KEY, JSON.stringify({ ...current, ...partial }));
  } catch {
    /* ignore */
  }
}

function readStoredModelPreference(): string | null {
  if (typeof window === 'undefined') return null;
  try {
    const value = appStorage.getItem(MODEL_PREFERENCE_STORAGE_KEY);
    if (!value || value.trim().length === 0) return null;
    return value;
  } catch {
    return null;
  }
}

function writeStoredModelPreference(model: string): void {
  if (typeof window === 'undefined') return;
  try {
    appStorage.setItem(MODEL_PREFERENCE_STORAGE_KEY, model);
  } catch {
    // Ignore storage failures (private mode, quota, etc.)
  }
}

function readStoredConcurrency(): number {
  if (typeof window === 'undefined') return 1;
  try {
    const value = appStorage.getItem(CONCURRENCY_STORAGE_KEY);
    if (!value) return 1;
    const n = parseInt(value, 10);
    return n >= 1 && n <= 6 ? n : 1;
  } catch {
    return 1;
  }
}

function writeStoredConcurrency(n: number): void {
  if (typeof window === 'undefined') return;
  try {
    appStorage.setItem(CONCURRENCY_STORAGE_KEY, String(n));
  } catch {
    // Ignore storage failures
  }
}

// Keep SSR/CSR first render deterministic to avoid hydration mismatch.
// Real preference is loaded on mount via hydrateModelPreference().
const initialPreferredModel = DEFAULT_MODEL;

export interface AIModelInfo {
  value: string;
  displayName: string;
  description: string;
}

interface AIState {
  messages: ChatMessage[];
  isStreaming: boolean;
  isPanelOpen: boolean;
  activeTab: 'chat' | 'code';
  generatedCode: string;
  codeFormat: 'react-tailwind' | 'html-css' | 'react-inline';
  model: string;
  preferredModel: string;
  availableModels: AIModelInfo[];
  modelGroups: ModelGroup[];
  isLoadingModels: boolean;
  panelCorner: PanelCorner;
  isMinimized: boolean;
  panelWidth: number;
  panelHeight: number;
  isMaximized: boolean;
  toggleMaximize: () => void;
  setPanelSize: (width: number, height: number) => void;
  chatTitle: string;
  generationProgress: { current: number; total: number } | null;
  /** Step tags from orchestrator during Agent tool execution (bypasses message content) */
  agentOrchestrationSteps: string | null;
  setAgentOrchestrationSteps: (steps: string | null) => void;
  concurrency: number;
  toolCallBlocks: ToolCallBlockData[];
  pendingAttachments: ChatAttachment[];
  abortController: AbortController | null;

  addToolCallBlock: (block: ToolCallBlockData) => void;
  updateToolCallBlock: (id: string, updates: Partial<ToolCallBlockData>) => void;
  clearToolCallBlocks: () => void;
  setConcurrency: (n: number) => void;
  setChatTitle: (title: string) => void;
  setGenerationProgress: (progress: { current: number; total: number } | null) => void;

  hydrateModelPreference: () => void;
  selectModel: (model: string) => void;
  setModel: (model: string) => void;
  setAvailableModels: (models: AIModelInfo[]) => void;
  setModelGroups: (groups: ModelGroup[]) => void;
  setLoadingModels: (v: boolean) => void;
  addMessage: (msg: ChatMessage) => void;
  updateLastMessage: (content: string) => void;
  setStreaming: (v: boolean) => void;
  togglePanel: () => void;
  setPanelOpen: (open: boolean) => void;
  setActiveTab: (tab: 'chat' | 'code') => void;
  setGeneratedCode: (code: string) => void;
  setCodeFormat: (f: 'react-tailwind' | 'html-css' | 'react-inline') => void;
  clearMessages: () => void;
  setPanelCorner: (corner: PanelCorner) => void;
  toggleMinimize: () => void;
  addPendingAttachment: (attachment: ChatAttachment) => void;
  removePendingAttachment: (id: string) => void;
  clearPendingAttachments: () => void;
  setAbortController: (c: AbortController | null) => void;
  stopStreaming: () => void;
}

export const useAIStore = create<AIState>((set, get) => ({
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
  panelWidth: 360,
  panelHeight: 400,
  isMaximized: false,
  chatTitle: 'New Chat',
  concurrency: 1,
  generationProgress: null,
  agentOrchestrationSteps: null,
  setAgentOrchestrationSteps: (steps) => set({ agentOrchestrationSteps: steps }),
  toolCallBlocks: [],
  pendingAttachments: [],
  abortController: null,

  addToolCallBlock: (block) => set((s) => ({ toolCallBlocks: [...s.toolCallBlocks, block] })),
  updateToolCallBlock: (id, updates) =>
    set((s) => ({
      toolCallBlocks: s.toolCallBlocks.map((b) => (b.id === id ? { ...b, ...updates } : b)),
    })),
  clearToolCallBlocks: () => set({ toolCallBlocks: [] }),
  setConcurrency: (n) => {
    const clamped = Math.max(1, Math.min(6, n));
    writeStoredConcurrency(clamped);
    set({ concurrency: clamped });
  },
  setChatTitle: (chatTitle) => set({ chatTitle }),
  setGenerationProgress: (generationProgress) => set({ generationProgress }),

  hydrateModelPreference: () => {
    const stored = readStoredModelPreference();
    if (stored) set({ model: stored, preferredModel: stored });
    const storedConcurrency = readStoredConcurrency();
    if (storedConcurrency !== 1) set({ concurrency: storedConcurrency });
    const prefs = readUIPrefs();
    if (typeof prefs.isPanelOpen === 'boolean') set({ isPanelOpen: prefs.isPanelOpen });
    if (prefs.panelCorner) set({ panelCorner: prefs.panelCorner });
    if (typeof prefs.isMinimized === 'boolean') set({ isMinimized: prefs.isMinimized });
    if (prefs.panelWidth) set({ panelWidth: prefs.panelWidth });
    if (prefs.panelHeight) set({ panelHeight: prefs.panelHeight });
    if (prefs.codeFormat) set({ codeFormat: prefs.codeFormat });
  },

  addMessage: (msg) =>
    set((s) => ({
      messages: [...s.messages, msg],
      // Clear stale orchestration steps when a new user message starts a new turn
      agentOrchestrationSteps: msg.role === 'user' ? null : s.agentOrchestrationSteps,
    })),

  updateLastMessage: (content) =>
    set((s) => {
      const msgs = [...s.messages];
      const last = msgs[msgs.length - 1];
      if (last && last.role === 'assistant') {
        msgs[msgs.length - 1] = { ...last, content };
      }
      return { messages: msgs };
    }),

  setStreaming: (isStreaming) => set({ isStreaming }),

  togglePanel: () => {
    const next = !get().isPanelOpen;
    set({ isPanelOpen: next });
    writeUIPrefs({ isPanelOpen: next });
  },

  setPanelOpen: (isPanelOpen) => {
    set({ isPanelOpen });
    writeUIPrefs({ isPanelOpen });
  },

  setActiveTab: (activeTab) => set({ activeTab }),

  setGeneratedCode: (generatedCode) => set({ generatedCode }),

  setCodeFormat: (codeFormat) => {
    set({ codeFormat });
    writeUIPrefs({ codeFormat });
  },

  selectModel: (model) => {
    writeStoredModelPreference(model);
    set({ model, preferredModel: model });
  },
  setModel: (model) => set({ model }),
  setAvailableModels: (availableModels) => set({ availableModels }),
  setModelGroups: (modelGroups) => set({ modelGroups }),
  setLoadingModels: (isLoadingModels) => set({ isLoadingModels }),
  clearMessages: () =>
    set({ messages: [], chatTitle: 'New Chat', toolCallBlocks: [], agentOrchestrationSteps: null }),

  setPanelCorner: (panelCorner) => {
    set({ panelCorner });
    writeUIPrefs({ panelCorner });
  },
  toggleMinimize: () => {
    const next = !get().isMinimized;
    set({ isMinimized: next });
    writeUIPrefs({ isMinimized: next });
  },

  addPendingAttachment: (attachment) =>
    set((s) => ({ pendingAttachments: [...s.pendingAttachments, attachment] })),
  removePendingAttachment: (id) =>
    set((s) => ({ pendingAttachments: s.pendingAttachments.filter((a) => a.id !== id) })),
  clearPendingAttachments: () => set({ pendingAttachments: [] }),

  setAbortController: (abortController) => set({ abortController }),
  stopStreaming: () =>
    set((s) => {
      s.abortController?.abort();
      return { isStreaming: false, abortController: null };
    }),

  toggleMaximize: () => {
    const next = !get().isMaximized;
    set({ isMaximized: next });
  },
  setPanelSize: (panelWidth, panelHeight) => {
    set({ panelWidth, panelHeight });
    writeUIPrefs({ panelWidth, panelHeight });
  },
}));
