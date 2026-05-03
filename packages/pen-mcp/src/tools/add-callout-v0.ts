import { assignIdsRecursively, buildCallout, type CalloutParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCalloutV0Params extends CalloutParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddCalloutV0(
  params: AddCalloutV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const callout = buildCallout(params);
  assignIdsRecursively(callout);
  return insertElementTree({ binding: 'callout', tree: callout, ...params });
}
