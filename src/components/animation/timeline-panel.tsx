import { Separator } from '@/components/ui/separator'
import PlaybackControls from './playback-controls'
import ScrubBar from './scrub-bar'
import TrackList from './track-list'
import PresetPanel from './preset-panel'

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

      {/* Content: tracks left, presets right */}
      <div className="flex" style={{ maxHeight: '200px' }}>
        {/* Track list */}
        <div className="flex-1 min-w-0 overflow-y-auto">
          <TrackList />
        </div>

        {/* Preset panel */}
        <div className="w-64 border-l border-border shrink-0 overflow-y-auto">
          <PresetPanel />
        </div>
      </div>
    </div>
  )
}
