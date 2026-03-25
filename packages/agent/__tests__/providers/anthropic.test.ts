import { describe, it, expect } from 'vitest'
import { createAnthropicProvider } from '../../src/providers/anthropic'

describe('createAnthropicProvider', () => {
  it('creates a provider with correct metadata', () => {
    const provider = createAnthropicProvider({
      apiKey: 'test-key',
      model: 'claude-sonnet-4-20250514',
    })
    expect(provider.id).toBe('anthropic')
    expect(provider.supportsThinking).toBe(true)
    expect(provider.maxContextTokens).toBe(200_000)
    expect(provider.model).toBeDefined()
  })

  it('allows custom maxContextTokens', () => {
    const provider = createAnthropicProvider({
      apiKey: 'test-key',
      model: 'claude-sonnet-4-20250514',
      maxContextTokens: 100_000,
    })
    expect(provider.maxContextTokens).toBe(100_000)
  })
})
