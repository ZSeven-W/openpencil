import { createOpenAI } from '@ai-sdk/openai';
import type { AgentProvider } from './types';

const DEFAULT_MAX_CONTEXT = 128_000;

export interface OllamaProviderConfig {
  model: string;
  baseURL?: string;
  maxContextTokens?: number;
}

export function createOllamaProvider(config: OllamaProviderConfig): AgentProvider {
  const ollama = createOpenAI({
    apiKey: 'ollama',
    baseURL: config.baseURL ?? 'http://localhost:11434/v1',
  });
  return {
    model: ollama(config.model),
    id: 'ollama',
    maxContextTokens: config.maxContextTokens ?? DEFAULT_MAX_CONTEXT,
    supportsThinking: false,
  };
}
