import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface CookieBannerV1Params {
  /** Headline. Default "We use cookies". */
  title?: string;
  /** Body / disclosure text. Default a generic disclosure paragraph. */
  body?: string;
  /** Accept button label. Default "Accept all". */
  accept_label?: string;
  /** Decline button label. Default "Reject". */
  decline_label?: string;
  /**
   * When true, render a third "Cookie settings" link below the
   * buttons (the GDPR fine-grained consent affordance). Default
   * false.
   */
  show_settings_link?: boolean;
  /** Cookie settings link label. Default "Cookie settings". */
  settings_label?: string;
  /** Banner width in px. Default 720. Min 320. */
  width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_cookie_banner_v0.
   * - `'dark'`: dark surface/text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Cookie consent banner — theme-aware version of buildCookieBanner.
 * Light mode is byte-equal to add_cookie_banner_v0.
 *
 * Color mapping:
 *   card bg (#FFFFFF)            → surface
 *   card stroke (#E2E8F0)        → border
 *   title (#0F172A)              → textPrimary
 *   body (#475569)               → textMuted
 *   decline bg (#F1F5F9)         → surface2
 *   decline fg (#0F172A)         → textPrimary
 *   accept bg (#2563EB)          → accent (brand-invariant in all modes)
 *   accept fg (#FFFFFF)          → kept as-is (white on accent)
 *   settings link (#2563EB)      → accent
 *   shadow (#0F172A26)           → kept as-is (shadow is theme-agnostic)
 */
export function buildCookieBannerV1(params: CookieBannerV1Params): ElementTree {
  const width = Math.max(320, Math.floor(params.width ?? 720));
  const title = params.title ?? 'We use cookies';
  const body =
    params.body ??
    'We use cookies to enhance your experience, analyze site traffic, and personalize content.';
  const acceptLabel = params.accept_label ?? 'Accept all';
  const declineLabel = params.decline_label ?? 'Reject';
  const showSettings = params.show_settings_link ?? false;
  const settingsLabel = params.settings_label ?? 'Cookie settings';
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const surfaceColor = isLight ? '#FFFFFF' : t.colors.surface;
  const borderColor = isLight ? '#E2E8F0' : t.colors.border;
  const titleColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const bodyColor = isLight ? '#475569' : t.colors.textMuted;
  const declineBg = isLight ? '#F1F5F9' : t.colors.surface2;
  const declineFg = isLight ? '#0F172A' : t.colors.textPrimary;
  // accent + white-on-accent are brand-invariant in all modes
  const accentColor = isLight ? '#2563EB' : t.colors.accent;

  const declineButton: ElementTree = {
    type: 'frame',
    name: 'Decline Button',
    role: 'cookie-banner-decline',
    width: 'fit_content',
    height: 40,
    cornerRadius: 8,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    padding: [0, 18],
    fill: [{ type: 'solid', color: declineBg }],
    children: [
      {
        type: 'text',
        name: 'Decline Label',
        role: 'cookie-banner-decline-label',
        content: declineLabel,
        fontSize: 13,
        fontWeight: 500,
        fill: [{ type: 'solid', color: declineFg }],
      },
    ],
  };

  const acceptButton: ElementTree = {
    type: 'frame',
    name: 'Accept Button',
    role: 'cookie-banner-accept',
    width: 'fit_content',
    height: 40,
    cornerRadius: 8,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    padding: [0, 18],
    fill: [{ type: 'solid', color: accentColor }],
    children: [
      {
        type: 'text',
        name: 'Accept Label',
        role: 'cookie-banner-accept-label',
        content: acceptLabel,
        fontSize: 13,
        fontWeight: 600,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
      },
    ],
  };

  const children: ElementTree[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'cookie-banner-title',
      content: title,
      fontSize: 16,
      fontWeight: 600,
      fill: [{ type: 'solid', color: titleColor }],
    },
    {
      type: 'text',
      name: 'Body',
      role: 'cookie-banner-body',
      content: body,
      fontSize: 13,
      fontWeight: 400,
      lineHeight: 1.5,
      fill: [{ type: 'solid', color: bodyColor }],
    },
    {
      type: 'frame',
      name: 'Actions',
      role: 'cookie-banner-actions',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      gap: 12,
      children: [declineButton, acceptButton],
    },
  ];

  if (showSettings) {
    children.push({
      type: 'text',
      name: 'Settings Link',
      role: 'cookie-banner-settings',
      content: settingsLabel,
      fontSize: 12,
      fontWeight: 500,
      fill: [{ type: 'solid', color: accentColor }],
    });
  }

  return {
    type: 'frame',
    name: 'Cookie Banner',
    role: 'cookie-banner',
    width,
    height: 'fit_content',
    cornerRadius: 12,
    layout: 'vertical',
    gap: 12,
    padding: 20,
    fill: [{ type: 'solid', color: surfaceColor }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: borderColor }] },
    effects: [
      {
        type: 'shadow',
        offsetX: 0,
        offsetY: 8,
        blur: 24,
        spread: 0,
        color: '#0F172A26',
      },
    ],
    children,
  };
}
