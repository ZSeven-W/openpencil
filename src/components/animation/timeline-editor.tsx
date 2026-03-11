import { useRef, useMemo, useCallback, useEffect } from 'react'
import { Timeline } from '@cyca/react-timeline-editor'
import type {
  TimelineRow,
  TimelineAction,
  TimelineEffect,
  TimelineState,
} from '@cyca/react-timeline-editor'
import '@cyca/react-timeline-editor/dist/react-timeline-editor.css'
import './timeline-editor.css'
import { useTimelineStore } from '@/stores/timeline-store'
import { useDocumentStore } from '@/stores/document-store'
import {
  toTimelineRows,
  applyActionMove,
  applyActionResize,
  validateActionMove,
  validateActionResize,
} from '@/animation/timeline-adapter'
import {
  secToMs,
  EFFECT_ANIMATION_PHASE,
  EFFECT_VIDEO_CLIP,
  type ActionMetadataMap,
  type TimelineStores,
  type VideoNodeProjection,
} from '@/animation/timeline-adapter-types'
import type { PenNode, VideoNode } from '@/types/pen'
import { useCanvasStore } from '@/stores/canvas-store'
import { isCursorUpdateRecent } from '@/animation/canvas-bridge'
import { setTimelineRef } from '@/animation/playback-loop'

// ---------------------------------------------------------------------------
// Effects registry (no engine callbacks — we bypass the library's engine)
// ---------------------------------------------------------------------------

const effects: Record<string, TimelineEffect> = {
  [EFFECT_ANIMATION_PHASE]: { id: EFFECT_ANIMATION_PHASE, name: 'Animation Phase' },
  [EFFECT_VIDEO_CLIP]: { id: EFFECT_VIDEO_CLIP, name: 'Video Clip' },
}

// ---------------------------------------------------------------------------
// Store bridge (creates TimelineStores interface from real Zustand stores)
// ---------------------------------------------------------------------------

function getStores(): TimelineStores {
  return {
    getTimelineState: () => {
      const s = useTimelineStore.getState()
      return { tracks: s.tracks, duration: s.duration, videoClipIds: s.videoClipIds }
    },
    updateKeyframe: useTimelineStore.getState().updateKeyframe,
    getDocumentState: () => ({
      getNodeById: useDocumentStore.getState().getNodeById,
    }),
    updateNode: useDocumentStore.getState().updateNode,
  }
}

// ---------------------------------------------------------------------------
// Video node projection helper
// ---------------------------------------------------------------------------

function getVideoNodeProjections(videoClipIds: string[]): VideoNodeProjection[] {
  const { getNodeById } = useDocumentStore.getState()
  const projections: VideoNodeProjection[] = []
  for (const id of videoClipIds) {
    const node = getNodeById(id) as (PenNode & VideoNode) | undefined
    if (node?.type === 'video') {
      projections.push({
        id: node.id,
        name: node.name,
        inPoint: node.inPoint,
        outPoint: node.outPoint,
        timelineOffset: node.timelineOffset,
        videoDuration: node.videoDuration,
      })
    }
  }
  return projections
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function TimelineEditor() {
  const timelineRef = useRef<TimelineState>(null)

  // Subscribe to structural data only (not currentTime)
  const tracks = useTimelineStore((s) => s.tracks)
  const duration_ms = useTimelineStore((s) => s.duration)
  const videoClipIds = useTimelineStore((s) => s.videoClipIds)

  // Mutable ref layer for drag operations
  const isDragging = useRef(false)
  const frozenRows = useRef<TimelineRow[] | null>(null)
  const metadataRef = useRef<ActionMetadataMap>(new Map())

  // Compute projection from stores
  const videoNodes = useMemo(
    () => getVideoNodeProjections(videoClipIds),
    [videoClipIds],
  )

  const { rows: computedRows, metadata } = useMemo(
    () => toTimelineRows(tracks, videoNodes, duration_ms),
    [tracks, videoNodes, duration_ms],
  )

  // Keep metadata ref in sync
  metadataRef.current = metadata

  // Use frozen rows during drag, computed rows otherwise
  const displayRows = isDragging.current ? (frozenRows.current ?? computedRows) : computedRows

  // Wire timeline ref to playback loop for cursor sync
  useEffect(() => {
    if (timelineRef.current) {
      setTimelineRef(timelineRef.current)
    }
    return () => {
      setTimelineRef(null)
      isDragging.current = false
      frozenRows.current = null
    }
  }, [])

  // --- Drag callbacks ---

  const onActionMoveStart = useCallback(() => {
    isDragging.current = true
    frozenRows.current = computedRows
  }, [computedRows])

  const onActionMoving = useCallback(
    ({ action, start, end }: { action: TimelineAction; row: TimelineRow; start: number; end: number }) => {
      return validateActionMove(action.id, start, end, metadataRef.current, getStores())
    },
    [],
  )

  const onActionMoveEnd = useCallback(
    ({ action, start, end }: { action: TimelineAction; row: TimelineRow; start: number; end: number }) => {
      applyActionMove(action.id, start, end, metadataRef.current, getStores())
      isDragging.current = false
      frozenRows.current = null
    },
    [],
  )

  const onActionResizeStart = useCallback(() => {
    isDragging.current = true
    frozenRows.current = computedRows
  }, [computedRows])

  const onActionResizing = useCallback(
    ({ action, start, end }: { action: TimelineAction; row: TimelineRow; start: number; end: number; dir: 'right' | 'left' }) => {
      return validateActionResize(action.id, start, end, metadataRef.current, getStores())
    },
    [],
  )

  const onActionResizeEnd = useCallback(
    ({ action, start, end, dir }: { action: TimelineAction; row: TimelineRow; start: number; end: number; dir: 'right' | 'left' }) => {
      applyActionResize(action.id, start, end, dir, metadataRef.current, getStores())
      isDragging.current = false
      frozenRows.current = null
    },
    [],
  )

  // --- Cursor callbacks ---

  const onCursorDrag = useCallback((time_s: number) => {
    // Skip if playback engine just set the cursor (prevents feedback loop)
    if (isCursorUpdateRecent()) return
    useTimelineStore.getState().setCurrentTime(secToMs(time_s))
  }, [])

  const onClickTimeArea = useCallback((time_s: number) => {
    useTimelineStore.getState().setCurrentTime(secToMs(time_s))
    return undefined
  }, [])

  // --- Click callbacks ---

  const onClickRow = useCallback(
    (_e: React.MouseEvent, { row }: { row: TimelineRow; time: number }) => {
      useCanvasStore.getState().setSelection([row.id], row.id)
    },
    [],
  )

  return (
    <div className="timeline-editor-wrapper">
      <Timeline
        ref={timelineRef}
        editorData={displayRows}
        effects={effects}
        autoReRender={false}
        scale={1}
        scaleWidth={160}
        startLeft={20}
        rowHeight={32}
        gridSnap={false}
        dragLine
        style={{ width: '100%', height: '100%' }}
        onActionMoveStart={onActionMoveStart}
        onActionMoving={onActionMoving}
        onActionMoveEnd={onActionMoveEnd}
        onActionResizeStart={onActionResizeStart}
        onActionResizing={onActionResizing}
        onActionResizeEnd={onActionResizeEnd}
        onCursorDrag={onCursorDrag}
        onClickTimeArea={onClickTimeArea}
        onClickRow={onClickRow}
      />
    </div>
  )
}
