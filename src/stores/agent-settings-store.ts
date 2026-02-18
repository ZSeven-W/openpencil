import { create } from 'zustand'
import type {
  AIProviderType,
  AIProviderConfig,
  MCPCliIntegration,
  GroupedModel,
} from '@/types/agent-settings'

const STORAGE_KEY = 'openpencil-agent-settings'

interface PersistedState {
  providers: Record<AIProviderType, AIProviderConfig>
  mcpIntegrations: MCPCliIntegration[]
}

interface AgentSettingsState extends PersistedState {
  dialogOpen: boolean

  connectProvider: (
    provider: AIProviderType,
    method: AIProviderConfig['connectionMethod'],
    models: GroupedModel[],
  ) => void
  disconnectProvider: (provider: AIProviderType) => void
  toggleMCPIntegration: (tool: string) => void
  setDialogOpen: (open: boolean) => void
  persist: () => void
  hydrate: () => void
}

const DEFAULT_PROVIDERS: Record<AIProviderType, AIProviderConfig> = {
  anthropic: {
    type: 'anthropic',
    displayName: 'Claude Code',
    isConnected: false,
    connectionMethod: null,
    models: [],
  },
  openai: {
    type: 'openai',
    displayName: 'Codex CLI',
    isConnected: false,
    connectionMethod: null,
    models: [],
  },
}

const DEFAULT_MCP_INTEGRATIONS: MCPCliIntegration[] = [
  { tool: 'claude-code', displayName: 'Claude Code CLI', enabled: false, installed: false },
  { tool: 'codex-cli', displayName: 'Codex CLI', enabled: false, installed: false },
  { tool: 'gemini-cli', displayName: 'Gemini CLI', enabled: false, installed: false },
  { tool: 'opencode-cli', displayName: 'OpenCode CLI', enabled: false, installed: false },
  { tool: 'kiro-cli', displayName: 'Kiro CLI', enabled: false, installed: false },
]

export const useAgentSettingsStore = create<AgentSettingsState>((set, get) => ({
  providers: { ...DEFAULT_PROVIDERS },
  mcpIntegrations: [...DEFAULT_MCP_INTEGRATIONS],
  dialogOpen: false,

  connectProvider: (provider, method, models) =>
    set((s) => ({
      providers: {
        ...s.providers,
        [provider]: {
          ...s.providers[provider],
          isConnected: true,
          connectionMethod: method,
          models,
        },
      },
    })),

  disconnectProvider: (provider) =>
    set((s) => ({
      providers: {
        ...s.providers,
        [provider]: {
          ...DEFAULT_PROVIDERS[provider],
        },
      },
    })),

  toggleMCPIntegration: (tool) =>
    set((s) => ({
      mcpIntegrations: s.mcpIntegrations.map((m) =>
        m.tool === tool ? { ...m, enabled: !m.enabled } : m,
      ),
    })),

  setDialogOpen: (dialogOpen) => set({ dialogOpen }),

  persist: () => {
    try {
      const { providers, mcpIntegrations } = get()
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ providers, mcpIntegrations }))
    } catch {
      // ignore
    }
  },

  hydrate: () => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (!raw) return
      const data = JSON.parse(raw) as Partial<PersistedState>
      if (data.providers) {
        // Merge with defaults to ensure new fields (e.g. models) exist
        const merged = { ...DEFAULT_PROVIDERS }
        for (const key of Object.keys(merged) as AIProviderType[]) {
          if (data.providers[key]) {
            merged[key] = { ...merged[key], ...data.providers[key] }
            // Ensure models array always exists
            if (!Array.isArray(merged[key].models)) merged[key].models = []
          }
        }
        set({ providers: merged })
      }
      if (data.mcpIntegrations) set({ mcpIntegrations: data.mcpIntegrations })
    } catch {
      // ignore
    }
  },
}))
