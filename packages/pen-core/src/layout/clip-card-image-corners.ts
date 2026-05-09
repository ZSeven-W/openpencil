import type { PenNode } from '@zseven-w/pen-types';

/**
 * Card-image corner cleanup.
 *
 * When a sub-agent emits a card-style frame whose FIRST child is a
 * full-width image (or canonical image-placeholder) AND that image
 * carries its own scalar cornerRadius, the rendering shows the image
 * with all four corners rounded — most visibly, the BOTTOM corners
 * are rounded inside the card while the title text below sits flush
 * against the card's flat surface, producing the visual artifact the
 * 2026-05-10 user report flagged on the Taco Fiesta card. Real product
 * cards either:
 *   (a) clip the parent card with `clipContent: true` so the image's
 *       hypothetical corners are clipped to the card's outer corners
 *       (top corners rounded, bottom flush against the title), OR
 *   (b) set the image's cornerRadius to the per-corner array form
 *       `[topLeft, topRight, 0, 0]` so only the top corners round.
 *
 * This pass enforces (a): card-shape parents get `clipContent: true`
 * and the first-child image has its own scalar cornerRadius removed.
 * The visual outcome is identical to (b) without needing per-corner
 * support in every sub-agent.
 *
 * Match policy — conservative, fires only when ALL of:
 *   - parent is a frame with scalar cornerRadius > 0 (it IS a card-shape)
 *   - parent has 2+ children (the image plus title/rating/etc., not a
 *     standalone image)
 *   - first child is `type: 'image'` OR `role: 'image-placeholder'`
 *   - first child has its own scalar cornerRadius (otherwise nothing to
 *     fix and clipContent is a no-op)
 *
 * Returns true when any node was modified.
 */
export function clipCardImageCorners(rootNode: PenNode): boolean {
  let changed = false;
  walk(rootNode);
  return changed;

  function walk(node: PenNode): void {
    const isFrame = node.type === 'frame' || node.type === 'group';
    if (isFrame && 'children' in node && Array.isArray(node.children)) {
      maybeFix(node);
      for (const child of node.children) walk(child);
    }
  }

  function maybeFix(parent: PenNode): void {
    const p = parent as PenNode & {
      cornerRadius?: unknown;
      clipContent?: boolean;
      children?: PenNode[];
    };
    const radius = p.cornerRadius;
    if (typeof radius !== 'number' || radius <= 0) return;
    if (!Array.isArray(p.children) || p.children.length < 2) return;
    const first = p.children[0];
    if (!isImageNode(first)) return;
    const f = first as PenNode & { cornerRadius?: unknown };
    if (typeof f.cornerRadius !== 'number' || f.cornerRadius <= 0) return;

    // 1) Set clipContent on the parent so the parent's cornerRadius clips
    //    every descendant (the image included). If clipContent was already
    //    true, leave it alone.
    if (p.clipContent !== true) {
      p.clipContent = true;
      changed = true;
    }
    // 2) Remove the image's own cornerRadius — it would otherwise round
    //    the image's bottom corners, which look ragged against the title
    //    that sits flush below.
    delete f.cornerRadius;
    changed = true;
  }
}

function isImageNode(node: PenNode | undefined | null): boolean {
  if (!node) return false;
  if (node.type === 'image') return true;
  if (node.type === 'frame') {
    const role = (node as PenNode & { role?: string }).role;
    if (role === 'image-placeholder') return true;
  }
  return false;
}
