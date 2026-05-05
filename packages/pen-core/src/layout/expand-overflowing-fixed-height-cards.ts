import type { PenNode } from '@zseven-w/pen-types';
import { fitContentHeight } from './engine.js';

/**
 * Recursively walk the tree and switch any `role: 'card'` frame whose
 * fixed numeric height is smaller than its content's natural height
 * to `height: 'fit_content'`. Without this pass, sub-agents that
 * emit a card with a fixed pixel height (e.g. the food-app banner
 * shipping `featured-promo-card { height: 165, clipContent: true }`)
 * combined with content that takes more vertical space than that
 * height end up clipping the bottom rows — the "Order now" button
 * disappears mid-glyph behind the card's clip rect.
 *
 * Why fit_content rather than removing clipContent: clipContent on a
 * card with rounded corners is what makes nested image children
 * respect the card's corner radius. Removing it would fix the button
 * clipping but break image rounding on every card with photos. Just
 * making the card taller keeps both behaviors right.
 *
 * Scope:
 *   - Only `role: 'card'` (and the variants in CARD_ROLES). Other
 *     roles legitimately use fixed heights (avatars, icon buttons,
 *     status pills, etc).
 *   - Only frames with a numeric `height`. `'fill_container'` /
 *     `'fit_content'` are already auto-sizing — no fix needed.
 *   - Only when the natural content height EXCEEDS the declared
 *     fixed height. A card sized 200 with 80px of content stays 200
 *     — that's the model's intentional whitespace, not a bug.
 *
 * Returns true if any card was patched.
 */
const CARD_ROLES = new Set([
  'card',
  'stat-card',
  'pricing-card',
  'feature-card',
  'image-card',
  'testimonial',
  'event-card',
  'product-card',
]);

export function expandOverflowingFixedHeightCards(rootFrame: PenNode): boolean {
  let changed = false;

  const walk = (node: PenNode): void => {
    if (node.type === 'frame') {
      const role = (node as PenNode & { role?: string }).role;
      const height = (node as PenNode & { height?: unknown }).height;
      if (role && CARD_ROLES.has(role) && typeof height === 'number' && height > 0) {
        const natural = fitContentHeight(node);
        if (natural > 0 && natural > height) {
          (node as PenNode & { height?: unknown }).height = 'fit_content';
          changed = true;
        }
      }
    }
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) walk(child);
    }
  };
  walk(rootFrame);

  return changed;
}
