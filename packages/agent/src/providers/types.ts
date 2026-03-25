import type { LanguageModel } from 'ai'

export interface ProviderConfig {
  apiKey: string
  model: string
  baseURL?: string
  maxContextTokens?: number
  providerOptions?: Record<string, unknown>
}

export interface AgentProvider {
  model: LanguageModel
  id: string
  maxContextTokens: number
  supportsThinking: boolean
}
