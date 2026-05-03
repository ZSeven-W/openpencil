import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface CommentV1Params {
  /** Author display name (shown in bold inline with timestamp). */
  author: string;
  /** Relative timestamp string ("2 hours ago", "Just now", "3d"). */
  timestamp?: string;
  /** Body text. Supports multi-line (rendered as a single text node). */
  body: string;
  /**
   * Avatar initial (1-2 chars). When absent, the avatar is rendered
   * as a blank circle (placeholder for a user photo).
   */
  avatar_initial?: string;
  /** Avatar diameter in px. Default 40. Min 24. */
  avatar_size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_comment_v0.
   * - `'dark'`: dark fills for avatar bg, initial text, timestamp.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Comment row — theme-aware version of buildComment.
 * Light mode is byte-equal to add_comment_v0.
 *
 * Color mapping:
 *   avatar bg (#E2E8F0)    → surface2 (light gray placeholder)
 *   initial text (#475569) → textBody
 *   timestamp (#64748B)    → textMuted
 *
 * Author text and body text have no explicit fill in v0; kept unfilled
 * in all modes for byte-parity.
 */
export function buildCommentV1(params: CommentV1Params): ElementTree {
  const avatarSize = Math.max(24, Math.floor(params.avatar_size ?? 40));
  const avatarRadius = avatarSize / 2;
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  const avatarBg = isLight ? '#E2E8F0' : t.colors.surface2;
  const initialColor = isLight ? '#475569' : t.colors.textBody;
  const timestampColor = isLight ? '#64748B' : t.colors.textMuted;

  const avatarChildren: ElementTree[] = [];
  if (params.avatar_initial) {
    avatarChildren.push({
      type: 'text',
      name: 'Initial',
      role: 'avatar-initial',
      content: params.avatar_initial.slice(0, 2).toUpperCase(),
      fontSize: Math.max(10, Math.round(avatarSize * 0.4)),
      fontWeight: 600,
      fill: [{ type: 'solid', color: initialColor }],
    });
  }

  const headerChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Author',
      role: 'comment-author',
      content: params.author,
      fontSize: 14,
      fontWeight: 600,
    },
  ];
  if (params.timestamp) {
    headerChildren.push({
      type: 'text',
      name: 'Timestamp',
      role: 'comment-timestamp',
      content: params.timestamp,
      fontSize: 12,
      fontWeight: 400,
      fill: [{ type: 'solid', color: timestampColor }],
    });
  }

  return {
    type: 'frame',
    name: 'Comment',
    role: 'comment',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'start',
    gap: 12,
    children: [
      {
        type: 'frame',
        name: 'Avatar',
        role: 'comment-avatar',
        width: avatarSize,
        height: avatarSize,
        cornerRadius: avatarRadius,
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'center',
        fill: [{ type: 'solid', color: avatarBg }],
        children: avatarChildren,
      },
      {
        type: 'frame',
        name: 'Body Column',
        role: 'comment-body-column',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 4,
        children: [
          {
            type: 'frame',
            name: 'Header',
            role: 'comment-header',
            width: 'fit_content',
            height: 'fit_content',
            layout: 'horizontal',
            alignItems: 'baseline',
            gap: 6,
            children: headerChildren,
          },
          {
            type: 'text',
            name: 'Body',
            role: 'comment-body',
            content: params.body,
            fontSize: 14,
            fontWeight: 400,
            lineHeight: 1.5,
          },
        ],
      },
    ],
  };
}
