import { useTimelineStore } from '@/stores/timeline-store'
import { useDocumentStore } from '@/stores/document-store'
import type { AnimationTrack } from '@/types/animation'

function PhaseBar({
  track,
  duration,
}: {
  track: AnimationTrack
  duration: number
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
    <div className="relative h-5 w-full">
      {/* In phase */}
      <div
        className="absolute top-0 h-full bg-green-500/30 border border-green-500/50 rounded-l"
        style={{ left: `${inStart}%`, width: `${inWidth}%` }}
      />
      {/* While phase */}
      <div
        className="absolute top-0 h-full bg-blue-500/20 border-y border-blue-500/30"
        style={{ left: `${whileStart}%`, width: `${whileWidth}%` }}
      />
      {/* Out phase */}
      <div
        className="absolute top-0 h-full bg-red-500/30 border border-red-500/50 rounded-r"
        style={{ left: `${outStart}%`, width: `${outWidth}%` }}
      />
      {/* Keyframe diamonds */}
      {track.keyframes.map((kf) => {
        const pos = toPercent(startDelay + kf.time)
        return (
          <div
            key={kf.id}
            className="absolute top-1/2 -translate-y-1/2 w-2 h-2 bg-foreground rotate-45 -ml-1"
            style={{ left: `${pos}%` }}
          />
        )
      })}
    </div>
  )
}

export default function TrackList() {
  const tracks = useTimelineStore((s) => s.tracks)
  const duration = useTimelineStore((s) => s.duration)
  const getNodeById = useDocumentStore((s) => s.getNodeById)

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
        return (
          <div key={track.nodeId} className="flex items-center gap-2 h-7">
            <span className="text-xs text-muted-foreground w-20 truncate shrink-0">
              {name}
            </span>
            <div className="flex-1 min-w-0">
              <PhaseBar track={track} duration={duration} />
            </div>
          </div>
        )
      })}
    </div>
  )
}
