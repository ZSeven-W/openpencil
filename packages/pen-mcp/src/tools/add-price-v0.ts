import { assignIdsRecursively, buildPrice, type PriceParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPriceV0Params extends PriceParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Price display ("$29/month"). Tree build delegated to `buildPrice`. */
export async function handleAddPriceV0(
  params: AddPriceV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const price = buildPrice(params);
  assignIdsRecursively(price);
  return insertElementTree({ binding: 'price', tree: price, ...params });
}
