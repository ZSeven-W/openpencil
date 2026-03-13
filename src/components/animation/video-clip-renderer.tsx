/**
 * Custom action renderer for video clip actions.
 * Shows a violet rounded bar with a film-strip thumbnail row underneath
 * and segment overlays (VideoIn | hold | VideoOut).
 * Colors sourced from --clip-video design tokens.
 */

import { Film } from 'lucide-react'
import { useDocumentStore } from '@/stores/document-store'

interface VideoClipRendererProps {
  name: string
  duration_s: number
  thumbnailUrl?: string
}

function formatTimecode(seconds: number): string {
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins}:${secs.toFixed(1).padStart(4, '0')}`
}

export default function VideoClipRenderer({
  name: nodeId,
  duration_s,
  thumbnailUrl,
}: VideoClipRendererProps) {
  // Resolve display name from document store (name prop is actually nodeId)
  const displayName = useDocumentStore((s) => {
    const node = s.getNodeById(nodeId)
    return node?.name ?? nodeId.slice(0, 8)
  })
  const showSegments = duration_s > 0.5

  return (
    <div className="group/vclip h-full w-full relative overflow-hidden rounded-xl bg-clip-video-bg border border-clip-video-border hover:border-clip-video hover:border-[1.5px] transition-colors">
      {/* Film strip background layer */}
      {thumbnailUrl && (
        <div className="absolute inset-0 flex gap-0.5 p-0.5 opacity-30">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="flex-1 rounded-[10px] bg-cover bg-center"
              style={{ backgroundImage: `url(${thumbnailUrl})` }}
            />
          ))}
        </div>
      )}

      {/* Segment overlay layer */}
      <div className="absolute inset-0 flex items-stretch">
        {showSegments ? (
          <>
            {/* VideoIn segment */}
            <div className="w-[60px] shrink-0 flex items-center justify-center border-r border-clip-video-border/50 bg-clip-video-bg/40 rounded-l-xl">
              <Film size={10} className="text-clip-video/60" />
            </div>

            {/* Hold segment */}
            <div className="flex-1 min-w-0 flex items-center justify-center gap-1">
              <Film size={10} className="shrink-0 text-clip-video" />
              <span className="text-[10px] text-clip-video truncate">
                {displayName}
              </span>
              <span className="text-[8px] font-mono tabular-nums text-clip-video/60 shrink-0">
                {formatTimecode(duration_s)}
              </span>
            </div>

            {/* VideoOut segment */}
            <div className="w-[60px] shrink-0 flex items-center justify-center border-l border-clip-video-border/50 bg-clip-video-bg/40 rounded-r-xl">
              <Film size={10} className="text-clip-video/60" />
            </div>
          </>
        ) : (
          <div className="flex-1 min-w-0 flex items-center justify-center gap-1">
            <Film size={10} className="shrink-0 text-clip-video" />
            <span className="text-[10px] text-clip-video truncate">
              {displayName}
            </span>
            <span className="text-[8px] font-mono tabular-nums text-clip-video/60 shrink-0">
              {formatTimecode(duration_s)}
            </span>
          </div>
        )}
      </div>
    </div>
  )
}
