import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ProfileHeaderV1Params {
  /** Display name. */
  name: string;
  /** Optional handle (e.g. "@sarah"). */
  handle?: string;
  /** Optional bio / role line. */
  bio?: string;
  /** Optional avatar initial (1-2 chars). */
  initial?: string;
  /** Avatar diameter. Default 96. Clamped 64..160. */
  avatar_size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_profile_header_v0.
   * - `'dark'`: name → textPrimary, handle → textMuted, bio → textBody;
   *   avatar bg stays brand color (#3B82F6); initial text stays white.
   * - `'system'`: emits `$color-*` refs for name/handle/bio fills;
   *   avatar bg and initial stay hardcoded (brand-invariant).
   */
  theme?: V1Theme;
}

/**
 * Large profile header (v1) — theme-aware variant of buildProfileHeader.
 * Light mode is byte-equal to add_profile_header_v0.
 *
 * Color mapping:
 *   name text    (#0F172A slate-950) → textPrimary
 *   handle text  (#64748B slate-500) → textMuted
 *   bio text     (#475569 slate-600) → textBody
 *   avatar bg    (#3B82F6 blue-500)  — brand-invariant, kept across themes
 *   initial text (#FFFFFF white)     — white-on-brand, invariant
 */
export function buildProfileHeaderV1(params: ProfileHeaderV1Params): ElementTree {
  const size = Math.min(160, Math.max(64, Math.floor(params.avatar_size ?? 96)));
  const initialFontSize = Math.max(20, Math.round(size * 0.4));

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const nameColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const handleColor = isLight ? '#64748B' : t.colors.textMuted;
  const bioColor = isLight ? '#475569' : t.colors.textBody;
  // Avatar bg and initial text are brand-invariant
  const avatarBg = '#3B82F6';
  const initialColor = '#FFFFFF';

  const avatarChildren: ElementTree[] = [];
  if (params.initial) {
    avatarChildren.push({
      type: 'text',
      name: 'Initial',
      role: 'profile-header-initial',
      content: params.initial,
      fontSize: initialFontSize,
      fontWeight: 600,
      fill: [{ type: 'solid', color: initialColor }],
    });
  }
  const stackChildren: ElementTree[] = [
    {
      type: 'frame',
      name: 'Avatar',
      role: 'profile-header-avatar',
      width: size,
      height: size,
      cornerRadius: size / 2,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: [{ type: 'solid', color: avatarBg }],
      children: avatarChildren,
    },
    {
      type: 'text',
      name: 'Name',
      role: 'profile-header-name',
      content: params.name,
      fontSize: 22,
      fontWeight: 700,
      fill: [{ type: 'solid', color: nameColor }],
    },
  ];
  if (params.handle) {
    stackChildren.push({
      type: 'text',
      name: 'Handle',
      role: 'profile-header-handle',
      content: params.handle,
      fontSize: 14,
      fontWeight: 400,
      fill: [{ type: 'solid', color: handleColor }],
    });
  }
  if (params.bio) {
    stackChildren.push({
      type: 'text',
      name: 'Bio',
      role: 'profile-header-bio',
      content: params.bio,
      fontSize: 14,
      fontWeight: 400,
      lineHeight: 1.5,
      fill: [{ type: 'solid', color: bioColor }],
      width: 'fill_container',
      textGrowth: 'fixed-width',
    });
  }
  return {
    type: 'frame',
    name: 'Profile Header',
    role: 'profile-header',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 12,
    padding: [24, 16],
    children: stackChildren,
  };
}
