import type { VariableDefinition } from './variables'

export interface VibeKit {
  id: string
  name: string
  description?: string
  version: string
  sourceUrl?: string
  themes?: Record<string, string[]>
  variables: Record<string, VariableDefinition>
  assets: Record<string, VibeAsset>
  metadata: {
    createdAt: string
    extractedFrom?: string
    generatedBy?: 'extraction' | 'ai' | 'manual'
  }
}

export interface VibeAsset {
  type: 'texture' | 'lut' | 'sfx'
  url: string
  mimeType: string
  size?: number
}
