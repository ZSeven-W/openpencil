import {
  assignIdsRecursively,
  buildInputWithAction,
  type InputWithActionParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInputWithActionV0Params extends InputWithActionParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Input field with inline action button. Tree build delegated to `buildInputWithAction`. */
export async function handleAddInputWithActionV0(
  params: AddInputWithActionV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const r = buildInputWithAction(params);
  assignIdsRecursively(r);
  return insertElementTree({ binding: 'inputWithAction', tree: r, ...params });
}
