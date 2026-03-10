import { useRef, useCallback } from 'react'
import { useTimelineStore } from '@/stores/timeline-store'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import type { AnimationTrack, Keyframe, KeyframePhase } from '@/types/animation'

const phaseColors: Record<KeyframePhase, { bg: string; border: string }> = {
  in: { bg: 'bg-emerald-500', border: 'border-emerald-400' },
  while: { bg: 'bg-blue-500', border: 'border-blue-400' },
  out: { bg: 'bg-red-500', border: 'border-red-400' },
}

function KeyframeDiamond({
  keyframe,
  nodeId,
  duration,
  startDelay,
  isSelected,
}: {
  keyframe: Keyframe
  nodeId: string
  duration: number
  startDelay: number
  isSelected: boolean
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const updateKeyframe = useTimelineStore((s) => s.updateKeyframe)

  const pos = ((startDelay + keyframe.time) / duration) * 100

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (!isSelected) return
      e.stopPropagation()
      e.preventDefault()

      const container = containerRef.current?.parentElement
      if (!container) return

      const onMouseMove = (ev: MouseEvent) => {
        const rect = container.getBoundingClientRect()
        const ratio = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width))
        const newTime = Math.max(0, ratio * duration - startDelay)
        updateKeyframe(nodeId, keyframe.id, { time: Math.round(newTime) })
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
    [isSelected, duration, startDelay, nodeId, keyframe.id, updateKeyframe],
  )

  const colors = keyframe.phase ? phaseColors[keyframe.phase] : null

  return (
    <div
      ref={containerRef}
      className={`absolute top-1/2 -translate-y-1/2 w-2.5 h-2.5 rotate-45 -ml-[5px] border ${
        isSelected
          ? `${colors?.bg ?? 'bg-primary'} ${colors?.border ?? 'border-primary'} cursor-ew-resize hover:scale-125 transition-transform`
          : 'bg-foreground border-foreground/80'
      }`}
      style={{ left: `${pos}%` }}
      onMouseDown={handleMouseDown}
    />
  )
}

function PhaseBar({
  track,
  duration,
  isSelected,
}: {
  track: AnimationTrack
  duration: number
  isSelected: boolean
}) {
  const toPercent = (ms: number) => (ms / duration) * 100

  const { phases, startDelay } = track
  const inStart = toPercent(startDelay + phases.in.start)
  const inWidth = toPercent(phases.in.duration)
  const whileStart = toPercent(startDelay + phases.while.start)
  const whileWidth = toPercent(phases.while.duration)
  const outStart = toPercent(startDelay + phases.out.start)
  const outWidth = toPercent(phases.out.duration)

  return (
    <div className={`relative h-6 w-full ${isSelected ? 'opacity-100' : 'opacity-60'}`}>
      {/* In phase */}
      <div
        className="absolute top-0.5 bottom-0.5 bg-emerald-500/30 border border-emerald-500/50 rounded-l"
        style={{ left: `${inStart}%`, width: `${inWidth}%` }}
      />
      {/* While phase */}
      <div
        className="absolute top-0.5 bottom-0.5 bg-blue-500/20 border-y border-blue-500/30"
        style={{ left: `${whileStart}%`, width: `${whileWidth}%` }}
      />
      {/* Out phase */}
      <div
        className="absolute top-0.5 bottom-0.5 bg-red-500/30 border border-red-500/50 rounded-r"
        style={{ left: `${outStart}%`, width: `${outWidth}%` }}
      />
      {/* Keyframe diamonds */}
      {track.keyframes.map((kf) => (
        <KeyframeDiamond
          key={kf.id}
          keyframe={kf}
          nodeId={track.nodeId}
          duration={duration}
          startDelay={startDelay}
          isSelected={isSelected}
        />
      ))}
    </div>
  )
}

export default function TrackList() {
  const tracks = useTimelineStore((s) => s.tracks)
  const duration = useTimelineStore((s) => s.duration)
  const getNodeById = useDocumentStore((s) => s.getNodeById)
  const selectedId = useCanvasStore((s) => s.selection.activeId)

  const trackEntries = Object.values(tracks)

  if (trackEntries.length === 0) {
    return (
      <div className="px-3 py-4 text-xs text-muted-foreground text-center">
        Select a layer and apply an animation preset
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-0.5 px-2 py-1">
      {trackEntries.map((track) => {
        const node = getNodeById(track.nodeId)
        const name = node?.name || node?.type || track.nodeId.slice(0, 6)
        const isSelected = track.nodeId === selectedId
        return (
          <div key={track.nodeId} className="flex items-center gap-2 h-7">
            <span
              className={`text-[11px] w-20 truncate shrink-0 ${
                isSelected ? 'text-foreground font-medium' : 'text-muted-foreground'
              }`}
            >
              {name}
            </span>
            <div className="flex-1 min-w-0">
              <PhaseBar track={track} duration={duration} isSelected={isSelected} />
            </div>
          </div>
        )
      })}
    </div>
  )
}
