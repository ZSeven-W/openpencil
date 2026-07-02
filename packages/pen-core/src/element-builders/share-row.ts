import type { ElementTree } from './helpers.js';

export interface ShareTarget {
  /** Display label (e.g. "Twitter", "Copy link"). */
  label: string;
  /** Lucide icon slug. */
  icon: string;
}

export interface ShareRowParams {
  targets: ShareTarget[];
}

/**
 * Social-share button row — horizontal list of 40×40 circular icon
 * buttons each labeled below. Distinct from `add_social_login_row_v0`
 * (sign-in CTAs with provider names + "Continue with X"). Use for
 * "Share to X / Y / Z", "Send via", post-success share affordances.
 */
export function buildShareRow(params: ShareRowParams): ElementTree {
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
          fill: [{ type: 'solid', color: '#F1F5F9' }],
          children: [
            {
              type: 'icon_font',
              name: 'Icon',
              iconFontName: target.icon,
              iconFontFamily: 'lucide',
              width: 18,
              height: 18,
              fill: [{ type: 'solid', color: '#475569' }],
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
          fill: [{ type: 'solid', color: '#475569' }],
        },
      ],
    })),
  };
}
