import type { PenNode } from '@zseven-w/pen-types';

/**
 * Recursively walk the tree and switch any container that's clearly a
 * "layered hero / banner with overlay" — currently shipped by the
 * model with `layout: 'vertical'` — to `layout: 'none'` so its
 * children stack on top of each other instead of in sequence.
 *
 * Why this exists: weak models (MiniMax M2.7 observed) emit hero
 * sections like
 *   frame { layout: 'vertical', height: 200, children: [
 *     image  { width: 'fill_container', height: 200 }, // full-bg photo
 *     rect   { width: 'fill_container', height: 200 }, // gradient overlay
 *     frame  { ...content with title + cta on top }
 *   ]}
 * intending the image and rectangle to LAYER on top of each other
 * as the background+gradient, with the content frame floating on
 * top. With `layout: 'vertical'` the layout engine instead stacks
 * them in sequence: 200 (image) + 200 (overlay) + content_h ≈ 500+,
 * overflowing the 200 container; combined with no `clipContent`
 * the overflow renders into the NEXT sibling section. The food-app
 * M2.7 run shipped a hero whose overflow drew the "Hungry?" search
 * pile-up over the "Near You" restaurant cards — sections visibly
 * collided.
 *
 * Pattern detection (conservative — false positives are worse than
 * misses):
 *   - The frame has `layout: 'vertical'` (or undefined, which
 *     `inferLayout` would also resolve to vertical).
 *   - The frame has a NUMERIC fixed height `H`.
 *   - At least 2 of the children are visually-large background-
 *     candidate types (`image`, `rectangle`, or `frame`) AND have
 *     `height` exactly equal to `H` (or `'fill_container'`). That's
 *     the load-bearing signal: nobody emits two same-height bg
 *     siblings inside a vertical stack on purpose.
 *
 * Repair: switch the container's `layout` to `'none'`. The layout
 * engine then respects each child's own `x` / `y` (defaulting to
 * 0/0 when missing), which is exactly the layered intent — image
 * at (0,0), overlay at (0,0), content at (0,0) on top.
 *
 * Returns true if any container was patched.
 */
export function convertStackedOverlayToAbsolute(rootFrame: PenNode): boolean {
  let changed = false;

  const walk = (node: PenNode): void => {
    if (node.type === 'frame') {
      const c = node as PenNode & {
        layout?: string;
        height?: unknown;
        children?: PenNode[];
      };
      if ((c.layout === 'vertical' || c.layout === undefined) && typeof c.height === 'number') {
        const containerH = c.height;
        const children = Array.isArray(c.children) ? c.children : [];
        let bgLike = 0;
        for (const child of children) {
          if (child.type !== 'image' && child.type !== 'rectangle' && child.type !== 'frame') {
            continue;
          }
          const childH = (child as PenNode & { height?: unknown }).height;
          if (
            (typeof childH === 'number' && childH === containerH) ||
            childH === 'fill_container'
          ) {
            bgLike += 1;
          }
        }
        if (bgLike >= 2) {
          c.layout = 'none';
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
