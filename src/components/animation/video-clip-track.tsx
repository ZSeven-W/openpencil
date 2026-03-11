import { useRef, useCallback } from 'react'
import { Film, Scissors } from 'lucide-react'
import { useTimelineStore } from '@/stores/timeline-store'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { seekTo } from '@/animation/playback-loop'
import type { VideoNode } from '@/types/pen'

function formatTimecode(ms: number): string {
  const totalSec = Math.floor(ms / 1000)
  const min = Math.floor(totalSec / 60)
  const sec = totalSec % 60
  return `${min}:${String(sec).padStart(2, '0')}`
}

function VideoClipBar({
  node,
  duration,
  isSelected,
}: {
  node: VideoNode
  duration: number
  isSelected: boolean
}) {
  const barRef = useRef<HTMLDivElement>(null)
  const updateNode = useDocumentStore((s) => s.updateNode)
  const canvas = useCanvasStore((s) => s.fabricCanvas)

  const inPoint = node.inPoint ?? 0
  const outPoint = node.outPoint ?? (node.videoDuration ?? duration)
  const timelineOffset = node.timelineOffset ?? 0
  const clipDuration = outPoint - inPoint
  const videoDuration = node.videoDuration ?? duration

  // Position and width on the timeline
  const leftPercent = (timelineOffset / duration) * 100
  const widthPercent = (clipDuration / duration) * 100

  // Trim left handle: adjusts inPoint
  const handleTrimLeft = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation()
      e.preventDefault()
      const bar = barRef.current?.parentElement
      if (!bar) return

      const onMouseMove = (ev: MouseEvent) => {
        const rect = bar.getBoundingClientRect()
        const ratio = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width))
        const timeAtCursor = ratio * duration - timelineOffset
        const newInPoint = Math.max(0, Math.min(outPoint - 100, Math.round(timeAtCursor)))
        updateNode(node.id, { inPoint: newInPoint } as Partial<VideoNode>)
        // Scrub to the trim point
        if (canvas) seekTo(canvas, timelineOffset + newInPoint)
      }

      const onMouseUp = () => {
        document.body.style.cursor = ''
        document.body.style.userSelect = ''
        window.removeEventListener('mousemove', onMouseMove)
        window.removeEventListener('mouseup', onMouseUp)
      }

      document.body.style.cursor = 'ew-resize'
      document.body.style.userSelect = 'none'
      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
    },
    [inPoint, outPoint, timelineOffset, duration, node.id, updateNode, canvas],
  )

  // Trim right handle: adjusts outPoint
  const handleTrimRight = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation()
      e.preventDefault()
      const bar = barRef.current?.parentElement
      if (!bar) return

      const onMouseMove = (ev: MouseEvent) => {
        const rect = bar.getBoundingClientRect()
        const ratio = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width))
        const timeAtCursor = ratio * duration - timelineOffset
        const newOutPoint = Math.max(inPoint + 100, Math.min(videoDuration, Math.round(timeAtCursor)))
        updateNode(node.id, { outPoint: newOutPoint } as Partial<VideoNode>)
        // Scrub to the trim point
        if (canvas) seekTo(canvas, timelineOffset + newOutPoint)
      }

      const onMouseUp = () => {
        document.body.style.cursor = ''
        document.body.style.userSelect = ''
        window.removeEventListener('mousemove', onMouseMove)
        window.removeEventListener('mouseup', onMouseUp)
      }

      document.body.style.cursor = 'ew-resize'
      document.body.style.userSelect = 'none'
      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
    },
    [inPoint, timelineOffset, duration, videoDuration, node.id, updateNode, canvas],
  )

  // Drag the whole clip to reposition on timeline
  const handleDragClip = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      const bar = barRef.current?.parentElement
      if (!bar) return

      const rect = bar.getBoundingClientRect()
      const startRatio = (e.clientX - rect.left) / rect.width
      const startOffset = timelineOffset

      const onMouseMove = (ev: MouseEvent) => {
        const currentRatio = (ev.clientX - rect.left) / rect.width
        const deltaMs = (currentRatio - startRatio) * duration
        const newOffset = Math.max(0, Math.round(startOffset + deltaMs))
        updateNode(node.id, { timelineOffset: newOffset } as Partial<VideoNode>)
      }

      const onMouseUp = () => {
        document.body.style.cursor = ''
        document.body.style.userSelect = ''
        window.removeEventListener('mousemove', onMouseMove)
        window.removeEventListener('mouseup', onMouseUp)
      }

      document.body.style.cursor = 'grabbing'
      document.body.style.userSelect = 'none'
      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
    },
    [timelineOffset, duration, node.id, updateNode],
  )

  return (
    <div
      ref={barRef}
      className={`absolute top-0.5 bottom-0.5 rounded flex items-center overflow-hidden cursor-grab active:cursor-grabbing ${
        isSelected
          ? 'bg-violet-500/40 border border-violet-400/70'
          : 'bg-violet-500/25 border border-violet-500/40'
      }`}
      style={{ left: `${leftPercent}%`, width: `${widthPercent}%`, minWidth: '20px' }}
      onMouseDown={handleDragClip}
    >
      {/* Left trim handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-violet-300/40 flex items-center justify-center z-10"
        onMouseDown={handleTrimLeft}
      >
        <div className="w-0.5 h-3 bg-violet-300/60 rounded-full" />
      </div>

      {/* Clip content */}
      <div className="flex-1 flex items-center gap-1 px-3 min-w-0 pointer-events-none select-none">
        <Film size={10} className="shrink-0 text-violet-300" />
        <span className="text-[9px] text-violet-200 truncate">{node.name ?? 'Video'}</span>
        <span className="text-[8px] text-violet-300/60 ml-auto shrink-0">
          {formatTimecode(clipDuration)}
        </span>
      </div>

      {/* Right trim handle */}
      <div
        className="absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-violet-300/40 flex items-center justify-center z-10"
        onMouseDown={handleTrimRight}
      >
        <div className="w-0.5 h-3 bg-violet-300/60 rounded-full" />
      </div>
    </div>
  )
}

export default function VideoClipTrack() {
  const videoClipIds = useTimelineStore((s) => s.videoClipIds)
  const duration = useTimelineStore((s) => s.duration)
  const getNodeById = useDocumentStore((s) => s.getNodeById)
  const selectedId = useCanvasStore((s) => s.selection.activeId)

  if (videoClipIds.length === 0) return null

  return (
    <div className="flex flex-col gap-0.5 px-2 py-1">
      {videoClipIds.map((nodeId) => {
        const node = getNodeById(nodeId)
        if (!node || node.type !== 'video') return null
        const videoNode = node as VideoNode
        const isSelected = nodeId === selectedId
        const name = videoNode.name ?? 'Video'

        return (
          <div key={nodeId} className="flex items-center gap-2 h-7">
            <div className="flex items-center gap-1 w-20 shrink-0">
              <Scissors size={10} className="text-violet-400 shrink-0" />
              <span
                className={`text-[11px] truncate ${
                  isSelected ? 'text-violet-300 font-medium' : 'text-muted-foreground'
                }`}
              >
                {name}
              </span>
            </div>
            <div className="flex-1 min-w-0 relative h-6">
              <VideoClipBar
                node={videoNode}
                duration={duration}
                isSelected={isSelected}
              />
            </div>
          </div>
        )
      })}
    </div>
  )
}
