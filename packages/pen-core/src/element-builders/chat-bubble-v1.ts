import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type ChatBubbleV1Side = 'left' | 'right';

export interface ChatBubbleV1Params {
  /** Message body text. Required. */
  message: string;
  /**
   * Which side the bubble aligns to.
   *   - `'left'` (default): from-someone-else.
   *   - `'right'`: from-self. Author suppressed. Accent-colored surface.
   */
  side?: ChatBubbleV1Side;
  /** Sender display name. ONLY rendered on `side: 'left'`. */
  author?: string;
  /** Relative time string (e.g. "2m", "Just now"). Rendered below the bubble. */
  timestamp?: string;
  /** Max bubble width in px. Default 280. Clamped 160..480. */
  max_width?: number;
  /**
   * Accent color for self-side bubbles (right). Default #2563EB.
   * Only used in light mode (v0 byte-parity). Dark/system modes use
   * the semantic accent token ($color-accent / dark-mode hex).
   */
  accent_color?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_chat_bubble_v0.
   * - `'dark'`: dark-mode fills using semantic palette tokens.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Chat bubble — theme-aware version of buildChatBubble.
 * Light mode is byte-equal to add_chat_bubble_v0.
 *
 * Left-side (from-others): surface2 bg, textPrimary text, textMuted author/timestamp.
 * Right-side (from-self): accent bg, white text (white kept across themes — text on
 * accent is always white). Author/timestamp: textMuted in dark/system.
 */
export function buildChatBubbleV1(params: ChatBubbleV1Params): ElementTree {
  const side: ChatBubbleV1Side = params.side ?? 'left';
  const maxWidth = Math.max(160, Math.min(480, Math.floor(params.max_width ?? 280)));
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';
  const isSelf = side === 'right';

  // Light mode: byte-parity with v0
  const accent = isLight ? (params.accent_color ?? '#2563EB') : t.colors.accent;
  const surfaceBg = isSelf ? accent : isLight ? '#F1F5F9' : t.colors.surface2;
  const textColor = isSelf ? '#FFFFFF' : isLight ? '#0F172A' : t.colors.textPrimary;
  const mutedColor = isLight ? '#64748B' : t.colors.textMuted;

  const alignItems: 'flex-start' | 'flex-end' = isSelf ? 'flex-end' : 'flex-start';

  const outerChildren: ElementTree[] = [];

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

  outerChildren.push({
    type: 'frame',
    name: 'Surface',
    role: 'chat-bubble-surface',
    width: maxWidth,
    height: 'fit_content',
    cornerRadius: 16,
    layout: 'vertical',
    padding: [10, 14],
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
