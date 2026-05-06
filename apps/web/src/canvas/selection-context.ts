import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

/**
 * Pure 用于深度感知选
 * 择的实用程序模块。 Determines 在当前输入帧深度可选择哪些节点。
 */

/** Returns 在当前深度可选择的节点 IDs 的集合。 */
export function getSelectableNodeIds(): Set<string> {
  const { enteredFrameId } = useCanvasStore.getState().selection;
  const doc = useDocumentStore.getState().document;

  if (!enteredFrameId) {
    // Root level：仅活动页面的顶级子级可选择
    const activePageId = useCanvasStore.getState().activePageId;
    const children = getActivePageChildren(doc, activePageId);
    return new Set(children.map((n) => n.id));
  }

  const frame = useDocumentStore.getState().getNodeById(enteredFrameId);
  if (!frame || !('children' in frame) || !frame.children) {
    return new Set();
  }
  return new Set(frame.children.map((n) => n.id));
}

/**
 * Given a
 * Fabric 目标的 penNodeId，将其解析为当前深度的可选节点。 Walks 在父链上向上移动，直到在可选择集中找到一个节点。
 * Returns null 如果目标完全在当前上下文之外（例如，当在输入的框架内时属于不同的根框架）。
 *
 *
 *
 */
export function resolveTargetAtDepth(nodeId: string): string | null {
  const selectableIds = getSelectableNodeIds();

  // Direct 比赛
  if (selectableIds.has(nodeId)) return nodeId;

  // Handle 虚拟实例子 IDs (refid__childid)
  if (nodeId.includes('__')) {
    const refId = nodeId.substring(0, nodeId.indexOf('__'));
    if (selectableIds.has(refId)) return refId;
    // Walk 从 RefNode 向上
    let cur: string | undefined = refId;
    while (cur) {
      const parent = useDocumentStore.getState().getParentOf(cur);
      if (!parent) break;
      if (selectableIds.has(parent.id)) return parent.id;
      cur = parent.id;
    }
  }

  // Walk 向上父链
  let currentId: string | undefined = nodeId;
  while (currentId) {
    const parent = useDocumentStore.getState().getParentOf(currentId);
    if (!parent) break;
    if (selectableIds.has(parent.id)) return parent.id;
    currentId = parent.id;
  }

  return null;
}

/** Check 节点是否是可以通过双击“进入”的容器。 */
export function isEnterableContainer(nodeId: string): boolean {
  const node = useDocumentStore.getState().getNodeById(nodeId);
  if (!node) return false;
  if (node.type !== 'frame' && node.type !== 'group') return false;
  if (!('children' in node) || !node.children || node.children.length === 0) return false;
  return true;
}

/** Return 容器节点的直接子节点 IDs。 */
export function getChildIds(nodeId: string): Set<string> {
  const node = useDocumentStore.getState().getNodeById(nodeId);
  if (!node || !('children' in node) || !node.children) return new Set();
  return new Set(node.children.map((n: PenNode) => n.id));
}
