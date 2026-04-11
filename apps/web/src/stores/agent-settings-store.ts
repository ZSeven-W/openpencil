import { create } from 'zustand';
import type {
  AIProviderType,
  AIProviderConfig,
  MCPCliIntegration,
  MCPTransportMode,
  GroupedModel,
} from '@/types/agent-settings';
import type { ImageGenConfig, ImageGenProfile } from '@/types/image-service';
import { DEFAULT_IMAGE_GEN_CONFIG } from '@/types/image-service';
import { MCP_DEFAULT_PORT } from '@/constants/app';
import { canonicalizeBuiltinProviderConfig } from '@/lib/builtin-provider-presets';
import { appStorage } from '@/utils/app-storage';

const STORAGE_KEY = 'openpencil-agent-settings';

export type BuiltinProviderPreset =
  | 'anthropic'
  | 'openai'
  | 'openrouter'
  | 'deepseek'
  | 'gemini'
  | 'minimax'
  | 'zhipu'
  | 'glm-coding'
  | 'kimi'
  | 'bailian'
  | 'bailian-coding'
  | 'doubao'
  | 'ark-coding'
  | 'xiaomi'
  | 'modelscope'
  | 'stepfun'
  | 'nvidia'
  | 'custom';

export interface BuiltinProviderConfig {
  id: string;
  displayName: string;
  type: 'anthropic' | 'openai-compat';
  apiKey: string;
  model: string;
  baseURL?: string;
  preset?: BuiltinProviderPreset;
  maxContextTokens?: number;
  enabled: boolean;
}

interface PersistedState {
  providers: Record<AIProviderType, AIProviderConfig>;
  mcpIntegrations: MCPCliIntegration[];
  mcpTransportMode: MCPTransportMode;
  mcpHttpPort: number;
  imageGenConfig: ImageGenConfig;
  imageGenProfiles: ImageGenProfile[];
  activeImageGenProfileId: string | null;
  openverseOAuth: { clientId: string; clientSecret: string } | null;
  builtinProviders: BuiltinProviderConfig[];
  teamEnabled: boolean;
  teamDesignModel: string | null;
}

interface AgentSettingsState extends PersistedState {
  dialogOpen: boolean;
  isHydrated: boolean;
  mcpServerRunning: boolean;
  mcpServerLocalIp: string | null;

  connectProvider: (
    provider: AIProviderType,
    method: AIProviderConfig['connectionMethod'],
    models: GroupedModel[],
    connectionInfo?: string,
    hintPath?: string,
  ) => void;
  disconnectProvider: (provider: AIProviderType) => void;
  toggleMCPIntegration: (tool: string) => void;
  setMCPTransport: (mode: MCPTransportMode, port?: number) => void;
  setMcpServerStatus: (running: boolean, localIp?: string | null) => void;
  setDialogOpen: (open: boolean) => void;
  setImageGenConfig: (config: Partial<ImageGenConfig>) => void;
  addImageGenProfile: (profile: Omit<ImageGenProfile, 'id'>) => string;
  updateImageGenProfile: (id: string, updates: Partial<Omit<ImageGenProfile, 'id'>>) => void;
  removeImageGenProfile: (id: string) => void;
  setActiveImageGenProfile: (id: string | null) => void;
  getActiveImageGenProfile: () => ImageGenProfile | null;
  setOpenverseOAuth: (oauth: { clientId: string; clientSecret: string } | null) => void;
  addBuiltinProvider: (config: Omit<BuiltinProviderConfig, 'id'>) => string;
  updateBuiltinProvider: (id: string, updates: Partial<BuiltinProviderConfig>) => void;
  removeBuiltinProvider: (id: string) => void;
  setTeamEnabled: (enabled: boolean) => void;
  setTeamDesignModel: (model: string | null) => void;
  persist: () => void;
  hydrate: () => void;
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
  gemini: {
    type: 'gemini',
    displayName: 'Gemini CLI',
    isConnected: false,
    connectionMethod: null,
    models: [],
  },
};

const DEFAULT_MCP_INTEGRATIONS: MCPCliIntegration[] = [
  { tool: 'claude-code', displayName: 'Claude Code CLI', enabled: false, installed: false },
  { tool: 'codex-cli', displayName: 'Codex CLI', enabled: false, installed: false },
  { tool: 'gemini-cli', displayName: 'Gemini CLI', enabled: false, installed: false },
  { tool: 'opencode-cli', displayName: 'OpenCode CLI', enabled: false, installed: false },
  { tool: 'kiro-cli', displayName: 'Kiro CLI', enabled: false, installed: false },
  { tool: 'copilot-cli', displayName: 'GitHub Copilot CLI', enabled: false, installed: false },
];

export const useAgentSettingsStore = create<AgentSettingsState>((set, get) => ({
  providers: { ...DEFAULT_PROVIDERS },
  mcpIntegrations: [...DEFAULT_MCP_INTEGRATIONS],
  mcpTransportMode: 'stdio',
  mcpHttpPort: MCP_DEFAULT_PORT,
  imageGenConfig: DEFAULT_IMAGE_GEN_CONFIG,
  imageGenProfiles: [],
  activeImageGenProfileId: null,
  openverseOAuth: null,
  builtinProviders: [],
  teamEnabled: false,
  teamDesignModel: null,
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

  addImageGenProfile: (profile) => {
    const id = `igp-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    const newProfile: ImageGenProfile = { ...profile, id };
    set((s) => {
      const profiles = [...s.imageGenProfiles, newProfile];
      // First profile becomes active by default
      const activeId = s.activeImageGenProfileId ?? id;
      return { imageGenProfiles: profiles, activeImageGenProfileId: activeId };
    });
    return id;
  },

  updateImageGenProfile: (id, updates) =>
    set((s) => ({
      imageGenProfiles: s.imageGenProfiles.map((p) => (p.id === id ? { ...p, ...updates } : p)),
    })),

  removeImageGenProfile: (id) =>
    set((s) => {
      const profiles = s.imageGenProfiles.filter((p) => p.id !== id);
      let activeId = s.activeImageGenProfileId;
      if (activeId === id) {
        activeId = profiles.length > 0 ? profiles[0].id : null;
      }
      return { imageGenProfiles: profiles, activeImageGenProfileId: activeId };
    }),

  setActiveImageGenProfile: (id) => set({ activeImageGenProfileId: id }),

  getActiveImageGenProfile: () => {
    const { imageGenProfiles, activeImageGenProfileId } = get();
    if (!activeImageGenProfileId) return imageGenProfiles[0] ?? null;
    return (
      imageGenProfiles.find((p) => p.id === activeImageGenProfileId) ?? imageGenProfiles[0] ?? null
    );
  },

  setOpenverseOAuth: (oauth) => set({ openverseOAuth: oauth }),

  addBuiltinProvider: (config) => {
    const id = `bp-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
    const newProvider = canonicalizeBuiltinProviderConfig({ ...config, id });
    set((s) => ({ builtinProviders: [...s.builtinProviders, newProvider] }));
    return id;
  },

  updateBuiltinProvider: (id, updates) =>
    set((s) => ({
      builtinProviders: s.builtinProviders.map((p) =>
        p.id === id ? canonicalizeBuiltinProviderConfig({ ...p, ...updates }) : p,
      ),
    })),

  removeBuiltinProvider: (id) =>
    set((s) => ({
      builtinProviders: s.builtinProviders.filter((p) => p.id !== id),
    })),

  setTeamEnabled: (teamEnabled) => set({ teamEnabled }),
  setTeamDesignModel: (teamDesignModel) => set({ teamDesignModel }),

  persist: () => {
    try {
      const {
        providers,
        mcpIntegrations,
        mcpTransportMode,
        mcpHttpPort,
        imageGenConfig,
        imageGenProfiles,
        activeImageGenProfileId,
        openverseOAuth,
        builtinProviders,
        teamEnabled,
        teamDesignModel,
      } = get();
      appStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          providers,
          mcpIntegrations,
          mcpTransportMode,
          mcpHttpPort,
          imageGenConfig,
          imageGenProfiles,
          activeImageGenProfileId,
          openverseOAuth,
          builtinProviders,
          teamEnabled,
          teamDesignModel,
        }),
      );
    } catch {
      // ignore
    }
  },

  hydrate: () => {
    try {
      const raw = appStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const data = JSON.parse(raw) as Partial<PersistedState>;
      if (data.providers) {
        // Merge with defaults to ensure new fields (e.g. models) exist
        const merged = { ...DEFAULT_PROVIDERS };
        for (const key of Object.keys(merged) as AIProviderType[]) {
          if (data.providers[key]) {
            merged[key] = { ...merged[key], ...data.providers[key] };
            // Ensure models array always exists
            if (!Array.isArray(merged[key].models)) merged[key].models = [];
          }
        }
        set({ providers: merged });
      }
      if (data.mcpIntegrations) {
        const mergedMcp = DEFAULT_MCP_INTEGRATIONS.map((def) => {
          const saved = data.mcpIntegrations!.find((m) => m.tool === def.tool);
          return saved ? { ...def, ...saved } : def;
        });
        set({ mcpIntegrations: mergedMcp });
      }
      if (data.mcpTransportMode) set({ mcpTransportMode: data.mcpTransportMode });
      if (data.mcpHttpPort) set({ mcpHttpPort: data.mcpHttpPort });
      if (data.imageGenConfig) set({ imageGenConfig: data.imageGenConfig });
      // Hydrate multi-profile image gen
      if ((data as Record<string, unknown>).imageGenProfiles) {
        const profiles = (data as Record<string, unknown>).imageGenProfiles as ImageGenProfile[];
        const activeId = (data as Record<string, unknown>).activeImageGenProfileId as string | null;
        set({ imageGenProfiles: profiles, activeImageGenProfileId: activeId });
      } else if (data.imageGenConfig && data.imageGenConfig.apiKey) {
        // Migrate old single config to profiles
        const migrated: ImageGenProfile = {
          id: 'igp-migrated',
          name:
            data.imageGenConfig.provider === 'custom'
              ? 'Custom'
              : data.imageGenConfig.provider.charAt(0).toUpperCase() +
                data.imageGenConfig.provider.slice(1),
          ...data.imageGenConfig,
        };
        set({ imageGenProfiles: [migrated], activeImageGenProfileId: migrated.id });
      }
      if (data.openverseOAuth !== undefined) set({ openverseOAuth: data.openverseOAuth });
      const builtinProviders = (data as Record<string, unknown>).builtinProviders;
      if (Array.isArray(builtinProviders)) {
        set({
          builtinProviders: builtinProviders.map((p: unknown) =>
            canonicalizeBuiltinProviderConfig(p as BuiltinProviderConfig),
          ) as BuiltinProviderConfig[],
        });
      }
      if ((data as Record<string, unknown>).teamEnabled !== undefined) {
        set({ teamEnabled: (data as Record<string, unknown>).teamEnabled as boolean });
      }
      if ((data as Record<string, unknown>).teamDesignModel !== undefined) {
        set({
          teamDesignModel: (data as Record<string, unknown>).teamDesignModel as string | null,
        });
      }
    } catch {
      // ignore
    } finally {
      set({ isHydrated: true });
    }
  },
}));
