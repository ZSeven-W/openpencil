/**
 * Custom action renderer for animation phase actions.
 * Shows a colored bar with keyframe diamond markers inside.
 */

import type { KeyframePhase, Keyframe } from '@/types/animation'
import { useTimelineStore } from '@/stores/timeline-store'
import { msToSec } from '@/animation/timeline-adapter-types'

// Phase color scheme (oklch, dark theme)
const PHASE_COLORS: Record<KeyframePhase, { bg: string; border: string; diamond: string }> = {
  in: {
    bg: 'oklch(0.55 0.15 155 / 0.20)',
    border: 'oklch(0.65 0.18 155 / 0.50)',
    diamond: 'oklch(0.65 0.18 155)',
  },
  while: {
    bg: 'oklch(0.55 0.12 250 / 0.15)',
    border: 'oklch(0.60 0.16 250 / 0.35)',
    diamond: 'oklch(0.60 0.16 250)',
  },
  out: {
    bg: 'oklch(0.55 0.18 25 / 0.20)',
    border: 'oklch(0.60 0.20 25 / 0.50)',
    diamond: 'oklch(0.60 0.20 25)',
  },
}

interface PhaseActionRendererProps {
  nodeId: string
  phase: KeyframePhase
  actionStart_s: number
  actionEnd_s: number
}

export default function PhaseActionRenderer({
  nodeId,
  phase,
  actionStart_s,
  actionEnd_s,
}: PhaseActionRendererProps) {
  const colors = PHASE_COLORS[phase]
  const actionDuration_s = actionEnd_s - actionStart_s

  // Get keyframes for this phase
  const keyframes = useTimelineStore((s) => {
    const track = s.tracks[nodeId]
    if (!track) return []
    return track.keyframes.filter((kf) => kf.phase === phase)
  })

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        backgroundColor: colors.bg,
        borderRadius: 3,
        border: `1px solid ${colors.border}`,
        position: 'relative',
        overflow: 'hidden',
      }}
    >
      {/* Keyframe diamonds */}
      {keyframes.map((kf) => (
        <KeyframeDiamond
          key={kf.id}
          keyframe={kf}
          actionStart_s={actionStart_s}
          actionDuration_s={actionDuration_s}
          color={colors.diamond}
        />
      ))}

      {/* Phase label (tiny, bottom-left) */}
      <span
        style={{
          position: 'absolute',
          bottom: 2,
          left: 4,
          fontSize: 8,
          color: colors.border,
          userSelect: 'none',
          pointerEvents: 'none',
          lineHeight: 1,
        }}
      >
        {phase}
      </span>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Keyframe diamond
// ---------------------------------------------------------------------------

interface KeyframeDiamondProps {
  keyframe: Keyframe
  actionStart_s: number
  actionDuration_s: number
  color: string
}

function KeyframeDiamond({
  keyframe,
  actionStart_s,
  actionDuration_s,
  color,
}: KeyframeDiamondProps) {
  const kfTime_s = msToSec(keyframe.time)
  const pct = actionDuration_s > 0
    ? ((kfTime_s - actionStart_s) / actionDuration_s) * 100
    : 50

  return (
    <div
      style={{
        position: 'absolute',
        left: `${pct}%`,
        top: '50%',
        width: 8,
        height: 8,
        transform: 'translate(-50%, -50%) rotate(45deg)',
        backgroundColor: color,
        boxShadow: '0 0 0 1px var(--card)',
        cursor: 'pointer',
        transition: 'transform 80ms ease',
      }}
      onMouseEnter={(e) => {
        ;(e.currentTarget as HTMLDivElement).style.transform =
          'translate(-50%, -50%) rotate(45deg) scale(1.3)'
      }}
      onMouseLeave={(e) => {
        ;(e.currentTarget as HTMLDivElement).style.transform =
          'translate(-50%, -50%) rotate(45deg)'
      }}
      title={`${keyframe.easing} @ ${Math.round(keyframe.time)}ms`}
    />
  )
}
