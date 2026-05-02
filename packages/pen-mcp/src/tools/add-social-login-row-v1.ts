import {
  assignIdsRecursively,
  buildSocialLoginRowV1,
  type SocialLoginRowV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSocialLoginRowV1Params extends SocialLoginRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { SocialLoginV1Provider as AddSocialLoginV1Provider } from '@zseven-w/pen-core';

/**
 * Social-auth provider button row (v1) — theme-aware variant of add_social_login_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 */
export async function handleAddSocialLoginRowV1(
  params: AddSocialLoginRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSocialLoginRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'socialLoginRow', tree, ...params });
}
