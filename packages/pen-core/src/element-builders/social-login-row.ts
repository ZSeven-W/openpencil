import type { ElementTree } from './helpers.js';

export interface SocialLoginProvider {
  /**
   * Provider name. Displayed inline as "Continue with {name}".
   * Known names ('google', 'apple', 'microsoft', 'github',
   * 'facebook', 'twitter', 'x', 'linkedin') auto-resolve to the
   * matching lucide icon. Override via explicit `icon`.
   */
  name: string;
  /**
   * Optional lucide icon override. Takes precedence over the
   * known-name mapping. Use this for providers we don't have a
   * default mapping for (e.g. "Okta", "SAML SSO").
   */
  icon?: string;
}

export interface SocialLoginRowParams {
  /** Array of providers to render. 2-4 recommended; clamped to 6 max. */
  providers: SocialLoginProvider[];
  /**
   * Layout orientation. Default 'vertical' (stacked full-width
   * buttons) — the pattern on most modern mobile login screens.
   * 'horizontal' renders icon-only pills side-by-side, the more
   * compact "also sign in with..." pattern.
   */
  orientation?: 'vertical' | 'horizontal';
  /** Button width in px. Default 320. Min 200. Ignored on horizontal orientation. */
  width?: number;
}

const KNOWN_ICONS: Record<string, string> = {
  google: 'chrome',
  apple: 'apple',
  microsoft: 'monitor',
  github: 'github',
  gitlab: 'git-branch',
  facebook: 'facebook',
  twitter: 'twitter',
  x: 'twitter',
  linkedin: 'linkedin',
  discord: 'message-circle',
  slack: 'hash',
  email: 'mail',
  phone: 'smartphone',
};

/**
 * Social-auth provider button row — the "Continue with Google /
 * Apple / Microsoft" pattern from every login / signup screen.
 *
 * Structure (vertical, default):
 *   frame(width, fit_content, layout=vertical, gap=10,
 *         role='social-login-row')
 *     └ frame(fill_container, 48px high, cornerRadius=12,
 *             bg=#FFFFFF, stroke=#E2E8F0, layout=horizontal,
 *             alignItems=center, gap=12, padding=[0,16],
 *             role='social-login-button')
 *         ├ icon_font(provider.icon, 20×20, slate-700)
 *         └ text("Continue with {Name}", 14/500, slate-900)
 *
 * Structure (horizontal): same buttons but `layout='horizontal'`
 * wrapper, buttons collapse to 48×48 square with only the icon
 * (no label). The compact variant for "quick alt login" rows.
 *
 * Why it's a tool: enough variation in provider order / icon
 * choice / orientation that models hand-crafting this via
 * batch_design frequently invert the icon+label order or miss the
 * button-height constraint. Narrow tool locks the shape.
 */
export function buildSocialLoginRow(params: SocialLoginRowParams): ElementTree {
  const raw = Array.isArray(params.providers) ? params.providers : [];
  if (raw.length === 0) {
    throw new Error('buildSocialLoginRow: providers array must not be empty');
  }
  const providers = raw.slice(0, 6);
  const orientation = params.orientation ?? 'vertical';
  const isVertical = orientation === 'vertical';
  const width = Math.max(200, Math.floor(params.width ?? 320));

  const buttons: ElementTree[] = providers.map((provider) => {
    const lowerName = provider.name.toLowerCase();
    const icon = provider.icon ?? KNOWN_ICONS[lowerName] ?? 'log-in';
    const label = `Continue with ${provider.name}`;
    // Capitalize properly: "google" → "Google". If already capitalized,
    // keep the caller's casing.
    const prettyLabel =
      provider.name.charAt(0) === provider.name.charAt(0).toUpperCase()
        ? label
        : `Continue with ${provider.name.charAt(0).toUpperCase() + provider.name.slice(1)}`;

    const iconNode: ElementTree = {
      type: 'icon_font',
      name: `${provider.name} Icon`,
      role: 'social-login-button-icon',
      iconFontName: icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
      fill: [{ type: 'solid', color: '#334155' }],
    };

    if (isVertical) {
      return {
        type: 'frame',
        name: `${provider.name} Button`,
        role: 'social-login-button',
        width: 'fill_container',
        height: 48,
        cornerRadius: 12,
        layout: 'horizontal',
        alignItems: 'center',
        gap: 12,
        paddingLeft: 16,
        paddingRight: 16,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
        children: [
          iconNode,
          {
            type: 'text',
            name: 'Label',
            role: 'social-login-button-label',
            content: prettyLabel,
            fontSize: 14,
            fontWeight: 500,
            fill: [{ type: 'solid', color: '#0F172A' }],
          },
        ],
      };
    }
    // Horizontal: icon-only square pill
    return {
      type: 'frame',
      name: `${provider.name} Button`,
      role: 'social-login-button-compact',
      width: 48,
      height: 48,
      cornerRadius: 12,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
      children: [iconNode],
    };
  });

  return {
    type: 'frame',
    name: 'Social Login Row',
    role: 'social-login-row',
    width: isVertical ? width : 'fit_content',
    height: 'fit_content',
    layout: isVertical ? 'vertical' : 'horizontal',
    alignItems: 'center',
    gap: 10,
    children: buttons,
  };
}
