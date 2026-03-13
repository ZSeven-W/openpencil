/**
 * Pure transform functions: Zustand stores ↔ react-timeline-editor.
 *
 * The library never owns data at rest. It receives a projection
 * (TimelineRow[]) and reports mutations back via callbacks.
 */

import type { TimelineRow, TimelineAction } from '@cyca/react-timeline-editor'
import type { AnimationClipData, VideoClipData } from '@/types/animation'
import type { PenNode } from '@/types/pen'
import {
  msToSec,
  secToMs,
  EFFECT_VIDEO_CLIP,
  EFFECT_ANIMATION_CLIP,
  type ActionMetadataMap,
  type TimelineStores,
} from './timeline-adapter-types'

// ---------------------------------------------------------------------------
// Store → Library projection
// ---------------------------------------------------------------------------

export interface TimelineProjection {
  rows: TimelineRow[]
  metadata: ActionMetadataMap
}

// ---------------------------------------------------------------------------
// Library → Store mutations (v2 clip-based)
// ---------------------------------------------------------------------------

/**
 * Apply a move operation from the library back to the correct store.
 * Called from onActionMoveEnd — NOT during drag.
 */
export function applyActionMove(
  actionId: string,
  newStart_s: number,
  _newEnd_s: number,
  metadata: ActionMetadataMap,
  stores: TimelineStores,
): void {
  const meta = metadata.get(actionId)
  if (!meta) return

  if (meta.type === 'clip') {
    applyClipMove(meta.nodeId, meta.clipId, newStart_s, stores)
  } else if (meta.type === 'animation-clip') {
    applyClipMove(meta.nodeId, meta.clipId, newStart_s, stores)
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
  _dir: 'left' | 'right',
  metadata: ActionMetadataMap,
  stores: TimelineStores,
): void {
  const meta = metadata.get(actionId)
  if (!meta) return

  if (meta.type === 'clip') {
    applyClipResize(meta.nodeId, meta.clipId, newStart_s, newEnd_s, stores)
  } else if (meta.type === 'animation-clip') {
    applyClipResize(meta.nodeId, meta.clipId, newStart_s, newEnd_s, stores)
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
  _stores: TimelineStores,
): boolean {
  if (start_s >= end_s) return false
  if (secToMs(end_s - start_s) < MIN_DURATION_MS) return false
  if (start_s < 0) return false

  const meta = metadata.get(actionId)
  if (!meta) return false

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
  return validateActionMove(actionId, start_s, end_s, metadata, stores)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function applyClipMove(
  nodeId: string,
  clipId: string,
  newStart_s: number,
  stores: TimelineStores,
): void {
  const node = stores.getDocumentState().getNodeById(nodeId)
  if (!node?.clips) return

  const newStartMs = secToMs(newStart_s)
  const updatedClips = node.clips.map((c) =>
    c.id === clipId ? { ...c, startTime: newStartMs } : c,
  )
  stores.updateNode(nodeId, { clips: updatedClips } as Partial<PenNode>)
}

function applyClipResize(
  nodeId: string,
  clipId: string,
  newStart_s: number,
  newEnd_s: number,
  stores: TimelineStores,
): void {
  const node = stores.getDocumentState().getNodeById(nodeId)
  if (!node?.clips) return

  const newStartMs = secToMs(newStart_s)
  const newDurationMs = secToMs(newEnd_s) - newStartMs
  const updatedClips = node.clips.map((c) =>
    c.id === clipId ? { ...c, startTime: newStartMs, duration: newDurationMs } : c,
  )
  stores.updateNode(nodeId, { clips: updatedClips } as Partial<PenNode>)
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
            type: 'clip',
            nodeId: node.id,
            clipId: clip.id,
            clipKind: clip.kind,
          })
        }

        rows.push({ id: `v2::${node.id}`, actions })
      }

      if ('children' in node && node.children) {
        walk(node.children as PenNode[])
      }
    }
  }

  walk(nodes)
  return { rows, metadata }
}
