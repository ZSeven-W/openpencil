import { assignIdsRecursively, buildPricingCard, type PricingCardParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPricingCardV0Params extends PricingCardParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** SaaS pricing-tier card. Tree build delegated to `buildPricingCard`. */
export async function handleAddPricingCardV0(
  params: AddPricingCardV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const r = buildPricingCard(params);
  assignIdsRecursively(r);
  return insertElementTree({ binding: 'pricingCard', tree: r, ...params });
}
