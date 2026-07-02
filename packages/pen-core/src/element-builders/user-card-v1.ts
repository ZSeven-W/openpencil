import { coerceNonEmptyString } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface UserCardV1Params {
  /** Display name (e.g. "Sarah Lee"). */
  name: string;
  /** Optional secondary line (e.g. "Senior Engineer"). */
  role?: string;
  /** Optional avatar initial (1-2 chars). */
  initial?: string;
  /** Avatar diameter. Default 48. Clamped 32..96. */
  avatar_size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_user_card_v0.
   * - `'dark'`: dark-mode fills for name text, role text.
   * - `'system'`: emits `$color-*` ref strings.
   */
  theme?: V1Theme;
}

/**
 * Profile / contact card (v1) — theme-aware variant of buildUserCard.
 * Light mode is byte-equal to add_user_card_v0.
 *
 * Color slots:
 * - Name text fill: #0F172A → textPrimary
 * - Role text fill: #64748B → textMuted
 * - Avatar bg: #3B82F6 — brand accent (spec §3.4), hardcoded all modes
 * - Initial text fill: #FFFFFF — always white on colored avatar, hardcoded all modes
 */
export function buildUserCardV1(params: UserCardV1Params): ElementTree {
  const name = coerceNonEmptyString(params.name, 'User Name', 'buildUserCardV1', 'name');
  const size = Math.min(96, Math.max(32, Math.floor(params.avatar_size ?? 48)));
  const initialFontSize = Math.max(12, Math.round(size * 0.4));

  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);

  // Light mode: v0 literals for byte-parity
  const nameColor = theme === 'light' ? '#0F172A' : t.colors.textPrimary;
  const roleColor = theme === 'light' ? '#64748B' : t.colors.textMuted;

  const stackChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Name',
      role: 'user-card-name',
      content: name,
      fontSize: 15,
      fontWeight: 600,
      fill: [{ type: 'solid', color: nameColor }],
    },
  ];
  // Conditional emit: builders never invent optional content, and an
  // empty-content text node still consumes flex gap so it is not
  // layout-neutral with a "no role line" output. Callers wanting the
  // user-card-role node must pass a non-empty `role`.
  if (params.role) {
    stackChildren.push({
      type: 'text',
      name: 'Role',
      role: 'user-card-role',
      content: params.role,
      fontSize: 13,
      fontWeight: 400,
      fill: [{ type: 'solid', color: roleColor }],
    });
  }
  const avatarChildren: ElementTree[] = [];
  if (params.initial) {
    avatarChildren.push({
      type: 'text',
      name: 'Initial',
      role: 'user-card-initial',
      content: params.initial,
      fontSize: initialFontSize,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    });
  }
  return {
    type: 'frame',
    name: 'User Card',
    role: 'user-card',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    children: [
      {
        type: 'frame',
        name: 'Avatar',
        role: 'user-card-avatar',
        width: size,
        height: size,
        cornerRadius: size / 2,
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'center',
        fill: [{ type: 'solid', color: '#3B82F6' }],
        children: avatarChildren,
      },
      {
        type: 'frame',
        name: 'Text Stack',
        role: 'user-card-text',
        width: 'fit_content',
        height: 'fit_content',
        layout: 'vertical',
        gap: 2,
        children: stackChildren,
      },
    ],
  };
}
