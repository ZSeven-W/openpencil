import type { ElementTree } from './helpers.js';

export interface PricingCardParams {
  /** Tier name (e.g. "Pro", "Team", "Enterprise"). Required. */
  tier: string;
  /** Price amount (number only, currency rendered separately — e.g. "29", "99", "0"). Required. */
  price: string;
  /** Currency symbol shown before price. Default "$". */
  currency?: string;
  /** Billing period shown after price (e.g. "/month", "/year", "/seat"). */
  period?: string;
  /**
   * Feature list. Each line rendered with a leading check icon.
   * 3-6 features is typical; longer lists compress readability.
   */
  features?: string[];
  /** Optional description beneath the tier name (e.g. "For growing teams"). */
  description?: string;
  /** Optional "Most popular" ribbon badge label. Default empty = no ribbon. */
  badge?: string;
  /** Primary CTA label (e.g. "Get started", "Contact sales"). Default "Get started". */
  cta?: string;
  /**
   * Card emphasis. Default 'default' (white card, slate border).
   * 'featured' renders the accent-tinted/bordered "highlight me" variant.
   */
  emphasis?: 'default' | 'featured';
  /** Card width in px. Default 280. Min 220. */
  width?: number;
  /** Corner radius. Default 16. */
  corner_radius?: number;
}

/**
 * Pricing plan tier card — the "Pro $29/month" card used in SaaS
 * pricing tables. A column built out of:
 *   - tier name + optional description
 *   - big price (currency + amount + period)
 *   - feature list (check + text rows)
 *   - primary CTA button at the bottom
 *
 * Two emphases:
 *   default  → white bg, slate border, slate CTA
 *   featured → white bg, accent (#2563EB) border, accent CTA,
 *              optional "Most popular" pill
 *
 * Structure:
 *   frame(width, fit_content, cornerRadius, padding=24, bg=#FFFFFF,
 *         stroke=accent-or-slate, layout=vertical, gap=20,
 *         role='pricing-card')
 *     ├ frame(badge)                                          ← featured only
 *     ├ frame(header vertical, gap=6, role='pricing-header')
 *     │   ├ text(tier, 18/600, role='pricing-tier')
 *     │   └ text(description, 13/400 muted, role='pricing-description')    ← optional
 *     ├ frame(price horizontal, align=baseline, gap=4, role='pricing-price-row')
 *     │   ├ text(currency, 18/500, role='pricing-currency')
 *     │   ├ text(price, 40/700, role='pricing-amount')
 *     │   └ text(period, 14/500 muted, role='pricing-period')
 *     ├ frame(features vertical, gap=10, role='pricing-features')
 *     │   └ [frame(check+text, horizontal, gap=10, role='pricing-feature')]
 *     └ frame(CTA 44px, fill_container, role='pricing-cta')
 */
export function buildPricingCard(params: PricingCardParams): ElementTree {
  const width = Math.max(220, Math.floor(params.width ?? 280));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 16));
  const emphasis = params.emphasis ?? 'default';
  const isFeatured = emphasis === 'featured';
  const borderColor = isFeatured ? '#2563EB' : '#E2E8F0';
  const borderThickness = isFeatured ? 2 : 1;
  const ctaBg = isFeatured ? '#2563EB' : '#0F172A';
  const ctaFg = '#FFFFFF';
  const features = (params.features ?? []).slice(0, 12);

  const children: ElementTree[] = [];

  if (params.badge || (isFeatured && params.badge !== '')) {
    const badgeLabel = params.badge ?? (isFeatured ? 'Most popular' : '');
    if (badgeLabel) {
      children.push({
        type: 'frame',
        name: 'Badge',
        role: 'pricing-badge',
        width: 'fit_content',
        height: 24,
        cornerRadius: 999,
        layout: 'horizontal',
        alignItems: 'center',
        padding: [0, 10],
        fill: [{ type: 'solid', color: isFeatured ? '#EFF6FF' : '#F1F5F9' }],
        children: [
          {
            type: 'text',
            name: 'Badge Label',
            role: 'pricing-badge-label',
            content: badgeLabel,
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: 0.5,
            fill: [{ type: 'solid', color: isFeatured ? '#1D4ED8' : '#334155' }],
          },
        ],
      });
    }
  }

  const headerChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Tier',
      role: 'pricing-tier',
      content: params.tier,
      fontSize: 18,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#0F172A' }],
    },
  ];
  if (params.description) {
    headerChildren.push({
      type: 'text',
      name: 'Description',
      role: 'pricing-description',
      content: params.description,
      fontSize: 13,
      fontWeight: 400,
      fill: [{ type: 'solid', color: '#64748B' }],
    });
  }
  children.push({
    type: 'frame',
    name: 'Header',
    role: 'pricing-header',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: headerChildren,
  });

  const priceRow: ElementTree[] = [
    {
      type: 'text',
      name: 'Currency',
      role: 'pricing-currency',
      content: params.currency ?? '$',
      fontSize: 18,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#0F172A' }],
    },
    {
      type: 'text',
      name: 'Amount',
      role: 'pricing-amount',
      content: params.price,
      fontSize: 40,
      fontWeight: 700,
      lineHeight: 1.0,
      fill: [{ type: 'solid', color: '#0F172A' }],
    },
  ];
  if (params.period) {
    priceRow.push({
      type: 'text',
      name: 'Period',
      role: 'pricing-period',
      content: params.period,
      fontSize: 14,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#64748B' }],
    });
  }
  children.push({
    type: 'frame',
    name: 'Price Row',
    role: 'pricing-price-row',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'flex-end',
    gap: 4,
    children: priceRow,
  });

  if (features.length > 0) {
    children.push({
      type: 'frame',
      name: 'Features',
      role: 'pricing-features',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      gap: 10,
      children: features.map((text) => ({
        type: 'frame',
        name: 'Feature',
        role: 'pricing-feature',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'horizontal',
        alignItems: 'center',
        gap: 10,
        children: [
          {
            type: 'icon_font',
            name: 'Check',
            role: 'pricing-feature-check',
            iconFontName: 'check',
            iconFontFamily: 'lucide',
            width: 16,
            height: 16,
            fill: [{ type: 'solid', color: isFeatured ? '#2563EB' : '#10B981' }],
          },
          {
            type: 'text',
            name: 'Label',
            role: 'pricing-feature-label',
            content: text,
            fontSize: 14,
            fontWeight: 400,
            fill: [{ type: 'solid', color: '#334155' }],
          },
        ],
      })),
    });
  }

  children.push({
    type: 'frame',
    name: 'CTA',
    role: 'pricing-cta',
    width: 'fill_container',
    height: 44,
    cornerRadius: 10,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    fill: [{ type: 'solid', color: ctaBg }],
    children: [
      {
        type: 'text',
        name: 'CTA Label',
        role: 'pricing-cta-label',
        content: params.cta ?? 'Get started',
        fontSize: 14,
        fontWeight: 600,
        fill: [{ type: 'solid', color: ctaFg }],
      },
    ],
  });

  return {
    type: 'frame',
    name: 'Pricing Card',
    role: 'pricing-card',
    width,
    height: 'fit_content',
    cornerRadius,
    layout: 'vertical',
    gap: 20,
    padding: 24,
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: borderThickness, fill: [{ type: 'solid', color: borderColor }] },
    children,
  };
}
