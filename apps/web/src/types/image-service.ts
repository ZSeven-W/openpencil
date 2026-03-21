export type ImageGenProvider = 'openai' | 'gemini' | 'replicate' | 'custom'

export interface ImageGenConfig {
  provider: ImageGenProvider
  apiKey: string
  model: string
  baseUrl?: string
}

export interface ImageGenProfile extends ImageGenConfig {
  id: string
  name: string
}

export interface ImageSearchResult {
  id: string
  url: string
  thumbUrl: string
  width: number
  height: number
  source: 'openverse' | 'wikimedia'
  license: string
  attribution?: string
}

export interface ImageSearchResponse {
  results: ImageSearchResult[]
  source: 'openverse' | 'wikimedia'
}

export const MODEL_PLACEHOLDERS: Record<ImageGenProvider, string> = {
  openai: 'dall-e-3',
  gemini: 'gemini-2.0-flash-preview-image-generation',
  replicate: 'black-forest-labs/flux-1.1-pro',
  custom: 'model-name',
}

export const DEFAULT_IMAGE_GEN_CONFIG: ImageGenConfig = {
  provider: 'openai',
  apiKey: '',
  model: '',
  baseUrl: undefined,
}
