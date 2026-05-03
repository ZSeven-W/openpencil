import {
  assignIdsRecursively,
  buildCookieBanner,
  type CookieBannerParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCookieBannerV0Params extends CookieBannerParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Cookie consent banner. Tree build delegated to `buildCookieBanner`. */
export async function handleAddCookieBannerV0(
  params: AddCookieBannerV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const r = buildCookieBanner(params);
  assignIdsRecursively(r);
  return insertElementTree({ binding: 'cookieBanner', tree: r, ...params });
}
