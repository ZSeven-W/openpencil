import { describe, it, expect } from 'vitest';
import { createOllamaProvider } from '../../src/providers/ollama';

describe('createOllamaProvider', () => {
  it('creates a provider with correct metadata', () => {
    const provider = createOllamaProvider({ model: 'llama3' });
    expect(provider.id).toBe('ollama');
    expect(provider.supportsThinking).toBe(false);
    expect(provider.maxContextTokens).toBe(128_000);
    expect(provider.model).toBeDefined();
  });

  it('allows custom baseURL', () => {
    const provider = createOllamaProvider({
      model: 'codellama',
      baseURL: 'http://192.168.1.100:11434/v1',
    });
    expect(provider.id).toBe('ollama');
  });

  it('allows custom maxContextTokens', () => {
    const provider = createOllamaProvider({
      model: 'llama3',
      maxContextTokens: 32_000,
    });
    expect(provider.maxContextTokens).toBe(32_000);
  });
});
