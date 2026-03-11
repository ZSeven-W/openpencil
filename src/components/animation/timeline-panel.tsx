import { Separator } from '@/components/ui/separator'
import PlaybackControls from './playback-controls'
import ScrubBar from './scrub-bar'
import TrackList from './track-list'
import VideoClipTrack from './video-clip-track'

export default function TimelinePanel() {
  return (
    <div className="border-t border-border bg-card">
      {/* Top row: playback controls + scrub bar */}
      <div className="flex items-center gap-3 px-3 py-1.5">
        <PlaybackControls />
        <div className="flex-1 min-w-0">
          <ScrubBar />
        </div>
      </div>

      <Separator />

      {/* Video clips */}
      <VideoClipTrack />

      {/* Animation tracks */}
      <div style={{ maxHeight: '200px' }} className="overflow-y-auto">
        <TrackList />
      </div>
    </div>
  )
}
