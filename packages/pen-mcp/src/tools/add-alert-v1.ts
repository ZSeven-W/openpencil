import { assignIdsRecursively, buildAlertV1, type AlertV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAlertV1Params extends AlertV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Inline alert / callout banner (v1) — theme-aware variant of add_alert_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Since buildAlert emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildAlertV1` in `@zseven-w/pen-core`.
 */
export async function handleAddAlertV1(
  params: AddAlertV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildAlertV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'al', tree, ...params });
}
