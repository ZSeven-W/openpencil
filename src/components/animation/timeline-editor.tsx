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
  buildTimelineRowsFromNodes,
  applyActionMove,
  applyActionResize,
  validateActionMove,
  validateActionResize,
} from '@/animation/timeline-adapter'
import {
  msToSec,
  secToMs,
  EFFECT_VIDEO_CLIP,
  EFFECT_ANIMATION_CLIP,
  type ActionMetadataMap,
} from '@/animation/timeline-adapter-types'
import type { PenNode } from '@/types/pen'
import { useCanvasStore } from '@/stores/canvas-store'
import { getActivePageChildren } from '@/stores/document-store'
import { seekToV2, usePlaybackTime } from '@/animation/use-playback-controller'
import { seekVideoClipsV2 } from '@/animation/video-sync'
import { buildAnimationIndex } from '@/animation/animation-index'
import { withTimelineUndoBatch } from '@/animation/timeline-undo'
import VideoClipRenderer from './video-clip-renderer'
import AnimationClipRenderer from './animation-clip-renderer'
import TrackHeaders from './track-headers'
import type { OnScrollParams } from '@cyca/react-timeline-editor'

// ---------------------------------------------------------------------------
// Effects registry (no engine callbacks — we bypass the library's engine)
// ---------------------------------------------------------------------------

const effects: Record<string, TimelineEffect> = {
  [EFFECT_VIDEO_CLIP]: { id: EFFECT_VIDEO_CLIP, name: 'Video Clip' },
  [EFFECT_ANIMATION_CLIP]: { id: EFFECT_ANIMATION_CLIP, name: 'Animation Clip' },
}

// ---------------------------------------------------------------------------
// Store bridge for timeline adapter
// ---------------------------------------------------------------------------

function getStores(): import('@/animation/timeline-adapter-types').TimelineStores {
  return {
    getDocumentState: () => ({
      getNodeById: useDocumentStore.getState().getNodeById,
    }),
    updateNode: (id, partial) => useDocumentStore.getState().updateNode(id, partial),
    getDuration: () => useTimelineStore.getState().duration,
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function TimelineEditor() {
  const timelineRef = useRef<TimelineState>(null)

  const duration_ms = useTimelineStore((s) => s.duration)

  // Scroll sync for track headers
  const [scrollTop, setScrollTop] = useState(0)

  // Mutable ref layer for drag operations
  const isDragging = useRef(false)
  const frozenRows = useRef<TimelineRow[] | null>(null)
  const metadataRef = useRef<ActionMetadataMap>(new Map())
  const frozenMetadataRef = useRef<ActionMetadataMap | null>(null)
  const computedRowsRef = useRef<TimelineRow[]>([])

  // v2: Subscribe to nodes with clips for clip-based timeline rows
  const activePageId = useCanvasStore((s) => s.activePageId)
  const pageChildren = useDocumentStore((s) => getActivePageChildren(s.document, activePageId))

  // Recompute when node clips change
  const clipVersion = useMemo(() => {
    let hash = 0
    function walk(nodes: PenNode[]) {
      for (const n of nodes) {
        if (n.clips && n.clips.length > 0) {
          hash += n.clips.length
          for (const c of n.clips) hash = hash * 31 + c.startTime + c.duration * 7
        }
        if ('children' in n && n.children) walk(n.children as PenNode[])
      }
    }
    walk(pageChildren)
    return hash
  }, [pageChildren])

  const { rows: computedRows, metadata } = useMemo(() => {
    return buildTimelineRowsFromNodes(pageChildren)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [duration_ms, clipVersion])

  // Keep refs in sync (read from callbacks, not during render)
  metadataRef.current = metadata
  computedRowsRef.current = computedRows

  // Use frozen rows during drag, computed rows otherwise
  const displayRows = isDragging.current ? (frozenRows.current ?? computedRows) : computedRows

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      isDragging.current = false
      frozenRows.current = null
    }
  }, [])

  // --- Playhead sync ---
  const playbackTime = usePlaybackTime()

  useEffect(() => {
    if (timelineRef.current) {
      timelineRef.current.time = msToSec(playbackTime)
    }
  }, [playbackTime])

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
    const timeMs = secToMs(time_s)
    seekToV2(timeMs)
    // Also seek video clips for scrub preview
    const index = buildAnimationIndex(getActivePageChildren(
      useDocumentStore.getState().document,
      useCanvasStore.getState().activePageId,
    ))
    const canvas = useCanvasStore.getState().fabricCanvas
    if (canvas) seekVideoClipsV2(canvas, timeMs, index)
  }, [])

  const onClickTimeArea = useCallback((time_s: number) => {
    const timeMs = secToMs(time_s)
    seekToV2(timeMs)
    return undefined
  }, [])

  // --- Click callbacks ---

  const onClickRow = useCallback(
    (_e: React.MouseEvent, { row }: { row: TimelineRow; time: number }) => {
      const nodeId = row.id.replace(/^v2::/, '')
      useCanvasStore.getState().setSelection([nodeId], nodeId)
    },
    [],
  )

  const onClickAction = useCallback(
    (_e: React.MouseEvent, { row }: { row: TimelineRow; action: TimelineAction }) => {
      const nodeId = row.id.replace(/^v2::/, '')
      useCanvasStore.getState().setSelection([nodeId], nodeId)
    },
    [],
  )

  // --- Custom rendering ---

  const getActionRender = useCallback(
    (action: TimelineAction, _row: TimelineRow) => {
      const meta = metadataRef.current.get(action.id)
      if (!meta) return null

      if (meta.type === 'clip') {
        if (meta.clipKind === 'video') {
          return (
            <VideoClipRenderer
              name={meta.nodeId}
              duration_s={action.end - action.start}
            />
          )
        }
        return (
          <AnimationClipRenderer
            clipId={meta.clipId}
          />
        )
      }

      // Legacy metadata support
      if (meta.type === 'video-clip') {
        return (
          <VideoClipRenderer
            name={meta.nodeId}
            duration_s={action.end - action.start}
          />
        )
      }

      if (meta.type === 'animation-clip') {
        return (
          <AnimationClipRenderer
            clipId={meta.clipId}
          />
        )
      }

      return null
    },
    [],
  )

  // --- Scroll sync ---

  const onScroll = useCallback((params: OnScrollParams) => {
    setScrollTop(params.scrollTop)
  }, [])

  // --- Selection sync (canvas ↔ timeline) ---
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds)

  // Mark rows as selected based on canvas selection
  const selectedDisplayRows = useMemo(() => {
    const selectedSet = new Set(selectedIds)
    return displayRows.map((row) => {
      const nodeId = row.id.replace(/^v2::/, '')
      return selectedSet.has(nodeId) ? { ...row, selected: true } : row
    })
  }, [displayRows, selectedIds])

  // Row IDs for track headers
  const rowIds = useMemo(() => displayRows.map((r) => r.id), [displayRows])

  return (
    <div className="timeline-editor-wrapper" style={{ display: 'flex' }}>
      <TrackHeaders rowIds={rowIds} rowHeight={32} scrollTop={scrollTop} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <Timeline
          ref={timelineRef}
          editorData={selectedDisplayRows}
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
          onClickAction={onClickAction}
        />
      </div>
    </div>
  )
}
