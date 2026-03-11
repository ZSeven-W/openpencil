/**
 * Pure transform functions: Zustand stores ↔ react-timeline-editor.
 *
 * The library never owns data at rest. It receives a projection
 * (TimelineRow[]) and reports mutations back via callbacks.
 */

import type { TimelineRow, TimelineAction } from '@cyca/react-timeline-editor'
import type { AnimationTrack, KeyframePhase, AnimationClipData, VideoClipData } from '@/types/animation'
import type { PenNode } from '@/types/pen'
import {
  msToSec,
  secToMs,
  EFFECT_ANIMATION_PHASE,
  EFFECT_VIDEO_CLIP,
  EFFECT_ANIMATION_CLIP,
  type ActionMetadataMap,
  type TimelineStores,
  type VideoNodeProjection,
} from './timeline-adapter-types'

// ---------------------------------------------------------------------------
// Store → Library projection
// ---------------------------------------------------------------------------

export interface TimelineProjection {
  rows: TimelineRow[]
  metadata: ActionMetadataMap
}

/**
 * Convert Zustand track/video data into TimelineRow[] + ActionMetadataMap.
 * Pure function — no side effects, no store reads.
 */
export function toTimelineRows(
  tracks: Record<string, AnimationTrack>,
  videoNodes: readonly VideoNodeProjection[],
  duration_ms: number,
): TimelineProjection {
  const rows: TimelineRow[] = []
  const metadata: ActionMetadataMap = new Map()

  // Animation tracks: each phase becomes one action
  for (const track of Object.values(tracks)) {
    const actions: TimelineAction[] = []
    const phases: KeyframePhase[] = ['in', 'while', 'out']

    for (const phase of phases) {
      const phaseData = track.phases[phase]
      if (phaseData.duration <= 0 && phase !== 'while') continue

      const actionId = `${track.nodeId}-${phase}`
      const start_ms = phaseData.start
      const end_ms = phaseData.start + phaseData.duration

      actions.push({
        id: actionId,
        start: msToSec(start_ms),
        end: msToSec(end_ms),
        effectId: EFFECT_ANIMATION_PHASE,
        flexible: true,
        movable: true,
      })

      metadata.set(actionId, {
        type: 'animation-phase',
        phase,
        nodeId: track.nodeId,
      })
    }

    if (actions.length > 0) {
      rows.push({
        id: track.nodeId,
        actions,
      })
    }
  }

  // Video clips: each clip becomes one row with one action
  for (const node of videoNodes) {
    const inPoint_ms = node.inPoint ?? 0
    const outPoint_ms = node.outPoint ?? (node.videoDuration ?? duration_ms)
    const offset_ms = node.timelineOffset ?? 0
    const clipDuration_ms = outPoint_ms - inPoint_ms

    const actionId = `${node.id}-video`

    rows.push({
      id: node.id,
      actions: [
        {
          id: actionId,
          start: msToSec(offset_ms),
          end: msToSec(offset_ms + clipDuration_ms),
          effectId: EFFECT_VIDEO_CLIP,
          flexible: true,
          movable: true,
        },
      ],
    })

    metadata.set(actionId, {
      type: 'video-clip',
      nodeId: node.id,
    })
  }

  return { rows, metadata }
}

// ---------------------------------------------------------------------------
// Library → Store mutations
// ---------------------------------------------------------------------------

/**
 * Apply a move operation from the library back to the correct store.
 * Called from onActionMoveEnd — NOT during drag.
 */
export function applyActionMove(
  actionId: string,
  newStart_s: number,
  newEnd_s: number,
  metadata: ActionMetadataMap,
  stores: TimelineStores,
): void {
  const meta = metadata.get(actionId)
  if (!meta) return

  if (meta.type === 'animation-phase') {
    applyPhaseMove(meta.nodeId, meta.phase, newStart_s, newEnd_s, stores)
  } else {
    applyVideoClipMove(meta.nodeId, newStart_s, stores)
  }
}

/**
 * Apply a resize operation from the library back to the correct store.
 * Called from onActionResizeEnd — NOT during drag.
 */
export function applyActionResize(
  actionId: string,
  newStart_s: number,
  newEnd_s: number,
  dir: 'left' | 'right',
  metadata: ActionMetadataMap,
  stores: TimelineStores,
): void {
  const meta = metadata.get(actionId)
  if (!meta) return

  if (meta.type === 'animation-phase') {
    applyPhaseResize(meta.nodeId, meta.phase, newStart_s, newEnd_s, dir, stores)
  } else {
    applyVideoClipResize(meta.nodeId, newStart_s, newEnd_s, dir, stores)
  }
}

// ---------------------------------------------------------------------------
// Validation guards (for onActionMoving / onActionResizing)
// ---------------------------------------------------------------------------

const MIN_DURATION_MS = 50

/** Return false to reject the proposed position */
export function validateActionMove(
  actionId: string,
  start_s: number,
  end_s: number,
  metadata: ActionMetadataMap,
  stores: TimelineStores,
): boolean {
  if (start_s >= end_s) return false
  if (secToMs(end_s - start_s) < MIN_DURATION_MS) return false
  if (start_s < 0) return false

  const meta = metadata.get(actionId)
  if (!meta) return false

  if (meta.type === 'video-clip') {
    return validateVideoClipBounds(meta.nodeId, start_s, end_s, stores)
  }

  // Phase ordering (in < while < out) is enforced at the store level
  // by recomputePhases() which derives boundaries from keyframe tags.

  return true
}

/** Return false to reject the proposed resize */
export function validateActionResize(
  actionId: string,
  start_s: number,
  end_s: number,
  metadata: ActionMetadataMap,
  stores: TimelineStores,
): boolean {
  // Same validation as move
  return validateActionMove(actionId, start_s, end_s, metadata, stores)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function applyPhaseMove(
  nodeId: string,
  phase: KeyframePhase,
  newStart_s: number,
  _newEnd_s: number,
  stores: TimelineStores,
): void {
  const { tracks } = stores.getTimelineState()
  const track = tracks[nodeId]
  if (!track) return

  const oldPhase = track.phases[phase]
  const oldStart_ms = oldPhase.start
  const newStart_ms = secToMs(newStart_s)
  const delta_ms = newStart_ms - oldStart_ms

  // Move all keyframes in this phase by the same delta
  for (const kf of track.keyframes) {
    if (kf.phase === phase) {
      stores.updateKeyframe(nodeId, kf.id, {
        time: kf.time + delta_ms,
      })
    }
  }
}

function applyPhaseResize(
  nodeId: string,
  phase: KeyframePhase,
  newStart_s: number,
  newEnd_s: number,
  dir: 'left' | 'right',
  stores: TimelineStores,
): void {
  const { tracks } = stores.getTimelineState()
  const track = tracks[nodeId]
  if (!track) return

  const phaseKeyframes = track.keyframes.filter((kf) => kf.phase === phase)
  if (phaseKeyframes.length === 0) return

  if (dir === 'left') {
    // Move the first keyframe in the phase to match new start
    const first = phaseKeyframes[0]
    if (first) {
      stores.updateKeyframe(nodeId, first.id, {
        time: secToMs(newStart_s),
      })
    }
  } else {
    // Move the last keyframe in the phase to match new end
    const last = phaseKeyframes[phaseKeyframes.length - 1]
    if (last) {
      stores.updateKeyframe(nodeId, last.id, {
        time: secToMs(newEnd_s),
      })
    }
  }
}

function applyVideoClipMove(
  nodeId: string,
  newStart_s: number,
  stores: TimelineStores,
): void {
  stores.updateNode(nodeId, {
    timelineOffset: secToMs(newStart_s),
  } as Partial<import('@/types/pen').PenNode>)
}

function applyVideoClipResize(
  nodeId: string,
  newStart_s: number,
  newEnd_s: number,
  dir: 'left' | 'right',
  stores: TimelineStores,
): void {
  const node = stores.getDocumentState().getNodeById(nodeId)
  if (!node || node.type !== 'video') return

  const videoNode = node as import('@/types/pen').VideoNode
  const offset_ms = videoNode.timelineOffset ?? 0
  const inPoint_ms = videoNode.inPoint ?? 0

  if (dir === 'left') {
    // Left resize changes inPoint and timelineOffset
    const newOffset_ms = secToMs(newStart_s)
    const offsetDelta_ms = newOffset_ms - offset_ms
    stores.updateNode(nodeId, {
      timelineOffset: newOffset_ms,
      inPoint: inPoint_ms + offsetDelta_ms,
    } as Partial<import('@/types/pen').PenNode>)
  } else {
    // Right resize changes outPoint
    const newEnd_ms = secToMs(newEnd_s)
    const newOutPoint_ms = inPoint_ms + (newEnd_ms - offset_ms)
    stores.updateNode(nodeId, {
      outPoint: newOutPoint_ms,
    } as Partial<import('@/types/pen').PenNode>)
  }
}

function validateVideoClipBounds(
  nodeId: string,
  start_s: number,
  end_s: number,
  stores: TimelineStores,
): boolean {
  const node = stores.getDocumentState().getNodeById(nodeId)
  if (!node || node.type !== 'video') return false

  const videoNode = node as import('@/types/pen').VideoNode
  const videoDuration_ms = videoNode.videoDuration ?? Infinity
  const clipDuration_ms = secToMs(end_s - start_s)

  // Clip duration can't exceed video duration
  if (clipDuration_ms > videoDuration_ms) return false
  // Can't start before 0
  if (start_s < 0) return false

  return true
}

// ---------------------------------------------------------------------------
// v2: Clip-based timeline rows (reads clips from PenNodes)
// ---------------------------------------------------------------------------

/**
 * Convert a v2 AnimationClipData to a TimelineAction (for the timeline library).
 */
export function clipToTimelineAction(clip: AnimationClipData | VideoClipData): TimelineAction {
  return {
    id: clip.id,
    start: msToSec(clip.startTime),
    end: msToSec(clip.startTime + clip.duration),
    effectId: clip.kind === 'animation' ? EFFECT_ANIMATION_CLIP : EFFECT_VIDEO_CLIP,
    flexible: true,
    movable: true,
  }
}

/**
 * Build timeline rows from nodes with clips (v2 model).
 * Each node with clips becomes a timeline row.
 * Returns rows + metadata for the v2 clip-based actions.
 */
export function buildTimelineRowsFromNodes(nodes: PenNode[]): TimelineProjection {
  const rows: TimelineRow[] = []
  const metadata: ActionMetadataMap = new Map()

  function walk(list: PenNode[]) {
    for (const node of list) {
      if (node.clips && node.clips.length > 0) {
        const actions: TimelineAction[] = []

        for (const clip of node.clips) {
          actions.push(clipToTimelineAction(clip))
          metadata.set(clip.id, {
            type: 'animation-clip',
            nodeId: node.id,
            clipId: clip.id,
          })
        }

        rows.push({ id: `v2-${node.id}`, actions })
      }

      if ('children' in node && node.children) {
        walk(node.children as PenNode[])
      }
    }
  }

  walk(nodes)
  return { rows, metadata }
}

