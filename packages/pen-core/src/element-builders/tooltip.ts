import type { ElementTree } from './helpers.js';

export interface TooltipParams {
  /** Tooltip body text (1-2 short lines). */
  text: string;
  /**
   * Where the tooltip is relative to the anchor — used only to set
   * the `role` hint so downstream post-passes can position it. The
   * tooltip itself does NOT render an arrow pointer (pen-core has
   * no triangle primitive that composes cleanly); callers that
   * need an arrow compose one via batch_design rectangle + rotate.
   * Default "top".
   */
  position?: 'top' | 'bottom' | 'left' | 'right';
}

/**
 * Tooltip pill — small dark pill with white text. Compact informative
 * hover / help label. The open state only (positioned by caller).
 *
 * Structure:
 *   frame(fit_content, horizontal, padding=[6,10], cornerRadius=6,
 *         fill=#111827 slate-900)
 *     └ text(text, 12/500, fill=#FFFFFF, role='tooltip-text')
 *
 * `role` on the outer frame encodes the position variant
 * (`tooltip-top`/`tooltip-bottom`/`tooltip-left`/`tooltip-right`)
 * so downstream positioning logic can differentiate. The visual
 * body is identical regardless.
 */
export function buildTooltip(params: TooltipParams): ElementTree {
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
