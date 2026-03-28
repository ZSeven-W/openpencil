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
 * get_selection — Returns the currently selected nodes on the live canvas.
 * Fetches selection state from the Nitro sync endpoint, then reads the
 * full node data for each selected ID from the document.
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
