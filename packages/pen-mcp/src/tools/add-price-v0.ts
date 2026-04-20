import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddPriceV0Params {
  amount: string;
  currency?: string;
  period?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Price display ("$29/month" typography). Three text parts inline:
 * currency glyph (20/500), big amount (40/700), optional period like
 * "/month" (14/500). Amount is a STRING (not number) so callers can
 * supply formatted values with thousands separators or decimals
 * ("1,299" / "29.99") without re-formatting inside the tool.
 *
 * Parts are wrapped in a horizontal fit_content frame with
 * alignItems=flex-end so the baselines of currency/amount/period land on
 * the same visual line — pen-core has no baseline alignItems, but
 * flex-end approximates it well enough for price rows in hero / pricing
 * cards.
 */
export async function handleAddPriceV0(
  params: AddPriceV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const currency = params.currency ?? '$';
  const children: Record<string, unknown>[] = [
    {
      type: 'text',
      name: 'Currency',
      role: 'price-currency',
      content: currency,
      fontSize: 20,
      fontWeight: 500,
    },
    {
      type: 'text',
      name: 'Amount',
      role: 'price-amount',
      content: params.amount,
      fontSize: 40,
      fontWeight: 700,
      lineHeight: 1.0,
    },
  ];
  if (params.period) {
    children.push({
      type: 'text',
      name: 'Period',
      role: 'price-period',
      content: params.period,
      fontSize: 14,
      fontWeight: 500,
    });
  }
  const price = {
    type: 'frame',
    name: 'Price',
    role: 'price',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'flex-end',
    gap: 2,
    children,
  };
  assignIdsRecursively(price);
  return insertElementTree({ binding: 'price', tree: price, ...params });
}
