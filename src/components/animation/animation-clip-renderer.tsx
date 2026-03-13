/**
 * Custom action renderer for animation clip actions.
 * Shows In / Hold / Out segments within a single clip bar.
 * Segment widths are proportional to their duration relative to total clip duration.
 * Colors sourced from --clip-animation design tokens.
 */

import { useDocumentStore } from '@/stores/document-store'
import { isAnimationClip } from '@/types/animation'
import { getEffect } from '@/animation/effect-registry'
import '@/animation/effects'

interface AnimationClipRendererProps {
  clipId: string
}

export default function AnimationClipRenderer({
  clipId,
}: AnimationClipRendererProps) {
  // Read clip data from document store for in/out segment sizing
  const clip = useDocumentStore((s) => {
    for (const node of s.getFlatNodes()) {
      const found = node.clips?.find((c) => c.id === clipId)
      if (found && isAnimationClip(found)) return found
    }
    return undefined
  })

  if (!clip) {
    return (
      <div className="h-full w-full flex items-center justify-center rounded-[14px] bg-clip-animation-bg border border-clip-animation-border">
        <span className="text-[9px] text-clip-animation/60">{clipId.slice(0, 6)}</span>
      </div>
    )
  }

  const inDur = clip.inEffect?.duration ?? 0
  const outDur = clip.outEffect?.duration ?? 0
  const totalDur = clip.duration
  const holdDur = Math.max(0, totalDur - inDur - outDur)

  const inPct = totalDur > 0 ? (inDur / totalDur) * 100 : 0
  const outPct = totalDur > 0 ? (outDur / totalDur) * 100 : 0
  const holdPct = 100 - inPct - outPct

  const inLabel = clip.inEffect ? getEffect(clip.inEffect.effectId)?.name : undefined
  const outLabel = clip.outEffect ? getEffect(clip.outEffect.effectId)?.name : undefined

  const hasSegments = inDur > 0 || outDur > 0

  return (
    <div className="h-full w-full flex items-stretch overflow-hidden rounded-[14px] bg-clip-animation-bg border border-clip-animation-border hover:border-clip-animation hover:border-[1.5px] transition-colors">
      {hasSegments ? (
        <>
          {/* In segment */}
          {inPct > 0 && (
            <div
              className="flex items-center justify-center bg-clip-animation/20 border-r border-clip-animation-border/50"
              style={{ width: `${inPct}%` }}
            >
              <span className="text-[8px] text-clip-animation truncate px-0.5">
                {inLabel ?? 'In'}
              </span>
            </div>
          )}

          {/* Hold segment */}
          <div
            className="flex-1 min-w-0 flex items-center justify-center"
            style={{ width: `${holdPct}%` }}
          >
            {holdDur > 100 && (
              <span className="text-[9px] text-clip-animation/60 truncate px-0.5">
                {(holdDur / 1000).toFixed(1)}s
              </span>
            )}
          </div>

          {/* Out segment */}
          {outPct > 0 && (
            <div
              className="flex items-center justify-center bg-clip-animation/20 border-l border-clip-animation-border/50"
              style={{ width: `${outPct}%` }}
            >
              <span className="text-[8px] text-clip-animation truncate px-0.5">
                {outLabel ?? 'Out'}
              </span>
            </div>
          )}
        </>
      ) : (
        /* No in/out effects — show simple label */
        <div className="flex-1 min-w-0 flex items-center justify-center px-1.5">
          <span className="text-[9px] text-clip-animation truncate">
            {clip.effectId ? (getEffect(clip.effectId)?.name ?? clipId.slice(0, 6)) : 'Custom'}
          </span>
        </div>
      )}
    </div>
  )
}
