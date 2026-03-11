/**
 * Types and utilities for bridging Zustand stores ↔ react-timeline-editor.
 *
 * Convention: library uses seconds, stores use milliseconds.
 * Variables use _s / _ms suffixes for clarity.
 */

import type { AnimationTrack, Keyframe, KeyframePhase } from '@/types/animation'
import type { PenNode, VideoNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Time conversion
// ---------------------------------------------------------------------------

export function msToSec(ms: number): number {
  return Math.round(ms) / 1000
}

export function secToMs(s: number): number {
  return Math.round(s * 1000)
}

// ---------------------------------------------------------------------------
// Action metadata (kept separate from library's TimelineAction)
// ---------------------------------------------------------------------------

export interface AnimationPhaseMetadata {
  type: 'animation-phase'
  phase: KeyframePhase
  nodeId: string
}

export interface VideoClipMetadata {
  type: 'video-clip'
  nodeId: string
}

export type ActionMetadata = AnimationPhaseMetadata | VideoClipMetadata

export type ActionMetadataMap = Map<string, ActionMetadata>

// ---------------------------------------------------------------------------
// Effect IDs (used as effectId on TimelineAction, also discriminates metadata)
// ---------------------------------------------------------------------------

export const EFFECT_ANIMATION_PHASE = 'animation-phase' as const
export const EFFECT_VIDEO_CLIP = 'video-clip' as const

// ---------------------------------------------------------------------------
// TimelineStores interface (dependency injection for testability)
// ---------------------------------------------------------------------------

export interface TimelineStores {
  getTimelineState: () => {
    tracks: Record<string, AnimationTrack>
    duration: number
    videoClipIds: string[]
  }
  updateKeyframe: (
    nodeId: string,
    keyframeId: string,
    updates: Partial<Pick<Keyframe, 'time' | 'properties' | 'easing'>>,
  ) => void
  getDocumentState: () => {
    getNodeById: (id: string) => PenNode | undefined
  }
  updateNode: (id: string, partial: Partial<PenNode>) => void
}

// ---------------------------------------------------------------------------
// Video node input type (minimal projection for adapter)
// ---------------------------------------------------------------------------

export type VideoNodeProjection = Pick<
  VideoNode,
  'id' | 'inPoint' | 'outPoint' | 'timelineOffset' | 'videoDuration' | 'name'
>
