import { assignIdsRecursively, buildSettingRow, type SettingRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSettingRowV0Params extends SettingRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddSettingRowV0(
  params: AddSettingRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildSettingRow(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'settingRow', tree: row, ...params });
}
