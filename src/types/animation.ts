// ============================================================
// Animation Types v2
// ============================================================

// --- v2: Cubic Bezier ---

/** Cubic bezier control points [x1, y1, x2, y2] */
export type CubicBezier = [number, number, number, number]

/** Named easing presets mapped to bezier values */
export type EasingPresetV2 =
  | 'linear'
  | 'ease'
  | 'easeIn'
  | 'easeOut'
  | 'easeInOut'
  | 'snappy'
  | 'bouncy'
  | 'gentle'
  | 'smooth'

/** Easing can be a named preset or custom bezier */
export type Easing = EasingPresetV2 | CubicBezier

/** Hex color string for type-safe color values */
export type HexColor = `#${string}`

/** Values that can be animated */
export type AnimatableValue = number | HexColor

// --- v2: Keyframes ---

/** A single keyframe within a clip (v2 — offset-based) */
export interface KeyframeV2 {
  id: string
  offset: number // 0.0–1.0 (percentage-based, duration-independent)
  properties: Record<string, AnimatableValue>
  easing: Easing
}

// --- v2: Clips ---

/** Timed effect config — for in/out transitions with a required duration */
export interface TimedEffectConfig {
  effectId: string
  duration: number // ms — how long the transition lasts
  params?: Record<string, unknown>
}

/** Base clip fields shared by all clip kinds */
export interface ClipBase {
  id: string
  startTime: number // ms
  duration: number // ms
  extrapolate?: 'clamp' | 'hold'
}

/** Animation clip — keyframe-driven property animation */
export interface AnimationClipData extends ClipBase {
  kind: 'animation'
  /** @deprecated Use inEffect/outEffect instead. Kept for migration. */
  effectId?: string
  keyframes: KeyframeV2[]
  /** @deprecated Use effect config params instead. */
  params?: Record<string, unknown>
  inEffect?: TimedEffectConfig
  outEffect?: TimedEffectConfig
}

/** Video clip — source media playback */
export interface VideoClipData extends ClipBase {
  kind: 'video'
  sourceStart: number // ms
  sourceEnd: number // ms
  playbackRate: number
}

/** Discriminated union of clip kinds */
export type Clip = AnimationClipData | VideoClipData

/** @deprecated Use Clip instead */
export type AnimationClip = Clip

/** Type guard for video clips */
export function isVideoClip(clip: Clip): clip is VideoClipData {
  return clip.kind === 'video'
}

/** Type guard for animation clips */
export function isAnimationClip(clip: Clip): clip is AnimationClipData {
  return clip.kind === 'animation'
}

// --- v2: Composition ---

/** Global composition settings */
export interface CompositionSettings {
  duration: number // ms
  fps: number
}

// ============================================================
// Playback state (shared between timeline-store and controllers)
// ============================================================

export type PlaybackMode = 'idle' | 'playing' | 'scrubbing'
