import type { KeyframeV2, AnimatableValue } from '@/types/animation'

export interface EffectParameter {
  key: string
  type: 'number' | 'select' | 'direction'
  default: unknown
  label: string
  options?: Array<{ label: string; value: unknown }>
}

export interface EffectGenerateConfig {
  duration: number
  params: Record<string, unknown>
  currentState: Record<string, AnimatableValue>
}

export interface EffectDescriptor {
  id: string
  name: string
  category: 'enter' | 'exit' | 'emphasis' | 'transition' | 'video' | 'custom'
  properties: string[]
  parameters: EffectParameter[]
  defaultDuration: number
  generate: (config: EffectGenerateConfig) => KeyframeV2[]
}

const effectRegistry = new Map<string, EffectDescriptor>()

export function registerEffect(desc: EffectDescriptor): void {
  if (effectRegistry.has(desc.id)) {
    throw new Error(`Effect "${desc.id}" is already registered. Unregister it first or use a different id.`)
  }
  effectRegistry.set(desc.id, desc)
}

export function getEffect(id: string): EffectDescriptor | undefined {
  return effectRegistry.get(id)
}

export function getAllEffects(): EffectDescriptor[] {
  return Array.from(effectRegistry.values())
}

export function getEffectsByCategory(category: EffectDescriptor['category']): EffectDescriptor[] {
  return getAllEffects().filter(e => e.category === category)
}

/**
 * Generate an AnimationClipData from an effect.
 */
export function generateClipFromEffect(
  effectId: string,
  duration?: number,
  params?: Record<string, unknown>,
  currentState?: Record<string, AnimatableValue>,
): { keyframes: KeyframeV2[]; duration: number } | null {
  const effect = effectRegistry.get(effectId)
  if (!effect) return null

  const resolvedDuration = duration ?? effect.defaultDuration
  const resolvedParams: Record<string, unknown> = {}
  for (const param of effect.parameters) {
    resolvedParams[param.key] = params?.[param.key] ?? param.default
  }

  const keyframes = effect.generate({
    duration: resolvedDuration,
    params: resolvedParams,
    currentState: currentState ?? {},
  })

  return { keyframes, duration: resolvedDuration }
}
