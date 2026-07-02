import { assignIdsRecursively, buildCalloutV1, type CalloutV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCalloutV1Params extends CalloutV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Inline doc callout (v1) — theme-aware variant of add_callout_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Tone-keyed bg/fg mapped to semantic alert palette tokens in dark/system modes.
 * Tree build delegated to `buildCalloutV1` in `@zseven-w/pen-core`.
 */
export async function handleAddCalloutV1(
  params: AddCalloutV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCalloutV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cl', tree, ...params });
}
