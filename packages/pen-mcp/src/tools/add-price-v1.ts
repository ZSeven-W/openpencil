import { assignIdsRecursively, buildPriceV1, type PriceV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPriceV1Params extends PriceV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Price display (v1) — theme-aware variant of add_price_v0.
 * No hardcoded colors in v0; light/dark/system modes are identical (byte-parity with v0
 * in all modes). Accepts theme param for API consistency across all v1 tools.
 */
export async function handleAddPriceV1(
  params: AddPriceV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildPriceV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'price', tree, ...params });
}
