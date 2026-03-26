import type { ModelMessage } from 'ai'
import type { ContextStrategy } from './types'

export interface SlidingWindowOptions {
  maxTurns: number
}

export function createSlidingWindowStrategy(options: SlidingWindowOptions): ContextStrategy {
  return {
    trim(messages: ModelMessage[], _maxTokens: number): ModelMessage[] {
      const systemMessages = messages.filter(m => m.role === 'system')
      const nonSystem = messages.filter(m => m.role !== 'system')

      // Count "logical turns" — a turn starts at a user or assistant message
      // and includes all following tool messages that belong to it.
      // We must never split an assistant+tool group.
      const turnStarts: number[] = []
      for (let i = 0; i < nonSystem.length; i++) {
        const role = nonSystem[i].role
        if (role === 'user' || role === 'assistant') {
          turnStarts.push(i)
        }
        // 'tool' messages belong to the preceding assistant turn — skip
      }

      if (turnStarts.length <= options.maxTurns) {
        return [...systemMessages, ...nonSystem]
      }

      // Keep the last N turns (by their start indices)
      const keepFrom = turnStarts[turnStarts.length - options.maxTurns]
      const kept = nonSystem.slice(keepFrom)

      // Ensure the first kept message is user or assistant, never tool
      // (should already be the case since we cut at turnStarts)
      return [...systemMessages, ...kept]
    },
  }
}
