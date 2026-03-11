import { Separator } from '@/components/ui/separator'
import PlaybackControls from './playback-controls'
import TimelineEditor from './timeline-editor'

export default function TimelinePanel() {
  return (
    <div className="border-t border-border bg-card" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Top row: playback controls */}
      <div className="flex items-center gap-3 px-3 py-1.5">
        <PlaybackControls />
      </div>

      <Separator />

      {/* Timeline editor (unified animation + video tracks) */}
      <div style={{ flex: 1, minHeight: 0 }}>
        <TimelineEditor />
      </div>
    </div>
  )
}
