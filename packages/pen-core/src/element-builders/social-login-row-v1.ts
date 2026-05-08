import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface SocialLoginV1Provider {
  /**
   * Provider name. Displayed inline as "Continue with {name}".
   * Known names ('google', 'apple', 'microsoft', 'github',
   * 'facebook', 'twitter', 'x', 'linkedin') auto-resolve to the
   * matching lucide icon. Override via explicit `icon`.
   */
  name: string;
  /**
   * Optional lucide icon override. Takes precedence over the
   * known-name mapping.
   */
  icon?: string;
}

export interface SocialLoginRowV1Params {
  /** Array of providers to render. 2-4 recommended; clamped to 6 max. */
  providers: SocialLoginV1Provider[];
  /**
   * Layout orientation. Default 'vertical' (stacked full-width buttons).
   * 'horizontal' renders icon-only pills side-by-side.
   */
  orientation?: 'vertical' | 'horizontal';
  /** Button width in px. Default 320. Min 200. Ignored on horizontal orientation. */
  width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_social_login_row_v0.
   * - `'dark'`: button bg → surface; border → border; icon fill → textMuted;
   *   label → textPrimary.
   * - `'system'`: $color-* refs for all color slots.
   */
  theme?: V1Theme;
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
 * Social-auth provider button row (v1) — theme-aware variant of buildSocialLoginRow.
 * Light mode is byte-equal to add_social_login_row_v0.
 *
 * Color mapping:
 *   button bg   (#FFFFFF white)          → surface token
 *   border      (#E2E8F0 slate-200)      → border token
 *   icon fill   (#334155 slate-700)      → textMuted token
 *   label text  (#0F172A slate-900)      → textPrimary token
 */
export function buildSocialLoginRowV1(params: SocialLoginRowV1Params): ElementTree {
  const raw = Array.isArray(params.providers) ? params.providers : [];
  if (raw.length === 0) {
    throw new Error('buildSocialLoginRowV1: providers array must not be empty');
  }
  const providers = raw.slice(0, 6);
  const orientation = params.orientation ?? 'vertical';
  const isVertical = orientation === 'vertical';
  const width = Math.max(200, Math.floor(params.width ?? 320));
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Button bg: light → white (#FFFFFF), dark/system → surface
  const buttonBg = isLight ? '#FFFFFF' : t.colors.surface;
  // Border: light → slate-200 (#E2E8F0), dark/system → border
  const borderColor = isLight ? '#E2E8F0' : t.colors.border;
  // Icon fill: light → slate-700 (#334155), dark/system → textMuted
  const iconFill = isLight ? '#334155' : t.colors.textMuted;
  // Label: light → slate-900 (#0F172A), dark/system → textPrimary
  const labelColor = isLight ? '#0F172A' : t.colors.textPrimary;

  const buttons: ElementTree[] = providers.map((provider) => {
    const lowerName = provider.name.toLowerCase();
    const icon = provider.icon ?? KNOWN_ICONS[lowerName] ?? 'log-in';
    const label = `Continue with ${provider.name}`;
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
      fill: [{ type: 'solid', color: iconFill }],
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
        padding: [0, 16],
        fill: [{ type: 'solid', color: buttonBg }],
        stroke: { thickness: 1, fill: [{ type: 'solid', color: borderColor }] },
        children: [
          iconNode,
          {
            type: 'text',
            name: 'Label',
            role: 'social-login-button-label',
            content: prettyLabel,
            fontSize: 14,
            fontWeight: 500,
            fill: [{ type: 'solid', color: labelColor }],
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
      fill: [{ type: 'solid', color: buttonBg }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: borderColor }] },
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
