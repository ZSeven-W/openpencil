import type { ElementTree } from './helpers.js';

export type ChatBubbleSide = 'left' | 'right';

export interface ChatBubbleParams {
  /** Message body text. Required. */
  message: string;
  /**
   * Which side the bubble aligns to.
   *   - `'left'` (default): from-someone-else. Slate-100 bg, slate-900 text.
   *     Optional author shown above. Justified to flex-start.
   *   - `'right'`: from-self. Accent-blue bg, white text. Author suppressed
   *     (the self-bubble never carries "You:"). Justified to flex-end.
   */
  side?: ChatBubbleSide;
  /** Sender display name. ONLY rendered on `side: 'left'`. */
  author?: string;
  /** Relative time string (e.g. "2m", "Just now"). Rendered below the bubble. */
  timestamp?: string;
  /** Max bubble width in px. Default 280. Clamped 160..480. */
  max_width?: number;
  /** Accent color for self-side bubbles. Default #2563EB. */
  accent_color?: string;
}

/**
 * Chat bubble — message unit for chat / messaging / customer-
 * support UIs. Left-side bubbles (from-others) carry the classic
 * "author above, muted gray bubble below" pattern; right-side
 * bubbles (from-self) are accent-colored and skip the author.
 *
 * Structure (left side):
 *   frame(vertical, gap=4, role='chat-bubble-left',
 *         alignItems=flex-start, fit_content)
 *     ├ text(author, 12/500 muted, role='chat-bubble-author')        ← only if author given
 *     ├ frame(cornerRadius=16, bg=slate-100, padding=[10,14],
 *             max_width, role='chat-bubble-surface')
 *     │   └ text(message, 14/400, wrapped, slate-900)
 *     └ text(timestamp, 11/400 muted, role='chat-bubble-timestamp')  ← only if timestamp given
 *
 * Structure (right side): same shape but alignItems=flex-end, no
 * author, accent-color fill on the surface, white text.
 *
 * `max_width` lives on the inner bubble frame so the message wraps
 * naturally without the outer column stretching to parent width.
 */
export function buildChatBubble(params: ChatBubbleParams): ElementTree {
  const side: ChatBubbleSide = params.side ?? 'left';
  const maxWidth = Math.max(160, Math.min(480, Math.floor(params.max_width ?? 280)));
  const accent = params.accent_color ?? '#2563EB';
  const isSelf = side === 'right';

  const surfaceBg = isSelf ? accent : '#F1F5F9';
  const textColor = isSelf ? '#FFFFFF' : '#0F172A';
  const mutedColor = '#64748B';
  const alignItems: 'flex-start' | 'flex-end' = isSelf ? 'flex-end' : 'flex-start';

  const outerChildren: ElementTree[] = [];

  // Author on left side only.
  if (!isSelf && params.author) {
    outerChildren.push({
      type: 'text',
      name: 'Author',
      role: 'chat-bubble-author',
      content: params.author,
      fontSize: 12,
      fontWeight: 500,
      fill: [{ type: 'solid', color: mutedColor }],
    });
  }

  // The bubble surface. `max_width` becomes the bubble's fixed
  // width — pen-core has no native max-width primitive, so fixed-
  // width is the pragmatic approximation. Very short messages get
  // extra padding on one side, which matches every real-world
  // chat client (iMessage / WhatsApp / Slack all do this).
  // `textGrowth: 'fixed-width'` on the text child ensures wrapping.
  outerChildren.push({
    type: 'frame',
    name: 'Surface',
    role: 'chat-bubble-surface',
    width: maxWidth,
    height: 'fit_content',
    cornerRadius: 16,
    layout: 'vertical',
    paddingTop: 10,
    paddingBottom: 10,
    paddingLeft: 14,
    paddingRight: 14,
    fill: [{ type: 'solid', color: surfaceBg }],
    children: [
      {
        type: 'text',
        name: 'Message',
        role: 'chat-bubble-message',
        content: params.message,
        fontSize: 14,
        fontWeight: 400,
        lineHeight: 1.4,
        width: 'fill_container',
        textGrowth: 'fixed-width',
        fill: [{ type: 'solid', color: textColor }],
      },
    ],
  });

  // Timestamp below bubble, both sides.
  if (params.timestamp) {
    outerChildren.push({
      type: 'text',
      name: 'Timestamp',
      role: 'chat-bubble-timestamp',
      content: params.timestamp,
      fontSize: 11,
      fontWeight: 400,
      fill: [{ type: 'solid', color: mutedColor }],
    });
  }

  return {
    type: 'frame',
    name: 'Chat Bubble',
    role: isSelf ? 'chat-bubble-right' : 'chat-bubble-left',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    alignItems,
    gap: 4,
    children: outerChildren,
  };
}
