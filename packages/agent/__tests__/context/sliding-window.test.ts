import { describe, it, expect } from 'vitest'
import { createSlidingWindowStrategy } from '../../src/context/sliding-window'

describe('createSlidingWindowStrategy', () => {
  const strategy = createSlidingWindowStrategy({ maxTurns: 3 })

  it('keeps messages within maxTurns', () => {
    const messages = Array.from({ length: 10 }, (_, i) => ({
      role: (i % 2 === 0 ? 'user' : 'assistant') as const,
      content: `Message ${i}`,
    }))
    const trimmed = strategy.trim(messages, 100_000)
    expect(trimmed.length).toBeLessThanOrEqual(6) // 3 turns = 6 messages
  })

  it('preserves system messages', () => {
    const messages = [
      { role: 'system' as const, content: 'You are helpful' },
      { role: 'user' as const, content: 'old message' },
      { role: 'assistant' as const, content: 'old reply' },
      { role: 'user' as const, content: 'new message' },
      { role: 'assistant' as const, content: 'new reply' },
    ]
    const strategy1 = createSlidingWindowStrategy({ maxTurns: 1 })
    const trimmed = strategy1.trim(messages, 100_000)
    expect(trimmed[0]).toEqual({ role: 'system', content: 'You are helpful' })
    expect(trimmed).toHaveLength(3) // system + 1 turn (user+assistant)
  })

  it('returns all messages if within limits', () => {
    const messages = [
      { role: 'user' as const, content: 'hi' },
      { role: 'assistant' as const, content: 'hello' },
    ]
    const trimmed = strategy.trim(messages, 100_000)
    expect(trimmed).toEqual(messages)
  })
})
