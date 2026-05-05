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
 *   - Only `role: 'card'` (and the text-content variants in
 *     CARD_ROLES). NOT `image-card` — that role exists precisely
 *     to lock in a fixed image crop / aspect ratio (a 16:9 photo
 *     tile, a 1:1 thumbnail). Auto-expanding an image-card would
 *     silently turn a 300×180 16:9 crop into a fit_content frame
 *     whose height is whatever fitContentHeight returns, breaking
 *     the intended visual. Authors that need an image card to
 *     auto-grow can use `role: 'card'` with an image child.
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
  // image-card intentionally excluded — see scope note above.
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
