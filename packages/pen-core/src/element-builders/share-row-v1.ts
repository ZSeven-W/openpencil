import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ShareV1Target {
  /** Display label (e.g. "Twitter", "Copy link"). */
  label: string;
  /** Lucide icon slug. */
  icon: string;
}

export interface ShareRowV1Params {
  targets: ShareV1Target[];
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_share_row_v0.
   * - `'dark'`: icon button bg → surface2; icon fill → textMuted; label → textMuted.
   * - `'system'`: $color-* refs for bg and icon/label colors.
   */
  theme?: V1Theme;
}

/**
 * Social-share button row (v1) — theme-aware variant of buildShareRow.
 * Light mode is byte-equal to add_share_row_v0.
 *
 * Color mapping:
 *   icon button bg (#F1F5F9 slate-100) → surface2 token
 *   icon fill      (#475569 slate-600) → textMuted token
 *   label text     (#475569 slate-600) → textMuted token
 */
export function buildShareRowV1(params: ShareRowV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Icon button bg: light → slate-100 (#F1F5F9), dark/system → surface2
  const iconBg = isLight ? '#F1F5F9' : t.colors.surface2;
  // Icon and label color: light → slate-600 (#475569), dark/system → textMuted
  const iconColor = isLight ? '#475569' : t.colors.textMuted;

  return {
    type: 'frame',
    name: 'Share Row',
    role: 'share-row',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'start',
    gap: 16,
    children: params.targets.map((target, i) => ({
      type: 'frame',
      name: `Target ${i + 1}`,
      role: 'share-target',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'vertical',
      alignItems: 'center',
      gap: 6,
      children: [
        {
          type: 'frame',
          name: 'Icon Button',
          role: 'share-target-icon',
          width: 40,
          height: 40,
          cornerRadius: 20,
          layout: 'horizontal',
          alignItems: 'center',
          justifyContent: 'center',
          fill: [{ type: 'solid', color: iconBg }],
          children: [
            {
              type: 'icon_font',
              name: 'Icon',
              iconFontName: target.icon,
              iconFontFamily: 'lucide',
              width: 18,
              height: 18,
              fill: [{ type: 'solid', color: iconColor }],
            },
          ],
        },
        {
          type: 'text',
          name: 'Label',
          role: 'share-target-label',
          content: target.label,
          fontSize: 11,
          fontWeight: 500,
          fill: [{ type: 'solid', color: iconColor }],
        },
      ],
    })),
  };
}
