import type { ElementTree } from './helpers.js';

export interface ProfileHeaderParams {
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
}

/**
 * Large profile header — centered avatar + display name + optional
 * handle / bio block. Distinct from `add_user_card_v0` (compact
 * horizontal row for lists / tiles) and `add_avatar_v0` (just the
 * disk). Use for profile pages, "About me" hero blocks, settings
 * / account screens.
 */
export function buildProfileHeader(params: ProfileHeaderParams): ElementTree {
  const size = Math.min(160, Math.max(64, Math.floor(params.avatar_size ?? 96)));
  const initialFontSize = Math.max(20, Math.round(size * 0.4));
  const avatarChildren: ElementTree[] = [];
  if (params.initial) {
    avatarChildren.push({
      type: 'text',
      name: 'Initial',
      role: 'profile-header-initial',
      content: params.initial,
      fontSize: initialFontSize,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
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
      fill: [{ type: 'solid', color: '#3B82F6' }],
      children: avatarChildren,
    },
    {
      type: 'text',
      name: 'Name',
      role: 'profile-header-name',
      content: params.name,
      fontSize: 22,
      fontWeight: 700,
      fill: [{ type: 'solid', color: '#0F172A' }],
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
      fill: [{ type: 'solid', color: '#64748B' }],
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
      fill: [{ type: 'solid', color: '#475569' }],
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
