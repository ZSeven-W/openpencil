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
 * Treat the frame as absolute-positioned when any non-overlay child has an
 * explicit x or y coordinate. Overlays (badges/pills/tags) don't count —
 * they legitimately carry x/y even inside auto-layout frames.
 */
function hasAbsolutePositionedChild(children: PenNode[]): boolean {
  for (const child of children) {
    if (isBadgeOverlayNode(child)) continue;
    const c = child as PenNode & { x?: number; y?: number };
    if (typeof c.x === 'number' || typeof c.y === 'number') return true;
  }
  return false;
}
