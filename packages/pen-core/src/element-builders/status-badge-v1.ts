import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export type StatusBadgeV1Tone = 'success' | 'warning' | 'error' | 'info' | 'neutral';

export interface StatusBadgeV1Params {
  label: string;
  /**
   * Semantic tone — picks the dot color. Default 'neutral'.
   */
  tone?: StatusBadgeV1Tone;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_status_badge_v0.
   * - `'dark'`: identical (dot colors are status semantics, theme-independent).
   * - `'system'`: identical.
   *
   * NOTE: status tone colors (success=emerald, warning=amber, error=red,
   * info=blue, neutral=slate) are intentionally theme-independent and kept
   * hardcoded across all theme modes per spec §3.4.
   */
  theme?: V1Theme;
}

/**
 * Status badge (v1) — theme-aware variant of buildStatusBadge.
 * Light mode is byte-equal to add_status_badge_v0.
 *
 * Dot colors are status semantics (not surface colors), kept hardcoded
 * across all theme modes. Accepts theme param for API consistency.
 *
 * Structure:
 *   frame(fit_content, horizontal, gap=6, alignItems=center)
 *     ├ frame(8×8, cornerRadius=4, fill=<toneColor>, role='status-dot')
 *     └ text(label, 13/500)
 */
export function buildStatusBadgeV1(params: StatusBadgeV1Params): ElementTree {
  const tone: StatusBadgeV1Tone = params.tone ?? 'neutral';
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

function dotColorForTone(tone: StatusBadgeV1Tone): string {
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
