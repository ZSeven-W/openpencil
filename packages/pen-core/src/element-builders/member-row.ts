import type { ElementTree } from './helpers.js';

export type MemberRowTrailing =
  | { kind: 'role_badge'; value: string }
  | { kind: 'menu' }
  | { kind: 'status_dot'; tone?: 'online' | 'busy' | 'away' | 'offline' };

export interface MemberRowParams {
  /** Display name (e.g. "Sarah Lee"). */
  name: string;
  /** Optional secondary line (e.g. "sarah@acme.com", "Designer", "Owner • Designer"). */
  subtitle?: string;
  /** Optional avatar initial (1-2 chars; falls back to name's first char). */
  initial?: string;
  /** Avatar background hex. Default '#3B82F6'. */
  avatar_color?: string;
  /** Trailing slot. Default: none (just name + subtitle). */
  trailing?: MemberRowTrailing;
}

const NAME_FG = '#0F172A';
const SUBTITLE_FG = '#64748B';
const BADGE_BG = '#F1F5F9';
const BADGE_FG = '#334155';
const MENU_FG = '#94A3B8';

type StatusTone = 'online' | 'busy' | 'away' | 'offline';

const STATUS_TONE_FILL: Record<StatusTone, string> = {
  online: '#10B981',
  busy: '#EF4444',
  away: '#F59E0B',
  offline: '#94A3B8',
};

function buildTrailing(t: MemberRowTrailing): ElementTree {
  if (t.kind === 'role_badge') {
    return {
      type: 'frame',
      name: 'Role Badge',
      role: 'member-row-badge',
      width: 'fit_content',
      height: 'fit_content',
      cornerRadius: 4,
      fill: [{ type: 'solid', color: BADGE_BG }],
      padding: [3, 8],
      children: [
        {
          type: 'text',
          name: 'Role',
          role: 'member-row-badge-text',
          content: t.value,
          fontSize: 12,
          fontWeight: 500,
          fill: [{ type: 'solid', color: BADGE_FG }],
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
      fill: [{ type: 'solid', color: MENU_FG }],
    };
  }
  const tone = t.tone ?? 'online';
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
 * Team / member list row — circular avatar + (name over optional
 * subtitle) + optional trailing (role badge, kebab menu, or status
 * dot). Distinct from `add_user_card_v0` (compact tile, fit_content,
 * no trailing slot — used in lobbies / chip-style lists) and
 * `add_list_row_v0` (no avatar slot, generic icon-led row).
 *
 * Structure:
 *   frame(horizontal, fill_container, fit_content, padding=[12,16],
 *         gap=12, alignItems=center, role='member-row')
 *     ├ frame(40×40 avatar, cornerRadius=20, flex-centered initial)
 *     ├ frame(vertical, fill_container, gap=2, role='member-row-text')
 *     │   ├ text(name, 15/500)
 *     │   └ text(subtitle, 13/400, muted) (if subtitle)
 *     └ <trailing>  (role_badge | menu | status_dot)
 */
export function buildMemberRow(params: MemberRowParams): ElementTree {
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
        fill: [{ type: 'solid', color: '#FFFFFF' }],
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
      fill: [{ type: 'solid', color: NAME_FG }],
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
      fill: [{ type: 'solid', color: SUBTITLE_FG }],
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
    rowChildren.push(buildTrailing(params.trailing));
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
