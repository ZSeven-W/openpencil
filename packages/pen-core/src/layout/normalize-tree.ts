import type { PenNode, ContainerProps } from '@zseven-w/pen-types';
import { isBadgeOverlayNode } from '../node-helpers.js';
import { inferLayout } from './engine.js';

/**
 * Normalize layout state across a node tree (mutates in place).
 *
 * Two fixes applied recursively to every frame:
 *
 * 1. When a frame has children but no explicit `layout`, write one:
 *    - First try `inferLayout()` (horizontal signals: gap, padding,
 *      fill_container children).
 *    - If that returns undefined and the frame has 2+ children, fall back
 *      to `vertical` — BUT only when no non-overlay child carries explicit
 *      `x`/`y`. Any such coordinate is treated as a deliberate signal that
 *      the frame is an absolute-positioning container (phone mockups, hero
 *      images with floating overlays, etc.) and we leave it alone.
 *
 * 2. When a frame has an active layout (`vertical` or `horizontal`), strip
 *    `x`/`y` from every non-overlay child. Overlay children (badges, pills,
 *    tags, floating indicators) keep their absolute coordinates.
 *
 * Used as a post-generation pass after an AI model produces a subtree. It
 * corrects two common model mistakes:
 *   - Forgetting to set `layout` on a container (children would otherwise
 *     stack at (0,0) because `computeLayoutPositions` skips layout-less
 *     parents).
 *   - Leaving stale `x`/`y` on children of an auto-layout frame (causes
 *     visible misalignment when the layout engine also tries to position
 *     them).
 *
 * Ordering requirement: MUST run AFTER any role-based resolution pass that
 * populates `layout` from semantic roles (e.g. navbar → 'horizontal').
 * Running this first would write the generic 'vertical' fallback onto a
 * navbar frame, and the later role resolver — which only fills undefined
 * fields — would refuse to overwrite it. Treat this function as the last
 * safety net, not the first opinion.
 */
export function normalizeTreeLayout(node: PenNode): void {
  if (node.type === 'frame' && 'children' in node && Array.isArray(node.children)) {
    const c = node as PenNode & ContainerProps;
    const children = node.children;

    // (1) Ensure an explicit layout when children exist.
    // Only fill in when `layout` is missing — an explicit `'none'` is
    // intentional (absolute positioning) and must be preserved.
    if (c.layout == null && children.length > 0) {
      const inferred = inferLayout(node);
      if (inferred) {
        c.layout = inferred;
      } else if (children.length >= 2 && !hasAbsolutePositionedChild(children)) {
        // Safe to treat as a "model forgot layout" case: nobody carries x/y,
        // so there's no absolute-positioning intent to destroy.
        c.layout = 'vertical';
      }
    }

    // (2) Strip x/y from non-overlay children of active-layout frames.
    if (c.layout === 'vertical' || c.layout === 'horizontal') {
      for (const child of children) {
        if (!isBadgeOverlayNode(child)) {
          if ('x' in child) delete (child as { x?: number }).x;
          if ('y' in child) delete (child as { y?: number }).y;
        }
      }
    }
  }

  // Recurse into children regardless of node type (groups/pages may nest frames).
  if ('children' in node && Array.isArray(node.children)) {
    for (const child of node.children) {
      normalizeTreeLayout(child);
    }
  }
}

/**
 * Decide whether a layout-less parent should be treated as an absolute-
 * positioning container (skip the `vertical` fallback) instead of being
 * verticalized.
 *
 * Two signals count as "this is an absolute-positioning container":
 *
 *   1. A non-overlay child has a numeric `x` or `y` (including 0). Models
 *      that explicitly position children clearly intend absolute layout.
 *
 *   2. EVERY non-overlay child is a `frame`. AI models routinely emit
 *      nested-frame compositions at (0,0) without writing x/y at all
 *      (rings + center text, badge stacks, hero overlays, multi-layer
 *      cards). The previous check returned false on these because their
 *      x/y fields were absent, and `normalizeTreeLayout` would silently
 *      rewrite the parent to `layout: 'vertical'`, stacking the overlays
 *      into a vertical list and clipping anything past the parent's
 *      bounds. When ALL children are frames, the most likely intent is
 *      structured composition (each child has its own internal layout),
 *      not a generic content stack — and the conservative thing to do
 *      is leave `layout` undefined so the renderer treats it as overlay.
 *      Mixed types (frame + rect, frame + text, etc.) still get the
 *      vertical fallback, since those are typically content stacks where
 *      verticalization is the right call.
 *
 * Overlay nodes (badges/pills/tags via `isBadgeOverlayNode`) are excluded
 * from the count — they legitimately carry x/y inside auto-layout frames
 * and shouldn't tip the all-frame heuristic.
 *
 * This is intentionally conservative: it accepts a few false negatives
 * (some genuinely vertical all-frame stacks will be left un-normalized,
 * forcing the AI to declare layout explicitly) to eliminate the
 * catastrophic false-positive of silently verticalizing an overlay
 * composition.
 */
function hasAbsolutePositionedChild(children: PenNode[]): boolean {
  // Signal 1: explicit numeric x/y on any non-overlay child.
  for (const child of children) {
    if (isBadgeOverlayNode(child)) continue;
    const c = child as PenNode & { x?: number; y?: number };
    if (typeof c.x === 'number' || typeof c.y === 'number') return true;
  }

  // Signal 2: every non-overlay child is a frame (>= 2 such children).
  let nonOverlayCount = 0;
  let frameCount = 0;
  for (const child of children) {
    if (isBadgeOverlayNode(child)) continue;
    nonOverlayCount++;
    if (child.type === 'frame') frameCount++;
  }
  if (nonOverlayCount >= 2 && frameCount === nonOverlayCount) return true;

  return false;
}
