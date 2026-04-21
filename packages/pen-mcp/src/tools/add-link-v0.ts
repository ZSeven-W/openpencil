import { assignIdsRecursively, buildLink, type LinkParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddLinkV0Params extends LinkParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Inline text link. Tree build delegated to `buildLink`. */
export async function handleAddLinkV0(
  params: AddLinkV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const link = buildLink(params);
  assignIdsRecursively(link);
  return insertElementTree({ binding: 'link', tree: link, ...params });
}
