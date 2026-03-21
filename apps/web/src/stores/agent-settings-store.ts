import { create } from 'zustand'
import type {
  AIProviderType,
  AIProviderConfig,
  MCPCliIntegration,
  MCPTransportMode,
  GroupedModel,
} from '@/types/agent-settings'
import type { ImageGenConfig } from '@/types/image-service'
import { DEFAULT_IMAGE_GEN_CONFIG } from '@/types/image-service'
import { MCP_DEFAULT_PORT } from '@/constants/app'
import { appStorage } from '@/utils/app-storage'

const STORAGE_KEY = 'openpencil-agent-settings'

interface PersistedState {
  providers: Record<AIProviderType, AIProviderConfig>
  mcpIntegrations: MCPCliIntegration[]
  mcpTransportMode: MCPTransportMode
  mcpHttpPort: number
  imageGenConfig: ImageGenConfig
  openverseOAuth: { clientId: string; clientSecret: string } | null
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
  setImageGenConfig: (config: Partial<ImageGenConfig>) => void
  setOpenverseOAuth: (oauth: { clientId: string; clientSecret: string } | null) => void
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
  imageGenConfig: DEFAULT_IMAGE_GEN_CONFIG,
  openverseOAuth: null,
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

  setImageGenConfig: (updates) =>
    set((s) => ({
      imageGenConfig: { ...s.imageGenConfig, ...updates },
    })),

  setOpenverseOAuth: (oauth) => set({ openverseOAuth: oauth }),

  persist: () => {
    try {
      const { providers, mcpIntegrations, mcpTransportMode, mcpHttpPort, imageGenConfig, openverseOAuth } = get()
      appStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ providers, mcpIntegrations, mcpTransportMode, mcpHttpPort, imageGenConfig, openverseOAuth }),
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
      if (data.imageGenConfig) set({ imageGenConfig: data.imageGenConfig })
      if (data.openverseOAuth !== undefined) set({ openverseOAuth: data.openverseOAuth })
    } catch {
      // ignore
    } finally {
      set({ isHydrated: true })
    }
  },
}))
