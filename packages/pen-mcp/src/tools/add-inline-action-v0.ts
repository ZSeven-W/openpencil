import {
  assignIdsRecursively,
  buildInlineAction,
  type InlineActionParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInlineActionV0Params extends InlineActionParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddInlineActionV0(
  params: AddInlineActionV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const action = buildInlineAction(params);
  assignIdsRecursively(action);
  return insertElementTree({ binding: 'inlineAction', tree: action, ...params });
}
