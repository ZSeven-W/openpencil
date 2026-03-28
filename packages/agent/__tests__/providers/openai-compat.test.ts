import { describe, it, expect } from 'vitest'
import { createOpenAICompatProvider } from '../../src/providers/openai-compat'

describe('createOpenAICompatProvider', () => {
  it('creates a provider with correct metadata', () => {
    const provider = createOpenAICompatProvider({
      apiKey: 'test-key',
      model: 'gpt-4o',
    })
    expect(provider.id).toBe('openai-compat')
    expect(provider.supportsThinking).toBe(false)
    expect(provider.maxContextTokens).toBe(128_000)
    expect(provider.model).toBeDefined()
  })

  it('supports custom baseURL', () => {
    const provider = createOpenAICompatProvider({
      apiKey: 'test-key',
      model: 'deepseek-chat',
      baseURL: 'https://api.deepseek.com/v1',
    })
    expect(provider.id).toBe('openai-compat')
  })
})
