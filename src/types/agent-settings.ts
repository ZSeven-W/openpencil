export type AIProviderType = 'anthropic' | 'openai'

export interface AIProviderConfig {
  type: AIProviderType
  displayName: string
  isConnected: boolean
  connectionMethod: 'claude-code' | 'codex-cli' | null
  /** Models fetched when the user connects this provider */
  models: GroupedModel[]
}

export type MCPCliTool =
  | 'claude-code'
  | 'codex-cli'
  | 'gemini-cli'
  | 'opencode-cli'
  | 'kiro-cli'

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
