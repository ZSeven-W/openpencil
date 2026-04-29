import {
  assignIdsRecursively,
  buildCookieBannerV1,
  type CookieBannerV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCookieBannerV1Params extends CookieBannerV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Cookie consent banner (v1) — theme-aware variant of add_cookie_banner_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Card bg → surface, title → textPrimary, body → textMuted, decline bg → surface2,
 * accept bg → accent (brand-invariant), settings link → accent.
 */
export async function handleAddCookieBannerV1(
  params: AddCookieBannerV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCookieBannerV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cb', tree, ...params });
}
