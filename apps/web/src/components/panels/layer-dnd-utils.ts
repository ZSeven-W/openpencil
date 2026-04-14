import type { PenNode } from '@/types/pen';
import type { DropPosition } from './layer-item';

export interface LayerDropMove {
  parentId: string | null;
  index: number;
  preserveAbsolutePosition: boolean;
}

/**
 * Resolve the destination for a layer-panel drag/drop and whether the move
 * needs absolute-position preservation because it crosses parent boundaries.
 */
export function resolveLayerDropMove(
  dragId: string,
  overId: string,
  pos: Exclude<DropPosition, null>,
  rootChildren: PenNode[],
  getParentOf: (id: string) => PenNode | undefined,
): LayerDropMove | null {
  const currentParentId = getParentOf(dragId)?.id ?? null;

  if (pos === 'inside') {
    return {
      parentId: overId,
      index: 0,
      preserveAbsolutePosition: currentParentId !== overId,
    };
  }

  const parent = getParentOf(overId);
  const parentId = parent?.id ?? null;
  const siblings = parent ? ('children' in parent ? (parent.children ?? []) : []) : rootChildren;
  const targetIdx = siblings.findIndex((n) => n.id === overId);
  if (targetIdx === -1) return null;

  return {
    parentId,
    index: pos === 'above' ? targetIdx : targetIdx + 1,
    preserveAbsolutePosition: currentParentId !== parentId,
  };
}
