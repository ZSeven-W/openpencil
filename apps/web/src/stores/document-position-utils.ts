import { flattenToRenderNodes, premeasureTextHeights, resolveRefs } from '@zseven-w/pen-renderer';
import type { FrameNode, PenDocument, PenNode } from '@/types/pen';
import { getDefaultTheme, resolveNodeForCanvas } from '@/variables/resolve-variables';
import {
  findNodeInTree,
  findParentInTree,
  getActivePageChildren,
  getAllChildren,
  insertNodeInTree,
  removeNodeFromTree,
} from './document-tree-utils';

export function getNodeVisualPosition(
  doc: PenDocument,
  activePageId: string | null,
  nodeId: string,
): { x: number; y: number } | undefined {
  const pageChildren = getActivePageChildren(doc, activePageId);
  const allNodes = getAllChildren(doc);
  const resolved = resolveRefs(pageChildren, allNodes);
  const variables = doc.variables ?? {};
  const theme = getDefaultTheme(doc.themes);
  const variableResolved = resolved.map((node) => resolveNodeForCanvas(node, variables, theme));
  const measured = premeasureTextHeights(variableResolved);
  const renderNode = flattenToRenderNodes(measured).find((rn) => rn.node.id === nodeId);
  return renderNode ? { x: renderNode.absX, y: renderNode.absY } : undefined;
}

export function moveNodePreservingVisualPosition(
  doc: PenDocument,
  activePageId: string | null,
  id: string,
  newParentId: string | null,
  index?: number,
): PenNode[] | undefined {
  const pageChildren = getActivePageChildren(doc, activePageId);
  const node = findNodeInTree(pageChildren, id);
  if (!node) return undefined;
  const currentParent = findParentInTree(pageChildren, id);

  const currentVisual = getNodeVisualPosition(doc, activePageId, id);
  const parentVisual = newParentId
    ? getNodeVisualPosition(doc, activePageId, newParentId)
    : { x: 0, y: 0 };
  if (!currentVisual || !parentVisual) return undefined;

  const movedNode = {
    ...node,
    x: currentVisual.x - parentVisual.x,
    y: currentVisual.y - parentVisual.y,
  } as PenNode;

  if (node.type === 'frame' && !currentParent && newParentId !== null) {
    const movedFrame = movedNode as FrameNode;
    if (movedFrame.clipContent !== true) {
      movedFrame.clipContent = true;
    }
  }

  return insertNodeInTree(removeNodeFromTree(pageChildren, id), newParentId, movedNode, index);
}
