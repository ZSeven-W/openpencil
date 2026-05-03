import {
  assignIdsRecursively,
  buildSettingRowV1,
  type SettingRowV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSettingRowV1Params extends SettingRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Settings menu row (v1) — theme-aware variant of add_setting_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Tree build delegated to `buildSettingRowV1` in `@zseven-w/pen-core`.
 */
export async function handleAddSettingRowV1(
  params: AddSettingRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSettingRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'sr', tree, ...params });
}
