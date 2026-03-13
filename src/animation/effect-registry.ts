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

/**
 * Remap keyframe offsets from effect-local [0,1] to a sub-range of the clip.
 *
 * Effect generators produce offsets 0→1 relative to their own duration.
 * When stored on a clip with In/Hold/Out, we must remap:
 *   In  keyframes: [0, inDuration/clipDuration]
 *   Out keyframes: [(clipDuration - outDuration)/clipDuration, 1]
 */
function remapKeyframeOffsets(
  keyframes: KeyframeV2[],
  rangeStart: number,
  rangeEnd: number,
): KeyframeV2[] {
  const span = rangeEnd - rangeStart
  return keyframes.map((kf) => ({
    ...kf,
    offset: rangeStart + kf.offset * span,
  }))
}

/**
 * Build merged keyframes for a clip with in/out effects.
 * Remaps each effect's [0,1] offsets to the correct sub-range of the clip.
 */
export function buildMergedKeyframes(
  clipDuration: number,
  inEffect: { effectId: string; duration: number; params?: Record<string, unknown> } | undefined,
  outEffect: { effectId: string; duration: number; params?: Record<string, unknown> } | undefined,
  currentState: Record<string, AnimatableValue>,
): KeyframeV2[] {
  const result: KeyframeV2[] = []

  if (inEffect && clipDuration > 0) {
    const gen = generateClipFromEffect(inEffect.effectId, inEffect.duration, inEffect.params, currentState)
    if (gen) {
      const rangeEnd = inEffect.duration / clipDuration
      result.push(...remapKeyframeOffsets(gen.keyframes, 0, rangeEnd))
    }
  }

  if (outEffect && clipDuration > 0) {
    const gen = generateClipFromEffect(outEffect.effectId, outEffect.duration, outEffect.params, currentState)
    if (gen) {
      const rangeStart = (clipDuration - outEffect.duration) / clipDuration
      result.push(...remapKeyframeOffsets(gen.keyframes, rangeStart, 1))
    }
  }

  return result
}
