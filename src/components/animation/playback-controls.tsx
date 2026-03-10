import { Play, Pause, Square, Repeat } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { useTimelineStore } from '@/stores/timeline-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { play, pause, stop, isPlaying } from '@/animation/playback-loop'

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  const centiseconds = Math.floor((ms % 1000) / 10)
  return `${minutes}:${String(seconds).padStart(2, '0')}.${String(centiseconds).padStart(2, '0')}`
}

export default function PlaybackControls() {
  const currentTime = useTimelineStore((s) => s.currentTime)
  const duration = useTimelineStore((s) => s.duration)
  const playbackMode = useTimelineStore((s) => s.playbackMode)
  const loopEnabled = useTimelineStore((s) => s.loopEnabled)
  const toggleLoop = useTimelineStore((s) => s.toggleLoop)
  const canvas = useCanvasStore((s) => s.fabricCanvas)

  const handlePlayPause = () => {
    if (!canvas) return
    if (isPlaying()) {
      pause(canvas)
    } else {
      play(canvas)
    }
  }

  const handleStop = () => {
    if (!canvas) return
    stop(canvas)
  }

  return (
    <div className="flex items-center gap-1.5">
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handlePlayPause}
          >
            {playbackMode === 'playing' ? (
              <Pause className="h-3.5 w-3.5" />
            ) : (
              <Play className="h-3.5 w-3.5" />
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {playbackMode === 'playing' ? 'Pause' : 'Play'}
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleStop}
          >
            <Square className="h-3 w-3" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Stop</TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className={`h-7 w-7 ${loopEnabled ? 'bg-primary text-primary-foreground' : ''}`}
            onClick={toggleLoop}
          >
            <Repeat className="h-3.5 w-3.5" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Loop</TooltipContent>
      </Tooltip>

      <span className="text-xs text-muted-foreground font-mono ml-2">
        {formatTime(currentTime)} / {formatTime(duration)}
      </span>
    </div>
  )
}
