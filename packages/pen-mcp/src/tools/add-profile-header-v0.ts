import {
  assignIdsRecursively,
  buildProfileHeader,
  type ProfileHeaderParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddProfileHeaderV0Params extends ProfileHeaderParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddProfileHeaderV0(
  params: AddProfileHeaderV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const header = buildProfileHeader(params);
  assignIdsRecursively(header);
  return insertElementTree({ binding: 'profileHeader', tree: header, ...params });
}
