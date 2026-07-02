import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface TooltipV1Params {
  /** Tooltip body text (1-2 short lines). */
  text: string;
  /**
   * Where the tooltip is relative to the anchor. Default "top".
   */
  position?: 'top' | 'bottom' | 'left' | 'right';
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_tooltip_v0.
   * - `'dark'`: identical — tooltip is intentionally an inverted/dark surface in all modes.
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Tooltip pill (v1) — theme-aware variant of buildTooltip.
 * Light mode is byte-equal to add_tooltip_v0.
 *
 * Tooltip uses an intentionally dark surface (#111827 slate-900) in all modes —
 * this is an inverted-contrast pattern (dark-on-light tooltip on a light page,
 * dark-on-dark tooltip on a dark page). The colors are UI-paradigm constants,
 * not surface theme tokens. All theme modes produce identical trees.
 */
export function buildTooltipV1(params: TooltipV1Params): ElementTree {
  const position = params.position ?? 'top';
  return {
    type: 'frame',
    name: 'Tooltip',
    role: `tooltip-${position}`,
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    padding: [6, 10],
    cornerRadius: 6,
    fill: [{ type: 'solid', color: '#111827' }],
    children: [
      {
        type: 'text',
        name: 'Text',
        role: 'tooltip-text',
        content: params.text,
        fontSize: 12,
        fontWeight: 500,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
      },
    ],
  };
}
