import { create } from 'zustand'
import type {
  AIProviderType,
  AIProviderConfig,
  MCPCliIntegration,
  MCPTransportMode,
  GroupedModel,
} from '@/types/agent-settings'
import { MCP_DEFAULT_PORT } from '@/constants/app'
import { appStorage } from '@/utils/app-storage'

const STORAGE_KEY = 'openpencil-agent-settings'

interface PersistedState {
  providers: Record<AIProviderType, AIProviderConfig>
  mcpIntegrations: MCPCliIntegration[]
  mcpTransportMode: MCPTransportMode
  mcpHttpPort: number
}

interface AgentSettingsState extends PersistedState {
  dialogOpen: boolean
  isHydrated: boolean
  mcpServerRunning: boolean
  mcpServerLocalIp: string | null

  connectProvider: (
    provider: AIProviderType,
    method: AIProviderConfig['connectionMethod'],
    models: GroupedModel[],
    connectionInfo?: string,
    hintPath?: string,
  ) => void
  disconnectProvider: (provider: AIProviderType) => void
  toggleMCPIntegration: (tool: string) => void
  setMCPTransport: (mode: MCPTransportMode, port?: number) => void
  setMcpServerStatus: (running: boolean, localIp?: string | null) => void
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
  opencode: {
    type: 'opencode',
    displayName: 'OpenCode',
    isConnected: false,
    connectionMethod: null,
    models: [],
  },
  copilot: {
    type: 'copilot',
    displayName: 'GitHub Copilot',
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
  { tool: 'copilot-cli', displayName: 'GitHub Copilot CLI', enabled: false, installed: false },
]

export const useAgentSettingsStore = create<AgentSettingsState>((set, get) => ({
  providers: { ...DEFAULT_PROVIDERS },
  mcpIntegrations: [...DEFAULT_MCP_INTEGRATIONS],
  mcpTransportMode: 'stdio',
  mcpHttpPort: MCP_DEFAULT_PORT,
  dialogOpen: false,
  isHydrated: false,
  mcpServerRunning: false,
  mcpServerLocalIp: null,

  connectProvider: (provider, method, models, connectionInfo, hintPath) =>
    set((s) => ({
      providers: {
        ...s.providers,
        [provider]: {
          ...s.providers[provider],
          isConnected: true,
          connectionMethod: method,
          models,
          connectionInfo,
          hintPath,
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

  setMCPTransport: (mode, port) =>
    set({
      mcpTransportMode: mode,
      ...(port != null && { mcpHttpPort: port }),
    }),

  setMcpServerStatus: (running, localIp) =>
    set({ mcpServerRunning: running, mcpServerLocalIp: localIp ?? null }),

  setDialogOpen: (dialogOpen) => set({ dialogOpen }),

  persist: () => {
    try {
      const { providers, mcpIntegrations, mcpTransportMode, mcpHttpPort } = get()
      appStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ providers, mcpIntegrations, mcpTransportMode, mcpHttpPort }),
      )
    } catch {
      // ignore
    }
  },

  hydrate: () => {
    try {
      const raw = appStorage.getItem(STORAGE_KEY)
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
      if (data.mcpIntegrations) {
        const mergedMcp = DEFAULT_MCP_INTEGRATIONS.map((def) => {
          const saved = data.mcpIntegrations!.find((m) => m.tool === def.tool)
          return saved ? { ...def, ...saved } : def
        })
        set({ mcpIntegrations: mergedMcp })
      }
      if (data.mcpTransportMode) set({ mcpTransportMode: data.mcpTransportMode })
      if (data.mcpHttpPort) set({ mcpHttpPort: data.mcpHttpPort })
    } catch {
      // ignore
    } finally {
      set({ isHydrated: true })
    }
  },
}))
