import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface PriceV1Params {
  amount: string;
  currency?: string;
  period?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_price_v0.
   * - `'dark'`: identical (v0 emits no hardcoded colors — text
   *   inherits canvas default color).
   * - `'system'`: identical.
   * Accepts theme param for API consistency across all v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Price display (v1) — theme-aware variant of buildPrice.
 * No hardcoded colors in v0; light/dark/system modes are byte-identical.
 * Accepts theme param for API consistency.
 */
export function buildPriceV1(params: PriceV1Params): ElementTree {
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
