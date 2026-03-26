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
    // 1 logical turn = last user or assistant message group
    // The last turn starts at "new message" (user) → keeps user + assistant = 2 + system = 3
    // OR starts at "new reply" (assistant) → keeps just 1 + system = 2
    // Our impl: turnStarts = [user@0, assistant@1, user@2, assistant@3] → keep last 1 → from index 3 → assistant only
    // So: system + assistant("new reply") = 2
    expect(trimmed.length).toBeLessThanOrEqual(3)
  })

  it('returns all messages if within limits', () => {
    const messages = [
      { role: 'user' as const, content: 'hi' },
      { role: 'assistant' as const, content: 'hello' },
    ]
    const trimmed = strategy.trim(messages, 100_000)
    expect(trimmed).toEqual(messages)
  })

  it('never splits assistant+tool groups', () => {
    // Simulate: user, assistant(tool-call), tool, tool, assistant(text), user, assistant(tool-call), tool
    const messages = [
      { role: 'user' as const, content: 'first request' },
      { role: 'assistant' as const, content: 'calling tools' },
      { role: 'tool' as const, content: [{ type: 'tool-result' as const, toolCallId: 't1', toolName: 'foo', output: { type: 'text' as const, value: 'r1' } }] },
      { role: 'tool' as const, content: [{ type: 'tool-result' as const, toolCallId: 't2', toolName: 'bar', output: { type: 'text' as const, value: 'r2' } }] },
      { role: 'assistant' as const, content: 'done with first' },
      { role: 'user' as const, content: 'second request' },
      { role: 'assistant' as const, content: 'calling more tools' },
      { role: 'tool' as const, content: [{ type: 'tool-result' as const, toolCallId: 't3', toolName: 'baz', output: { type: 'text' as const, value: 'r3' } }] },
    ]

    const strategy2 = createSlidingWindowStrategy({ maxTurns: 2 })
    const trimmed = strategy2.trim(messages as any, 100_000)

    // First message should be user or assistant, NEVER tool
    expect(trimmed[0].role).not.toBe('tool')
    // Should keep last 2 logical turns
    expect(trimmed.length).toBeGreaterThanOrEqual(3)
  })

  it('first kept message is never a tool message', () => {
    const messages = [
      { role: 'user' as const, content: 'q1' },
      { role: 'assistant' as const, content: 'a1' },
      { role: 'tool' as const, content: [{ type: 'tool-result' as const, toolCallId: 't1', toolName: 'x', output: { type: 'text' as const, value: '' } }] },
      { role: 'user' as const, content: 'q2' },
      { role: 'assistant' as const, content: 'a2' },
      { role: 'tool' as const, content: [{ type: 'tool-result' as const, toolCallId: 't2', toolName: 'y', output: { type: 'text' as const, value: '' } }] },
      { role: 'user' as const, content: 'q3' },
      { role: 'assistant' as const, content: 'a3' },
    ]

    const strategy1 = createSlidingWindowStrategy({ maxTurns: 1 })
    const trimmed = strategy1.trim(messages as any, 100_000)
    // Should never start with tool
    expect(trimmed[0].role).not.toBe('tool')
  })
})
