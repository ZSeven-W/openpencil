import type { ElementTree } from './helpers.js';

export interface CookieBannerParams {
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
}

/**
 * Cookie consent banner — the sticky bottom-of-page GDPR/CCPA
 * disclosure with title, body, accept/decline buttons, and an
 * optional fine-grained settings link. Caller positions the banner
 * (typically `position: sticky; bottom: 0` on web, or as the last
 * child of a fill-screen scrim layer in mockups). The tool emits
 * the banner card itself, not the positioning chrome.
 *
 * Structure:
 *   frame(width, fit_content, cornerRadius=12, padding=20, bg=#FFFFFF,
 *         stroke=#E2E8F0, layout=vertical, gap=12, role='cookie-banner')
 *     ├ text(title, 16/600 slate-900, role='cookie-banner-title')
 *     ├ text(body, 13/400 slate-600, role='cookie-banner-body')
 *     ├ frame(actions horizontal, gap=12, role='cookie-banner-actions')
 *     │   ├ frame(decline button — secondary, slate-100 bg, slate-900 fg)
 *     │   └ frame(accept button — primary, accent #2563EB bg, white fg)
 *     └ link(settings_label, role='cookie-banner-settings')   ← if show_settings_link
 */
export function buildCookieBanner(params: CookieBannerParams): ElementTree {
  const width = Math.max(320, Math.floor(params.width ?? 720));
  const title = params.title ?? 'We use cookies';
  const body =
    params.body ??
    'We use cookies to enhance your experience, analyze site traffic, and personalize content.';
  const acceptLabel = params.accept_label ?? 'Accept all';
  const declineLabel = params.decline_label ?? 'Reject';
  const showSettings = params.show_settings_link ?? false;
  const settingsLabel = params.settings_label ?? 'Cookie settings';

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
    fill: [{ type: 'solid', color: '#F1F5F9' }],
    children: [
      {
        type: 'text',
        name: 'Decline Label',
        role: 'cookie-banner-decline-label',
        content: declineLabel,
        fontSize: 13,
        fontWeight: 500,
        fill: [{ type: 'solid', color: '#0F172A' }],
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
    fill: [{ type: 'solid', color: '#2563EB' }],
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
      fill: [{ type: 'solid', color: '#0F172A' }],
    },
    {
      type: 'text',
      name: 'Body',
      role: 'cookie-banner-body',
      content: body,
      fontSize: 13,
      fontWeight: 400,
      lineHeight: 1.5,
      fill: [{ type: 'solid', color: '#475569' }],
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
      fill: [{ type: 'solid', color: '#2563EB' }],
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
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
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
