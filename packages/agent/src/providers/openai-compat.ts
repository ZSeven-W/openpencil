import { createOpenAI } from '@ai-sdk/openai'
import type { AgentProvider, ProviderConfig } from './types'

const DEFAULT_MAX_CONTEXT = 128_000

export function createOpenAICompatProvider(config: ProviderConfig): AgentProvider {
  const openai = createOpenAI({
    apiKey: config.apiKey,
    ...(config.baseURL ? { baseURL: config.baseURL } : {}),
  })
  return {
    model: openai.chat(config.model),
    id: 'openai-compat',
    maxContextTokens: config.maxContextTokens ?? DEFAULT_MAX_CONTEXT,
    supportsThinking: false,
  }
}
