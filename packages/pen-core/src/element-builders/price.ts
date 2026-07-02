import type { ElementTree } from './helpers.js';

export interface PriceParams {
  amount: string;
  currency?: string;
  period?: string;
}

/**
 * Price display ("$29/month"). Currency 20/500 + amount 40/700 +
 * optional period 14/500, inline with alignItems=flex-end to
 * approximate baseline alignment (pen-core has no baseline).
 * `amount` is a STRING so callers pass pre-formatted values.
 */
export function buildPrice(params: PriceParams): ElementTree {
  const currency = params.currency ?? '$';
  const children: ElementTree[] = [
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
  return {
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
}
