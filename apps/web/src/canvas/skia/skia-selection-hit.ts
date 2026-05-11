import { useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

function hasImageVisual(node: PenNode | undefined): boolean {
  if (!node) return false;
  if (node.type === 'image') return true;
  if (!('fill' in node)) return false;
  return Array.isArray(node.fill) && node.fill.some((fill: any) => fill?.type === 'image');
}

export function getPrimarySelectionIdForHit(nodeId: string): string {
  const docStore = useDocumentStore.getState();
  const clickedNode = docStore.getNodeById(nodeId);
  const parent = docStore.getParentOf(nodeId);
  if (
    !hasImageVisual(clickedNode) &&
    parent &&
    (parent.type === 'frame' || parent.type === 'group')
  ) {
    const grandparent = docStore.getParentOf(parent.id);
    if (!grandparent || grandparent.type === 'frame') {
      return parent.id;
    }
  }

  return nodeId;
}
