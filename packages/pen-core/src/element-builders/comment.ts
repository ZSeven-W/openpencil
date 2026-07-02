import type { ElementTree } from './helpers.js';

export interface CommentParams {
  /** Author display name (shown in bold inline with timestamp). */
  author: string;
  /** Relative timestamp string ("2 hours ago", "Just now", "3d"). */
  timestamp?: string;
  /** Body text. Supports multi-line (rendered as a single text node). */
  body: string;
  /**
   * Avatar initial (1-2 chars). When absent, the avatar is rendered
   * as a blank gray circle (placeholder for a user photo).
   */
  avatar_initial?: string;
  /** Avatar diameter in px. Default 40. Min 24. */
  avatar_size?: number;
}

/**
 * Comment row: circular avatar + (author + timestamp inline header)
 * + body text below. Typical social / UGC / feedback list unit.
 *
 * Structure:
 *   frame(fill_container, horizontal, gap=12, alignItems=flex-start)
 *     └ frame(avatar_size², cornerRadius=half, fill=slate-200)
 *          [optional text(initial) centered]
 *     └ frame(fill_container, vertical, gap=4)
 *          ├ frame(horizontal, gap=6, alignItems=baseline)
 *          │   ├ text(author, 14/600)
 *          │   └ optional text(timestamp, 12/400, slate-500)
 *          └ text(body, 14/400, lineHeight=1.5)
 *
 * Does NOT handle threaded replies / like count / action menu —
 * those are separate concerns. Compose via batch_design if needed.
 * Per the "应拆尽拆" principle, this tool stops at the single-comment
 * unit; multiple comments are emitted by calling the tool N times
 * inside a vertical parent.
 */
export function buildComment(params: CommentParams): ElementTree {
  const avatarSize = Math.max(24, Math.floor(params.avatar_size ?? 40));
  const avatarRadius = avatarSize / 2;

  const avatarChildren: ElementTree[] = [];
  if (params.avatar_initial) {
    avatarChildren.push({
      type: 'text',
      name: 'Initial',
      role: 'avatar-initial',
      content: params.avatar_initial.slice(0, 2).toUpperCase(),
      fontSize: Math.max(10, Math.round(avatarSize * 0.4)),
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#475569' }],
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
      fill: [{ type: 'solid', color: '#64748B' }],
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
        fill: [{ type: 'solid', color: '#E2E8F0' }],
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
