import type { ElementTree } from './helpers.js';

export type StatusBadgeTone = 'success' | 'warning' | 'error' | 'info' | 'neutral';

export interface StatusBadgeParams {
  label: string;
  /**
   * Semantic tone — picks the dot color + subtle text color. When
   * callers want a custom color palette, override via a follow-up
   * batch_design U-op on the returned nodes. Default 'neutral'.
   */
  tone?: StatusBadgeTone;
}

/**
 * Status indicator pill: small colored dot + short label. The "●
 * Online" / "● Busy" / "● Error" pattern. Always emits a dot
 * (that's what makes it visually a "status") so it stays distinct
 * from the more general `add_badge_v0` which is just a pill label.
 *
 * Structure:
 *   frame(fit_content, horizontal, gap=6, alignItems=center)
 *     ├ frame(8×8, cornerRadius=4, fill=<toneColor>, role='status-dot')
 *     └ text(label, 13/500)
 *
 * Dot uses `frame + cornerRadius=4` (pill), NOT `ellipse` — a small
 * 8×8 ellipse is the classic "status dot = stacked ellipses" anti-
 * pattern bait. Keep it a frame to stay clean of rewriteLlmAntiPatterns.
 */
export function buildStatusBadge(params: StatusBadgeParams): ElementTree {
  const tone: StatusBadgeTone = params.tone ?? 'neutral';
  const dotColor = dotColorForTone(tone);

  return {
    type: 'frame',
    name: 'Status Badge',
    role: 'status-badge',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 6,
    children: [
      {
        type: 'frame',
        name: 'Status Dot',
        role: 'status-dot',
        width: 8,
        height: 8,
        cornerRadius: 4,
        fill: [{ type: 'solid', color: dotColor }],
      },
      {
        type: 'text',
        name: 'Label',
        role: 'status-label',
        content: params.label,
        fontSize: 13,
        fontWeight: 500,
      },
    ],
  };
}

function dotColorForTone(tone: StatusBadgeTone): string {
  switch (tone) {
    case 'success':
      return '#10B981'; // emerald-500
    case 'warning':
      return '#F59E0B'; // amber-500
    case 'error':
      return '#EF4444'; // red-500
    case 'info':
      return '#3B82F6'; // blue-500
    case 'neutral':
    default:
      return '#94A3B8'; // slate-400
  }
}
