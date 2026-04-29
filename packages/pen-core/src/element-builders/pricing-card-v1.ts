import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface PricingCardV1Params {
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
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_pricing_card_v0.
   * - `'dark'`: card bg → surface, border → border (featured: accent invariant),
   *   tier/currency/amount → textPrimary, description/period → textMuted,
   *   feature label → textBody; CTA bg stays brand-invariant.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Pricing plan tier card (v1) — theme-aware variant of buildPricingCard.
 * Light mode is byte-equal to add_pricing_card_v0.
 *
 * Color mapping:
 *   card bg            (#FFFFFF)            → surface
 *   default border     (#E2E8F0 slate-200)  → border
 *   featured border    (#2563EB blue)       — accent, brand-invariant
 *   tier/currency/amt  (#0F172A slate-950)  → textPrimary
 *   description/period (#64748B slate-500)  → textMuted
 *   feature label      (#334155 slate-700)  → textBody
 *   badge bg default   (#F1F5F9 slate-100)  → surface2
 *   badge bg featured  (#EFF6FF blue-50)    — kept as-is (brand-tinted)
 *   badge text default (#334155 slate-700)  → textBody
 *   badge text featured(#1D4ED8 blue-700)   — kept as-is (brand)
 *   default CTA bg     (#0F172A slate-950)  → textPrimary (used as dark surface)
 *   featured CTA bg    (#2563EB blue)       — accent, brand-invariant
 *   CTA text           (#FFFFFF white)      — white-on-accent, invariant
 *   check default      (#10B981 emerald)    → success
 *   check featured     (#2563EB blue)       — accent, brand-invariant
 */
export function buildPricingCardV1(params: PricingCardV1Params): ElementTree {
  const width = Math.max(220, Math.floor(params.width ?? 280));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 16));
  const emphasis = params.emphasis ?? 'default';
  const isFeatured = emphasis === 'featured';

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Brand-invariant accent colors (kept across all themes)
  const featuredAccent = '#2563EB';
  const ctaFg = '#FFFFFF';

  const borderColor = isFeatured ? featuredAccent : isLight ? '#E2E8F0' : t.colors.border;
  const borderThickness = isFeatured ? 2 : 1;
  const ctaBg = isFeatured ? featuredAccent : isLight ? '#0F172A' : t.colors.textPrimary;
  const cardBg = isLight ? '#FFFFFF' : t.colors.surface;
  const tierColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const descColor = isLight ? '#64748B' : t.colors.textMuted;
  const periodColor = isLight ? '#64748B' : t.colors.textMuted;
  const featureLabelColor = isLight ? '#334155' : t.colors.textBody;
  const checkDefault = isLight ? '#10B981' : t.colors.success;

  const features = (params.features ?? []).slice(0, 12);

  const children: ElementTree[] = [];

  if (params.badge || (isFeatured && params.badge !== '')) {
    const badgeLabel = params.badge ?? (isFeatured ? 'Most popular' : '');
    if (badgeLabel) {
      // Badge bg/text: featured uses brand tint (invariant); default uses surface2/textBody
      const badgeBg = isFeatured ? '#EFF6FF' : isLight ? '#F1F5F9' : t.colors.surface2;
      const badgeTextColor = isFeatured ? '#1D4ED8' : isLight ? '#334155' : t.colors.textBody;
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
        fill: [{ type: 'solid', color: badgeBg }],
        children: [
          {
            type: 'text',
            name: 'Badge Label',
            role: 'pricing-badge-label',
            content: badgeLabel,
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: 0.5,
            fill: [{ type: 'solid', color: badgeTextColor }],
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
      fill: [{ type: 'solid', color: tierColor }],
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
      fill: [{ type: 'solid', color: descColor }],
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
      fill: [{ type: 'solid', color: tierColor }],
    },
    {
      type: 'text',
      name: 'Amount',
      role: 'pricing-amount',
      content: params.price,
      fontSize: 40,
      fontWeight: 700,
      lineHeight: 1.0,
      fill: [{ type: 'solid', color: tierColor }],
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
      fill: [{ type: 'solid', color: periodColor }],
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
            fill: [{ type: 'solid', color: isFeatured ? featuredAccent : checkDefault }],
          },
          {
            type: 'text',
            name: 'Label',
            role: 'pricing-feature-label',
            content: text,
            fontSize: 14,
            fontWeight: 400,
            fill: [{ type: 'solid', color: featureLabelColor }],
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
    fill: [{ type: 'solid', color: cardBg }],
    stroke: { thickness: borderThickness, fill: [{ type: 'solid', color: borderColor }] },
    children,
  };
}
