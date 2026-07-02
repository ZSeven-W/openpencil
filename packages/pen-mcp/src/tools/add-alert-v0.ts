import { assignIdsRecursively, buildAlert, type AlertParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAlertV0Params extends AlertParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Inline alert/callout banner. Tree build delegated to `buildAlert`. */
export async function handleAddAlertV0(
  params: AddAlertV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const alert = buildAlert(params);
  assignIdsRecursively(alert);
  return insertElementTree({ binding: 'alert', tree: alert, ...params });
}
