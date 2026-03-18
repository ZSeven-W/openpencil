export type AIProviderType = 'anthropic' | 'openai' | 'opencode' | 'copilot'

export interface AIProviderConfig {
  type: AIProviderType
  displayName: string
  isConnected: boolean
  connectionMethod: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot' | null
  /** Models fetched when the user connects this provider */
  models: GroupedModel[]
  /** Human-readable connection status, e.g. "Connected via API key" */
  connectionInfo?: string
  /** Config file path for the hint (client renders localized text) */
  hintPath?: string
}

export type MCPCliTool =
  | 'claude-code'
  | 'codex-cli'
  | 'gemini-cli'
  | 'opencode-cli'
  | 'kiro-cli'
  | 'copilot-cli'

export type MCPTransportMode = 'stdio' | 'http' | 'both'

export interface MCPCliIntegration {
  tool: MCPCliTool
  displayName: string
  enabled: boolean
  installed: boolean
}

export interface GroupedModel {
  value: string
  displayName: string
  description: string
  provider: AIProviderType
}

export interface ModelGroup {
  provider: AIProviderType
  providerName: string
  models: GroupedModel[]
}
