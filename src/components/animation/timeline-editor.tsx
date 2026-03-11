import { useRef, useMemo, useCallback, useEffect, useState } from 'react'
import { Timeline } from '@cyca/react-timeline-editor'
import type {
  TimelineRow,
  TimelineAction,
  TimelineEffect,
  TimelineState,
} from '@cyca/react-timeline-editor'
// @ts-expect-error — CSS not in package exports; resolved via Vite alias
import '@cyca/react-timeline-editor/css'
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
import { consumeCursorGuard } from '@/animation/canvas-bridge'
import { setTimelineRef } from '@/animation/playback-loop'
import { withTimelineUndoBatch } from '@/animation/timeline-undo'
import PhaseActionRenderer from './phase-action-renderer'
import VideoClipRenderer from './video-clip-renderer'
import TrackHeaders from './track-headers'
import type { OnScrollParams } from '@cyca/react-timeline-editor'

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
    updateKeyframe: (...args) => useTimelineStore.getState().updateKeyframe(...args),
    getDocumentState: () => ({
      getNodeById: useDocumentStore.getState().getNodeById,
    }),
    updateNode: (...args) => useDocumentStore.getState().updateNode(...args),
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

  // Scroll sync for track headers
  const [scrollTop, setScrollTop] = useState(0)

  // Mutable ref layer for drag operations
  const isDragging = useRef(false)
  const frozenRows = useRef<TimelineRow[] | null>(null)
  const metadataRef = useRef<ActionMetadataMap>(new Map())
  const frozenMetadataRef = useRef<ActionMetadataMap | null>(null)
  const computedRowsRef = useRef<TimelineRow[]>([])

  // Subscribe to video node property changes (inPoint, outPoint, timelineOffset)
  // so the timeline updates when video clips are trimmed or moved via other UI
  const videoNodeVersion = useDocumentStore((s) => {
    let hash = 0
    for (const id of videoClipIds) {
      const node = s.getNodeById(id)
      if (node && node.type === 'video') {
        const v = node as PenNode & VideoNode
        hash = hash * 31 + (v.inPoint ?? 0) + (v.outPoint ?? 0) * 7 + (v.timelineOffset ?? 0) * 13
      }
    }
    return hash
  })

  // Compute projection from stores
  const videoNodes = useMemo(
    () => getVideoNodeProjections(videoClipIds),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [videoClipIds, videoNodeVersion],
  )

  const { rows: computedRows, metadata } = useMemo(
    () => toTimelineRows(tracks, videoNodes, duration_ms),
    [tracks, videoNodes, duration_ms],
  )

  // Keep refs in sync (read from callbacks, not during render)
  metadataRef.current = metadata
  computedRowsRef.current = computedRows

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

  const onDragStart = useCallback(() => {
    isDragging.current = true
    frozenRows.current = computedRowsRef.current
    frozenMetadataRef.current = metadataRef.current
  }, [])

  const getDragMetadata = () => frozenMetadataRef.current ?? metadataRef.current

  const onActionMoving = useCallback(
    ({ action, start, end }: { action: TimelineAction; row: TimelineRow; start: number; end: number }) => {
      return validateActionMove(action.id, start, end, getDragMetadata(), getStores())
    },
    [],
  )

  const onActionMoveEnd = useCallback(
    ({ action, start, end }: { action: TimelineAction; row: TimelineRow; start: number; end: number }) => {
      withTimelineUndoBatch(() => {
        applyActionMove(action.id, start, end, getDragMetadata(), getStores())
      })
      isDragging.current = false
      frozenRows.current = null
      frozenMetadataRef.current = null
    },
    [],
  )

  const onActionResizing = useCallback(
    ({ action, start, end }: { action: TimelineAction; row: TimelineRow; start: number; end: number; dir: 'right' | 'left' }) => {
      return validateActionResize(action.id, start, end, getDragMetadata(), getStores())
    },
    [],
  )

  const onActionResizeEnd = useCallback(
    ({ action, start, end, dir }: { action: TimelineAction; row: TimelineRow; start: number; end: number; dir: 'right' | 'left' }) => {
      withTimelineUndoBatch(() => {
        applyActionResize(action.id, start, end, dir, getDragMetadata(), getStores())
      })
      isDragging.current = false
      frozenRows.current = null
      frozenMetadataRef.current = null
    },
    [],
  )

  // --- Cursor callbacks ---

  const onCursorDrag = useCallback((time_s: number) => {
    // Skip if playback engine just set the cursor (prevents feedback loop)
    if (consumeCursorGuard()) return
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

  // --- Custom rendering ---

  const getActionRender = useCallback(
    (action: TimelineAction, _row: TimelineRow) => {
      const meta = metadataRef.current.get(action.id)
      if (!meta) return null

      if (meta.type === 'animation-phase') {
        return (
          <PhaseActionRenderer
            nodeId={meta.nodeId}
            phase={meta.phase}
            actionStart_s={action.start}
            actionEnd_s={action.end}
          />
        )
      }

      if (meta.type === 'video-clip') {
        const node = videoNodes.find((n) => n.id === meta.nodeId)
        return (
          <VideoClipRenderer
            name={node?.name ?? 'Video'}
            duration_s={action.end - action.start}
          />
        )
      }

      return null
    },
    [videoNodes],
  )

  // --- Scroll sync ---

  const onScroll = useCallback((params: OnScrollParams) => {
    setScrollTop(params.scrollTop)
  }, [])

  // Row IDs for track headers
  const rowIds = useMemo(() => displayRows.map((r) => r.id), [displayRows])

  return (
    <div className="timeline-editor-wrapper" style={{ display: 'flex' }}>
      <TrackHeaders rowIds={rowIds} rowHeight={32} scrollTop={scrollTop} />
      <div style={{ flex: 1, minWidth: 0 }}>
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
          getActionRender={getActionRender}
          onScroll={onScroll}
          onActionMoveStart={onDragStart}
          onActionMoving={onActionMoving}
          onActionMoveEnd={onActionMoveEnd}
          onActionResizeStart={onDragStart}
          onActionResizing={onActionResizing}
          onActionResizeEnd={onActionResizeEnd}
          onCursorDrag={onCursorDrag}
          onClickTimeArea={onClickTimeArea}
          onClickRow={onClickRow}
        />
      </div>
    </div>
  )
}
