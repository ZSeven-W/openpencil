/**
 * Types and utilities for bridging Zustand stores ↔ react-timeline-editor.
 *
 * Convention: library uses seconds, stores use milliseconds.
 * Variables use _s / _ms suffixes for clarity.
 */

import type { PenNode } from '@/types/pen'

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

export interface VideoClipMetadata {
  type: 'video-clip'
  nodeId: string
}

export interface AnimationClipMetadata {
  type: 'animation-clip'
  nodeId: string
  clipId: string
}

export type ActionMetadata = VideoClipMetadata | AnimationClipMetadata

export type ActionMetadataMap = Map<string, ActionMetadata>

// ---------------------------------------------------------------------------
// Effect IDs (used as effectId on TimelineAction, also discriminates metadata)
// ---------------------------------------------------------------------------

export const EFFECT_VIDEO_CLIP = 'video-clip' as const
export const EFFECT_ANIMATION_CLIP = 'animation-clip' as const

// ---------------------------------------------------------------------------
// TimelineStores interface (dependency injection for testability)
// ---------------------------------------------------------------------------

export interface TimelineStores {
  getDocumentState: () => {
    getNodeById: (id: string) => PenNode | undefined
  }
  updateNode: (id: string, partial: Partial<PenNode>) => void
  getDuration: () => number
}
