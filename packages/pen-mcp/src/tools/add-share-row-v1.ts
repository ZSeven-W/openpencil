import { assignIdsRecursively, buildShareRowV1, type ShareRowV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddShareRowV1Params extends ShareRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Social-share button row (v1) — theme-aware variant of add_share_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Icon button bg → surface2; icon fill + label → textMuted in dark/system.
 */
export async function handleAddShareRowV1(
  params: AddShareRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildShareRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'shareRow', tree, ...params });
}
