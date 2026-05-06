import type { PenNode } from '@/types/pen';

/**
 * 对于 frame/sha
 * pe-style 节点来说，将被拖出其父级的子级重新设置为父级是令人惊讶的，因为用户希望这些嵌套对象在重新定位时保留其父级。
 * Primitive 内容节点仍然可以使用旧的“拖出以分离”行为。
 *
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
