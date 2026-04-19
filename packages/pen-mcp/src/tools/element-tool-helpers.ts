import { openDocument, resolveDocPath } from '../document-manager';
import { findNodeInTree, getDocChildren } from '../utils/node-operations';

/**
 * Validate that `parent_id` refers to an existing node before passing it to
 * handleBatchDesign. batch_design's underlying `insertNodeInTree` silently
 * returns the original tree when the parent is missing, producing a
 * success-looking response (binding + nodeId) with an orphaned node that
 * never lands on disk. Element tools (add_scroll_row_v0 / add_bottom_nav_v0
 * / add_activity_ring_v0) must fail fast with a clear error instead.
 *
 * Skips validation when parent_id is falsy (root-level insertion is always
 * valid). Throws a descriptive Error otherwise.
 */
export async function ensureParentExists(params: {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}): Promise<void> {
  if (!params.parent_id) return;
  const fp = resolveDocPath(params.filePath);
  const doc = await openDocument(fp);
  const children = getDocChildren(doc, params.pageId);
  const found = findNodeInTree(children, params.parent_id);
  if (!found) {
    throw new Error(
      `parent_id "${params.parent_id}" not found in document${
        params.pageId ? ` (pageId=${params.pageId})` : ''
      }. Pass a valid parent node id or omit parent_id for root-level insertion.`,
    );
  }
}
