import type { PenNode } from '@/types/pen';

/**
 * Auto-reparenting a dragged child out of its parent is surprising for
 * frame/shape-style nodes, because users expect those nested objects to keep
 * their parent while being repositioned. Primitive content nodes can still use
 * the legacy "drag out to detach" behavior.
 */
export function shouldAutoReparentOnDragOutsideParent(node: PenNode | undefined): boolean {
  if (!node) return true;

  switch (node.type) {
    case 'frame':
    case 'group':
    case 'rectangle':
    case 'ellipse':
    case 'line':
    case 'polygon':
    case 'path':
    case 'ref':
      return false;
    default:
      return true;
  }
}
