import type { ElementTree } from './helpers.js';

export interface UserCardParams {
  /** Display name (e.g. "Sarah Lee"). */
  name: string;
  /** Optional secondary line (e.g. "Senior Engineer", "@sarah", "PM"). */
  role?: string;
  /** Optional avatar initial (1-2 chars). */
  initial?: string;
  /** Avatar diameter. Default 48. Clamped 32..96. */
  avatar_size?: number;
}

/**
 * Profile / contact card — circular avatar + (name + optional role) text
 * stack. Distinct from `add_comment_v0` (which carries a message body
 * + timestamp) and `add_avatar_v0` (just the disk).
 *
 * Use for "team member tile", "contact card", "people picker row",
 * "user profile mini-card".
 */
export function buildUserCard(params: UserCardParams): ElementTree {
  const size = Math.min(96, Math.max(32, Math.floor(params.avatar_size ?? 48)));
  const initialFontSize = Math.max(12, Math.round(size * 0.4));
  const stackChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Name',
      role: 'user-card-name',
      content: params.name,
      fontSize: 15,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#0F172A' }],
    },
  ];
  if (params.role) {
    stackChildren.push({
      type: 'text',
      name: 'Role',
      role: 'user-card-role',
      content: params.role,
      fontSize: 13,
      fontWeight: 400,
      fill: [{ type: 'solid', color: '#64748B' }],
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
