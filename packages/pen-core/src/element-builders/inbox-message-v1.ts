import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface InboxMessageV1Params {
  /** Sender name (e.g. "Stripe", "Sarah Lee"). */
  from: string;
  /** Email subject line. */
  subject: string;
  /** Optional body preview (single line, ellipsis at render). */
  preview?: string;
  /** Time / date label (e.g. "10:42 AM", "Mar 14"). */
  timestamp?: string;
  /** When true, renders bolder typography + an unread indicator dot. */
  unread?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_inbox_message_v0.
   * - `'dark'`: primary → textPrimary, timestamp → textSubtle,
   *             preview → textMuted, unread dot → accent.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Inbox / email list row — theme-aware variant of buildInboxMessage.
 * Light mode is byte-equal to add_inbox_message_v0.
 *
 * Color mapping:
 *   from / subject text (#0F172A slate-950) → textPrimary
 *   timestamp           (#94A3B8 slate-400) → textSubtle
 *   preview             (#64748B slate-500) → textMuted
 *   unread dot bg       (#2563EB blue-600)  → accent
 */
export function buildInboxMessageV1(params: InboxMessageV1Params): ElementTree {
  const isUnread = params.unread === true;
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const primaryColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const timestampColor = isLight ? '#94A3B8' : t.colors.textSubtle;
  const previewColor = isLight ? '#64748B' : t.colors.textMuted;
  const dotColor = isLight ? '#2563EB' : t.colors.accent;

  const senderRowChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'From',
      role: 'inbox-message-from',
      content: params.from,
      fontSize: 14,
      fontWeight: isUnread ? 700 : 500,
      fill: [{ type: 'solid', color: primaryColor }],
    },
  ];
  if (params.timestamp) {
    senderRowChildren.push({
      type: 'text',
      name: 'Timestamp',
      role: 'inbox-message-timestamp',
      content: params.timestamp,
      fontSize: 12,
      fontWeight: 400,
      fill: [{ type: 'solid', color: timestampColor }],
    });
  }
  const stackChildren: ElementTree[] = [
    {
      type: 'frame',
      name: 'Sender Row',
      role: 'inbox-message-sender-row',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'space_between',
      gap: 8,
      children: senderRowChildren,
    },
    {
      type: 'text',
      name: 'Subject',
      role: 'inbox-message-subject',
      content: params.subject,
      fontSize: 14,
      fontWeight: isUnread ? 600 : 400,
      fill: [{ type: 'solid', color: primaryColor }],
      width: 'fill_container',
      textGrowth: 'fixed-width',
    },
  ];
  if (params.preview) {
    stackChildren.push({
      type: 'text',
      name: 'Preview',
      role: 'inbox-message-preview',
      content: params.preview,
      fontSize: 13,
      fontWeight: 400,
      fill: [{ type: 'solid', color: previewColor }],
      width: 'fill_container',
      textGrowth: 'fixed-width',
    });
  }
  const rowChildren: ElementTree[] = [];
  if (isUnread) {
    rowChildren.push({
      type: 'frame',
      name: 'Unread Dot',
      role: 'inbox-message-unread',
      width: 8,
      height: 8,
      cornerRadius: 4,
      fill: [{ type: 'solid', color: dotColor }],
      children: [],
    });
  }
  rowChildren.push({
    type: 'frame',
    name: 'Stack',
    role: 'inbox-message-stack',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 2,
    children: stackChildren,
  });
  return {
    type: 'frame',
    name: 'Inbox Message',
    role: 'inbox-message',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'start',
    gap: 10,
    padding: [12, 16],
    children: rowChildren,
  };
}
