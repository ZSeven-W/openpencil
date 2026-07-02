import {
  assignIdsRecursively,
  buildPricingCardV1,
  type PricingCardV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPricingCardV1Params extends PricingCardV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Pricing plan tier card (v1) — theme-aware variant of add_pricing_card_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * card bg → surface, border → border, text → textPrimary/textMuted/textBody in dark/system;
 * featured accent border/CTA bg and white text stay brand-invariant across all themes.
 */
export async function handleAddPricingCardV1(
  params: AddPricingCardV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildPricingCardV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'pricingCard', tree, ...params });
}
