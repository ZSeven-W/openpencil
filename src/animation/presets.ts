import type {
  AnimatableProperties,
  AnimationPhases,
  AnimationPresetName,
  EasingPreset,
  Keyframe,
  PresetConfig,
  SlideDirection,
} from '@/types/animation'
import { CANVAS_WIDTH, CANVAS_HEIGHT } from '@/types/animation'

interface PresetResult {
  keyframes: Omit<Keyframe, 'id'>[]
  phases: AnimationPhases
}

// --- Phase Timing ---

function defaultPhases(totalDuration: number): AnimationPhases {
  const inDuration = Math.min(500, totalDuration * 0.15)
  const outDuration = Math.min(500, totalDuration * 0.15)
  const whileDuration = totalDuration - inDuration - outDuration
  return {
    in: { start: 0, duration: inDuration },
    while: { start: inDuration, duration: whileDuration },
    out: { start: inDuration + whileDuration, duration: outDuration },
  }
}

// --- Slide offset calculation ---

function getSlideOffset(
  state: AnimatableProperties,
  direction: SlideDirection,
): { x: number; y: number } {
  // Slide from offscreen based on direction
  const margin = 100
  switch (direction) {
    case 'left':
      return { x: -(state.x + margin), y: 0 }
    case 'right':
      return { x: CANVAS_WIDTH + margin - state.x, y: 0 }
    case 'top':
      return { x: 0, y: -(state.y + margin) }
    case 'bottom':
      return { x: 0, y: CANVAS_HEIGHT + margin - state.y }
  }
}

// --- Preset Generators ---

function generateFade(
  state: AnimatableProperties,
  totalDuration: number,
  easing: EasingPreset,
): PresetResult {
  const phases = defaultPhases(totalDuration)

  return {
    phases,
    keyframes: [
      // In: opacity 0 → 1
      { time: 0, properties: { opacity: 0 }, easing, phase: 'in' as const },
      { time: phases.in.duration, properties: { opacity: 1 }, easing, phase: 'in' as const },
      // While: subtle scale pulse
      {
        time: phases.in.duration + phases.while.duration * 0.5,
        properties: { scaleX: state.scaleX * 1.02, scaleY: state.scaleY * 1.02 },
        easing: 'smooth',
        phase: 'while' as const,
      },
      {
        time: phases.in.duration + phases.while.duration,
        properties: { scaleX: state.scaleX, scaleY: state.scaleY },
        easing: 'smooth',
        phase: 'while' as const,
      },
      // Out: opacity 1 → 0
      {
        time: phases.in.duration + phases.while.duration,
        properties: { opacity: 1 },
        easing,
        phase: 'out' as const,
      },
      { time: totalDuration, properties: { opacity: 0 }, easing, phase: 'out' as const },
    ],
  }
}

function generateSlide(
  state: AnimatableProperties,
  totalDuration: number,
  easing: EasingPreset,
  direction: SlideDirection,
): PresetResult {
  const phases = defaultPhases(totalDuration)
  const offset = getSlideOffset(state, direction)

  // Opposite direction for exit
  const exitOffset = { x: -offset.x, y: -offset.y }

  return {
    phases,
    keyframes: [
      // In: slide from offscreen to current position
      {
        time: 0,
        properties: { x: state.x + offset.x, y: state.y + offset.y },
        easing,
        phase: 'in' as const,
      },
      {
        time: phases.in.duration,
        properties: { x: state.x, y: state.y },
        easing,
        phase: 'in' as const,
      },
      // Out: slide to opposite offscreen
      {
        time: phases.in.duration + phases.while.duration,
        properties: { x: state.x, y: state.y },
        easing,
        phase: 'out' as const,
      },
      {
        time: totalDuration,
        properties: { x: state.x + exitOffset.x, y: state.y + exitOffset.y },
        easing,
        phase: 'out' as const,
      },
    ],
  }
}

function generateScale(
  state: AnimatableProperties,
  totalDuration: number,
  easing: EasingPreset,
): PresetResult {
  const phases = defaultPhases(totalDuration)

  return {
    phases,
    keyframes: [
      // In: scale from 0 to current
      { time: 0, properties: { scaleX: 0, scaleY: 0, opacity: 0 }, easing, phase: 'in' as const },
      {
        time: phases.in.duration,
        properties: { scaleX: state.scaleX, scaleY: state.scaleY, opacity: 1 },
        easing,
        phase: 'in' as const,
      },
      // While: subtle breathe
      {
        time: phases.in.duration + phases.while.duration * 0.5,
        properties: { scaleX: state.scaleX * 1.03, scaleY: state.scaleY * 1.03 },
        easing: 'smooth',
        phase: 'while' as const,
      },
      {
        time: phases.in.duration + phases.while.duration,
        properties: { scaleX: state.scaleX, scaleY: state.scaleY },
        easing: 'smooth',
        phase: 'while' as const,
      },
      // Out: scale to 0
      {
        time: phases.in.duration + phases.while.duration,
        properties: { scaleX: state.scaleX, scaleY: state.scaleY, opacity: 1 },
        easing,
        phase: 'out' as const,
      },
      {
        time: totalDuration,
        properties: { scaleX: 0, scaleY: 0, opacity: 0 },
        easing,
        phase: 'out' as const,
      },
    ],
  }
}

function generateBounce(
  state: AnimatableProperties,
  totalDuration: number,
  easing: EasingPreset,
): PresetResult {
  const phases = defaultPhases(totalDuration)
  const inEnd = phases.in.duration
  const whileEnd = inEnd + phases.while.duration

  return {
    phases,
    keyframes: [
      // In: overshoot and settle
      { time: 0, properties: { scaleX: 0, scaleY: 0, opacity: 0 }, easing: 'bouncy', phase: 'in' as const },
      {
        time: inEnd * 0.6,
        properties: {
          scaleX: state.scaleX * 1.15,
          scaleY: state.scaleY * 1.15,
          opacity: 1,
        },
        easing: 'bouncy',
        phase: 'in' as const,
      },
      {
        time: inEnd,
        properties: { scaleX: state.scaleX, scaleY: state.scaleY, opacity: 1 },
        easing: 'smooth',
        phase: 'in' as const,
      },
      // While: gentle bob (y-axis)
      {
        time: inEnd + phases.while.duration * 0.5,
        properties: { y: state.y - 8 },
        easing: 'smooth',
        phase: 'while' as const,
      },
      {
        time: whileEnd,
        properties: { y: state.y },
        easing: 'smooth',
        phase: 'while' as const,
      },
      // Out: compress then exit
      {
        time: whileEnd,
        properties: {
          scaleX: state.scaleX,
          scaleY: state.scaleY,
          opacity: 1,
        },
        easing,
        phase: 'out' as const,
      },
      {
        time: whileEnd + phases.out.duration * 0.3,
        properties: {
          scaleX: state.scaleX * 1.1,
          scaleY: state.scaleY * 0.9,
          opacity: 1,
        },
        easing: 'snappy',
        phase: 'out' as const,
      },
      {
        time: totalDuration,
        properties: { scaleX: 0, scaleY: 0, opacity: 0 },
        easing: 'snappy',
        phase: 'out' as const,
      },
    ],
  }
}

// --- Public API ---

const presetGenerators: Record<
  AnimationPresetName,
  (
    state: AnimatableProperties,
    totalDuration: number,
    easing: EasingPreset,
    direction: SlideDirection,
  ) => PresetResult
> = {
  fade: (state, dur, easing) => generateFade(state, dur, easing),
  slide: (state, dur, easing, dir) => generateSlide(state, dur, easing, dir),
  scale: (state, dur, easing) => generateScale(state, dur, easing),
  bounce: (state, dur, easing) => generateBounce(state, dur, easing),
}

export function generatePresetKeyframes(
  presetName: AnimationPresetName,
  nodeState: AnimatableProperties,
  totalDuration: number,
  config?: PresetConfig,
): PresetResult {
  const easing = config?.easing ?? 'smooth'
  const direction = config?.direction ?? 'left'
  return presetGenerators[presetName](nodeState, totalDuration, easing, direction)
}
