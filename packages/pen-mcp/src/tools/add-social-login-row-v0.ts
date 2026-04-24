import {
  assignIdsRecursively,
  buildSocialLoginRow,
  type SocialLoginRowParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSocialLoginRowV0Params extends SocialLoginRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { SocialLoginProvider as AddSocialLoginProvider } from '@zseven-w/pen-core';

/** Social-auth button row. Tree build delegated to `buildSocialLoginRow`. */
export async function handleAddSocialLoginRowV0(
  params: AddSocialLoginRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const r = buildSocialLoginRow(params);
  assignIdsRecursively(r);
  return insertElementTree({ binding: 'socialLogin', tree: r, ...params });
}
