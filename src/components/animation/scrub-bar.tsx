import { useRef, useCallback } from 'react'
import { useTimelineStore } from '@/stores/timeline-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { seekTo, isPlaying, pause } from '@/animation/playback-loop'

export default function ScrubBar() {
  const currentTime = useTimelineStore((s) => s.currentTime)
  const duration = useTimelineStore((s) => s.duration)
  const canvas = useCanvasStore((s) => s.fabricCanvas)
  const barRef = useRef<HTMLDivElement>(null)

  const timeToPosition = (time: number): number => {
    return duration > 0 ? (time / duration) * 100 : 0
  }

  const positionToTime = useCallback(
    (clientX: number): number => {
      const bar = barRef.current
      if (!bar) return 0
      const rect = bar.getBoundingClientRect()
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width))
      return ratio * duration
    },
    [duration],
  )

  const handleScrub = useCallback(
    (clientX: number) => {
      if (!canvas) return
      const time = positionToTime(clientX)
      seekTo(canvas, time)
    },
    [canvas, positionToTime],
  )

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (!canvas) return

      // Pause if playing
      if (isPlaying()) {
        pause(canvas)
      }

      useTimelineStore.getState().setPlaybackMode('scrubbing')
      handleScrub(e.clientX)

      const onMouseMove = (ev: MouseEvent) => handleScrub(ev.clientX)
      const onMouseUp = () => {
        useTimelineStore.getState().setPlaybackMode('idle')
        window.removeEventListener('mousemove', onMouseMove)
        window.removeEventListener('mouseup', onMouseUp)
      }

      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
    },
    [canvas, handleScrub],
  )

  return (
    <div
      ref={barRef}
      className="relative h-6 cursor-pointer group"
      onMouseDown={handleMouseDown}
    >
      {/* Track background */}
      <div className="absolute inset-x-0 top-1/2 -translate-y-1/2 h-1 bg-border rounded-full" />

      {/* Progress fill */}
      <div
        className="absolute top-1/2 -translate-y-1/2 h-1 bg-primary rounded-full"
        style={{ width: `${timeToPosition(currentTime)}%` }}
      />

      {/* Playhead */}
      <div
        className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full -ml-1.5 shadow-sm transition-transform group-hover:scale-125"
        style={{ left: `${timeToPosition(currentTime)}%` }}
      />
    </div>
  )
}
