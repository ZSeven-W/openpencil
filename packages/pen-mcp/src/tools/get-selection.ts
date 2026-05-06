import { openDocument, resolveDocPath, fetchLiveSelection } from '../document-manager';
import { findNodeInTree, readNodeWithDepth, getDocChildren } from '../utils/node-operations';

export interface GetSelectionParams {
  filePath?: string;
  readDepth?: number;
}

export interface GetSelectionResult {
  selectedIds: string[];
  activePageId: string | null;
  nodes: Record<string, unknown>[];
}

/**
 * get_selectio
 * n — Returns 实时画布上当前选定的节点。 Fetches 从 Nitro 同步端点选择状态，然后从文档中读取每个选定的 ID
 * 的完整节点数据。
 */
export async function handleGetSelection(params: GetSelectionParams): Promise<GetSelectionResult> {
  const { selectedIds, activePageId } = await fetchLiveSelection();

  if (selectedIds.length === 0) {
    return { selectedIds: [], activePageId, nodes: [] };
  }

  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);
  const readDepth = params.readDepth ?? 2;
  const children = getDocChildren(doc, activePageId ?? undefined);

  const nodes: Record<string, unknown>[] = [];
  for (const id of selectedIds) {
    const node = findNodeInTree(children, id);
    if (node) {
      nodes.push(readNodeWithDepth(node, readDepth));
    }
  }

  return { selectedIds, activePageId, nodes };
}
