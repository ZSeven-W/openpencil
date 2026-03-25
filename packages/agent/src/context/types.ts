import type { ModelMessage } from 'ai'

export interface ContextStrategy {
  trim(messages: ModelMessage[], maxTokens: number): ModelMessage[]
}
