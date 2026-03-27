import { createOpenAI } from '@ai-sdk/openai'
import type { AgentProvider, ProviderConfig } from './types'

const DEFAULT_MAX_CONTEXT = 128_000

export function createOpenAICompatProvider(config: ProviderConfig): AgentProvider {
  const openai = createOpenAI({
    apiKey: config.apiKey,
    ...(config.baseURL ? { baseURL: config.baseURL } : {}),
    // Safety net: ensure tool_calls[].function.arguments is always a valid
    // JSON string. Some SDK versions or edge cases may produce missing/empty
    // arguments which strict APIs (MiniMax, StepFun) reject.
    fetch: async (url, options) => {
      if (options?.body && typeof options.body === 'string') {
        try {
          const body = JSON.parse(options.body)
          let patched = false
          for (const msg of body.messages ?? []) {
            if (!msg.tool_calls) continue
            for (const tc of msg.tool_calls) {
              if (!tc.function) continue
              const args = tc.function.arguments
              if (args === undefined || args === null || args === '') {
                tc.function.arguments = '{}'
                patched = true
              } else if (typeof args !== 'string') {
                tc.function.arguments = JSON.stringify(args)
                patched = true
              }
            }
          }
          if (patched) {
            options = { ...options, body: JSON.stringify(body) }
          }
        } catch { /* ignore */ }
      }
      return globalThis.fetch(url, options)
    },
  })
  return {
    model: openai.chat(config.model),
    id: 'openai-compat',
    maxContextTokens: config.maxContextTokens ?? DEFAULT_MAX_CONTEXT,
    supportsThinking: false,
  }
}
