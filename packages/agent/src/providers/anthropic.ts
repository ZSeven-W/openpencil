import { createAnthropic } from '@ai-sdk/anthropic'
import type { AgentProvider, ProviderConfig } from './types'

const DEFAULT_MAX_CONTEXT = 200_000

export function createAnthropicProvider(config: ProviderConfig): AgentProvider {
  const anthropic = createAnthropic({
    apiKey: config.apiKey,
    ...(config.baseURL ? { baseURL: config.baseURL } : {}),
  })
  return {
    model: anthropic(config.model),
    id: 'anthropic',
    maxContextTokens: config.maxContextTokens ?? DEFAULT_MAX_CONTEXT,
    supportsThinking: true,
  }
}
