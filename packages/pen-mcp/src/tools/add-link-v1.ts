import { assignIdsRecursively, buildLinkV1, type LinkV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddLinkV1Params extends LinkV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Inline text link (v1) — theme-aware variant of add_link_v0.
 * No hardcoded colors in v0, so light/dark/system modes are identical
 * (byte-parity with v0 in all modes). Accepts theme param for API
 * consistency across all v1 tools.
 */
export async function handleAddLinkV1(
  params: AddLinkV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildLinkV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'link', tree, ...params });
}
