import { Play, Pause, Square, Repeat } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { useTimelineStore } from '@/stores/timeline-store'
import {
  playV2,
  pauseV2,
  stopV2,
  usePlaybackTime,
  usePlaybackPlaying,
} from '@/animation/use-playback-controller'

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  const centiseconds = Math.floor((ms % 1000) / 10)
  return `${minutes}:${String(seconds).padStart(2, '0')}.${String(centiseconds).padStart(2, '0')}`
}

export default function PlaybackControls() {
  const currentTime = usePlaybackTime()
  const duration = useTimelineStore((s) => s.duration)
  const isPlaying = usePlaybackPlaying()
  const loopEnabled = useTimelineStore((s) => s.loopEnabled)
  const toggleLoop = useTimelineStore((s) => s.toggleLoop)

  const handlePlayPause = () => {
    if (isPlaying) {
      pauseV2()
    } else {
      playV2()
    }
  }

  const handleStop = () => {
    stopV2()
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
            {isPlaying ? (
              <Pause className="h-3.5 w-3.5" />
            ) : (
              <Play className="h-3.5 w-3.5" />
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {isPlaying ? 'Pause' : 'Play'}
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
