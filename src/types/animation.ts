// --- Animatable Properties ---

export interface AnimatableProperties {
  x: number
  y: number
  scaleX: number
  scaleY: number
  rotation: number
  opacity: number
}

// --- Easing ---

export type EasingPreset = 'smooth' | 'snappy' | 'bouncy' | 'gentle' | 'linear'

// --- Keyframes ---

export interface Keyframe {
  id: string
  time: number // ms from composition start
  properties: Partial<AnimatableProperties>
  easing: EasingPreset
}

// --- Animation Phases ---

export interface AnimationPhase {
  start: number // ms offset from track start
  duration: number // ms
}

export interface AnimationPhases {
  in: AnimationPhase
  while: AnimationPhase
  out: AnimationPhase
}

// --- Tracks ---

export interface AnimationTrack {
  nodeId: string
  keyframes: Keyframe[] // sorted by time
  phases: AnimationPhases
  startDelay: number // ms from composition start
}

// --- Timeline State (persisted to .op file) ---

export interface TimelineState {
  tracks: Record<string, AnimationTrack> // nodeId → track
  duration: number // total composition duration in ms
  fps: 24 | 30 | 60
}

// --- Playback State (ephemeral, not persisted) ---

export type PlaybackMode = 'idle' | 'playing' | 'scrubbing'

export interface PlaybackState {
  currentTime: number
  mode: PlaybackMode
  loopEnabled: boolean
}

// --- Presets ---

export type AnimationPresetName = 'fade' | 'slide' | 'scale' | 'bounce'

export type SlideDirection = 'left' | 'right' | 'top' | 'bottom'

export interface PresetConfig {
  direction?: SlideDirection
  easing?: EasingPreset
}

// --- Canvas dimensions (fixed 9:16) ---

export const CANVAS_WIDTH = 1080
export const CANVAS_HEIGHT = 1920
