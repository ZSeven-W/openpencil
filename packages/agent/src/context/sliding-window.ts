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
      const maxMessages = options.maxTurns * 2
      const kept = nonSystem.length <= maxMessages
        ? nonSystem
        : nonSystem.slice(-maxMessages)
      return [...systemMessages, ...kept]
    },
  }
}
