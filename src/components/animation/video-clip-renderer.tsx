/**
 * Custom action renderer for video clip actions.
 * Shows a muted violet bar with film icon, clip name, and duration timecode.
 */

import { Film } from 'lucide-react'

interface VideoClipRendererProps {
  name: string
  duration_s: number
}

function formatTimecode(seconds: number): string {
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins}:${secs.toFixed(1).padStart(4, '0')}`
}

export default function VideoClipRenderer({
  name,
  duration_s,
}: VideoClipRendererProps) {
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        backgroundColor: 'oklch(0.45 0.12 300 / 0.25)',
        border: '1px solid oklch(0.55 0.15 300 / 0.40)',
        borderRadius: 3,
        display: 'flex',
        alignItems: 'center',
        gap: 4,
        paddingInline: 6,
        overflow: 'hidden',
      }}
    >
      <Film size={10} style={{ flexShrink: 0, color: 'oklch(0.70 0.12 300)' }} />
      <span
        style={{
          fontSize: 10,
          color: 'oklch(0.75 0.08 300)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          flex: 1,
          minWidth: 0,
        }}
      >
        {name}
      </span>
      <span
        style={{
          fontSize: 8,
          fontFamily: 'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, monospace',
          fontVariantNumeric: 'tabular-nums',
          color: 'oklch(0.60 0.08 300)',
          flexShrink: 0,
        }}
      >
        {formatTimecode(duration_s)}
      </span>
    </div>
  )
}
