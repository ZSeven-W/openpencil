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
  effectId?: string
  keyframes: KeyframeV2[]
  params?: Record<string, unknown>
}

/** Video clip — source media playback */
export interface VideoClipData extends ClipBase {
  kind: 'video'
  sourceStart: number // ms
  sourceEnd: number // ms
  playbackRate: number
}

/** Discriminated union of clip kinds */
export type AnimationClip = AnimationClipData | VideoClipData

// --- v2: Composition ---

/** Global composition settings */
export interface CompositionSettings {
  duration: number // ms
  fps: number
}

// ============================================================
// Legacy v1 types (still referenced by existing codebase)
// ============================================================

/** @deprecated Use v2 types */
export interface AnimatableProperties {
  x: number
  y: number
  scaleX: number
  scaleY: number
  rotation: number
  opacity: number
}

/** @deprecated Use EasingPresetV2 */
export type EasingPreset = 'smooth' | 'snappy' | 'bouncy' | 'gentle' | 'linear'

/** @deprecated Use v2 types */
export type KeyframePhase = 'in' | 'while' | 'out'

/** @deprecated Use KeyframeV2 */
export interface Keyframe {
  id: string
  time: number // ms from composition start
  properties: Partial<AnimatableProperties>
  easing: EasingPreset
  phase?: KeyframePhase // which animation phase this keyframe belongs to
}

/** @deprecated Use v2 types */
export interface AnimationPhase {
  start: number // ms offset from track start
  duration: number // ms
}

/** @deprecated Use v2 types */
export interface AnimationPhases {
  in: AnimationPhase
  while: AnimationPhase
  out: AnimationPhase
}

/** @deprecated Use AnimationClip */
export interface AnimationTrack {
  nodeId: string
  keyframes: Keyframe[] // sorted by time
  phases: AnimationPhases
  startDelay: number // ms from composition start
}

/** @deprecated Use CompositionSettings */
export interface TimelineState {
  tracks: Record<string, AnimationTrack> // nodeId → track
  duration: number // total composition duration in ms
  fps: 24 | 30 | 60
}

/** @deprecated Use v2 types */
export type PlaybackMode = 'idle' | 'playing' | 'scrubbing'

/** @deprecated Use v2 types */
export interface PlaybackState {
  currentTime: number
  mode: PlaybackMode
  loopEnabled: boolean
}

/** @deprecated Use v2 types */
export type AnimationPresetName = 'fade' | 'slide' | 'scale' | 'bounce'

/** @deprecated Use v2 types */
export type SlideDirection = 'left' | 'right' | 'top' | 'bottom'

/** @deprecated Use v2 types */
export interface PresetConfig {
  direction?: SlideDirection
  easing?: EasingPreset
}

/** @deprecated Use CompositionSettings */
export const CANVAS_WIDTH = 1080

/** @deprecated Use CompositionSettings */
export const CANVAS_HEIGHT = 1920
