import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type MemberRowV1Trailing =
  | { kind: 'role_badge'; value: string }
  | { kind: 'menu' }
  | { kind: 'status_dot'; tone?: 'online' | 'busy' | 'away' | 'offline' };

export interface MemberRowV1Params {
  /** Display name (e.g. "Sarah Lee"). */
  name: string;
  /** Optional secondary line (e.g. "sarah@acme.com", "Designer", "Owner • Designer"). */
  subtitle?: string;
  /** Optional avatar initial (1-2 chars; falls back to name's first char). */
  initial?: string;
  /** Avatar background hex. Default '#3B82F6'. */
  avatar_color?: string;
  /** Trailing slot. Default: none (just name + subtitle). */
  trailing?: MemberRowV1Trailing;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_member_row_v0.
   * - `'dark'`: dark-mode hex fills.
   * - `'system'`: emits `$color-*` ref strings for all fill fields.
   */
  theme?: V1Theme;
}

// Builder-private constants (no corresponding palette token)
const KNOB_WHITE = '#FFFFFF';

type StatusTone = 'online' | 'busy' | 'away' | 'offline';
const VALID_STATUS_TONES = new Set<string>(['online', 'busy', 'away', 'offline']);

// Status dot colors are semantic/fixed — not theme-dependent
const STATUS_TONE_FILL: Record<StatusTone, string> = {
  online: '#10B981',
  busy: '#EF4444',
  away: '#F59E0B',
  offline: '#94A3B8',
};

function buildTrailing(t: MemberRowV1Trailing, theme: V1Theme): ElementTree {
  const colors = resolveTheme(theme).colors;

  if (t.kind === 'role_badge') {
    return {
      type: 'frame',
      name: 'Role Badge',
      role: 'member-row-badge',
      width: 'fit_content',
      height: 'fit_content',
      cornerRadius: 4,
      fill: [{ type: 'solid', color: colors.surface2 }],
      padding: [3, 8],
      children: [
        {
          type: 'text',
          name: 'Role',
          role: 'member-row-badge-text',
          content: t.value,
          fontSize: 12,
          fontWeight: 500,
          fill: [{ type: 'solid', color: colors.textBody }],
        },
      ],
    };
  }
  if (t.kind === 'menu') {
    return {
      type: 'icon_font',
      name: 'Menu',
      role: 'member-row-menu',
      iconFontName: 'more-vertical',
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
      fill: [{ type: 'solid', color: colors.textSubtle }],
    };
  }
  const requestedTone = (t.tone ?? 'online') as string;
  if (!VALID_STATUS_TONES.has(requestedTone)) {
    throw new Error(
      `add_member_row_v1: invalid trailing.tone "${requestedTone}"; expected one of: online, busy, away, offline`,
    );
  }
  const tone = requestedTone as StatusTone;
  return {
    type: 'frame',
    name: 'Status Dot',
    role: 'member-row-status',
    width: 10,
    height: 10,
    cornerRadius: 5,
    fill: [{ type: 'solid', color: STATUS_TONE_FILL[tone] }],
    children: [],
  };
}

/**
 * Team / member list row — theme-aware version of buildMemberRow.
 * Light mode is byte-equal to add_member_row_v0.
 * Dark/system modes use resolveTheme() for surface2, textBody, textMuted, textSubtle fills.
 */
export function buildMemberRowV1(params: MemberRowV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const colors = resolveTheme(theme).colors;

  const size = 40;
  const avatarColor = params.avatar_color ?? '#3B82F6';
  const initialChar = (params.initial ?? params.name.charAt(0) ?? '?').slice(0, 2).toUpperCase();

  const avatar: ElementTree = {
    type: 'frame',
    name: 'Avatar',
    role: 'member-row-avatar',
    width: size,
    height: size,
    cornerRadius: size / 2,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    fill: [{ type: 'solid', color: avatarColor }],
    children: [
      {
        type: 'text',
        name: 'Initial',
        role: 'member-row-avatar-initial',
        content: initialChar,
        fontSize: 15,
        fontWeight: 600,
        fill: [{ type: 'solid', color: KNOB_WHITE }],
      },
    ],
  };

  const textStackChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Name',
      role: 'member-row-name',
      content: params.name,
      fontSize: 15,
      fontWeight: 500,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: colors.textPrimary }],
    },
  ];
  if (params.subtitle) {
    textStackChildren.push({
      type: 'text',
      name: 'Subtitle',
      role: 'member-row-subtitle',
      content: params.subtitle,
      fontSize: 13,
      fontWeight: 400,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: colors.textMuted }],
    });
  }

  const rowChildren: ElementTree[] = [
    avatar,
    {
      type: 'frame',
      name: 'Text Stack',
      role: 'member-row-text',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      gap: 2,
      children: textStackChildren,
    },
  ];

  if (params.trailing) {
    rowChildren.push(buildTrailing(params.trailing, theme));
  }

  return {
    type: 'frame',
    name: 'Member Row',
    role: 'member-row',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    padding: [12, 16],
    children: rowChildren,
  };
}
