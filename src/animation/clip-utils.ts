import type { AnimationClipData } from '@/types/animation'

/** Duration of the hold portion (between in and out effects) */
export function getHoldDuration(clip: AnimationClipData): number {
  return clip.duration - (clip.inEffect?.duration ?? 0) - (clip.outEffect?.duration ?? 0)
}

/** Absolute time when the in-effect ends (hold begins) */
export function getInEnd(clip: AnimationClipData): number {
  return clip.startTime + (clip.inEffect?.duration ?? 0)
}

/** Absolute time when the out-effect starts (hold ends) */
export function getOutStart(clip: AnimationClipData): number {
  return clip.startTime + clip.duration - (clip.outEffect?.duration ?? 0)
}
