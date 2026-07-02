import type { ElementTree } from './helpers.js';

export interface InboxMessageParams {
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
}

/**
 * Inbox / email list row — sender + (subject inline with timestamp)
 * stacked over an optional preview line. Distinct from
 * `add_notification_row_v0` (single title + body, no separate sender
 * field, smaller leading icon) and `add_list_row_v0` (title/subtitle
 * + chevron, no timestamp + unread affordance).
 */
export function buildInboxMessage(params: InboxMessageParams): ElementTree {
  const isUnread = params.unread === true;
  const senderRowChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'From',
      role: 'inbox-message-from',
      content: params.from,
      fontSize: 14,
      fontWeight: isUnread ? 700 : 500,
      fill: [{ type: 'solid', color: '#0F172A' }],
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
      fill: [{ type: 'solid', color: '#94A3B8' }],
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
      fill: [{ type: 'solid', color: '#0F172A' }],
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
      fill: [{ type: 'solid', color: '#64748B' }],
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
      fill: [{ type: 'solid', color: '#2563EB' }],
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
