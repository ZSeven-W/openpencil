import { assignIdsRecursively, buildSwitch, type SwitchParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSwitchV0Params extends SwitchParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * iOS/Material toggle switch. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildSwitch`.
 */
export async function handleAddSwitchV0(
  params: AddSwitchV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const track = buildSwitch(params);
  assignIdsRecursively(track);
  return insertElementTree({ binding: 'switch', tree: track, ...params });
}
