import { describe, it, expect } from 'vitest'
import {
  DEFAULT_IMAGE_GEN_CONFIG,
  MODEL_PLACEHOLDERS,
  type ImageSearchResult,
  type ImageGenProvider,
} from '../image-service'

describe('image-service types', () => {
  it('DEFAULT_IMAGE_GEN_CONFIG has expected shape', () => {
    expect(DEFAULT_IMAGE_GEN_CONFIG.provider).toBe('openai')
    expect(DEFAULT_IMAGE_GEN_CONFIG.apiKey).toBe('')
    expect(DEFAULT_IMAGE_GEN_CONFIG.model).toBe('')
  })

  it('MODEL_PLACEHOLDERS covers all providers', () => {
    const providers: ImageGenProvider[] = ['openai', 'gemini', 'replicate', 'custom']
    for (const p of providers) {
      expect(MODEL_PLACEHOLDERS[p]).toBeTruthy()
    }
  })

  it('ImageSearchResult shape is correct', () => {
    const result: ImageSearchResult = {
      id: 'test',
      url: 'https://example.com/img.jpg',
      thumbUrl: 'https://example.com/thumb.jpg',
      width: 800,
      height: 600,
      source: 'openverse',
      license: 'CC BY 2.0',
    }
    expect(result.source).toBe('openverse')
  })
})
