import type { ElementTree } from './helpers.js';

export interface NotificationRowParams {
  title: string;
  body?: string;
  /** Relative timestamp ("2h ago", "Now"). */
  timestamp?: string;
  /** Lucide icon slug for the leading affordance. Default "bell". */
  icon?: string;
  /** When true, renders a small red dot next to the title as "unread" marker. */
  unread?: boolean;
}

/**
 * Notification list row: leading icon + (title inline with timestamp
 * + optional unread dot) + optional body preview below. Distinct
 * from `add_list_row_v0` (which is title/subtitle + trailing icon
 * — no timestamp, no unread marker).
 *
 * Structure:
 *   frame(fill_container, horizontal, padding=[12,16], gap=12, alignItems=start)
 *     ├ icon_font(20×20, role='notification-icon')
 *     └ frame(fill_container, vertical, gap=2)
 *          ├ frame(horizontal, justifyContent=space_between, alignItems=center)
 *          │   ├ frame(horizontal, gap=6, alignItems=center)
 *          │   │   ├ text(title, 14/600)
 *          │   │   └ optional frame(8×8 dot, red fill, role='notification-unread-dot')
 *          │   └ optional text(timestamp, 12/400 slate-400)
 *          └ optional text(body, 13/400 slate-600, lineHeight=1.4)
 */
export function buildNotificationRow(params: NotificationRowParams): ElementTree {
  const icon = params.icon ?? 'bell';

  const titleRowChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'notification-title',
      content: params.title,
      fontSize: 14,
      fontWeight: 600,
    },
  ];
  if (params.unread) {
    titleRowChildren.push({
      type: 'frame',
      name: 'Unread Dot',
      role: 'notification-unread-dot',
      width: 8,
      height: 8,
      cornerRadius: 4,
      fill: [{ type: 'solid', color: '#EF4444' }],
    });
  }

  const headerRowChildren: ElementTree[] = [
    {
      type: 'frame',
      name: 'Title Row',
      role: 'notification-title-row',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      gap: 6,
      children: titleRowChildren,
    },
  ];
  if (params.timestamp) {
    headerRowChildren.push({
      type: 'text',
      name: 'Timestamp',
      role: 'notification-timestamp',
      content: params.timestamp,
      fontSize: 12,
      fontWeight: 400,
      fill: [{ type: 'solid', color: '#94A3B8' }],
    });
  }

  const bodyColChildren: ElementTree[] = [
    {
      type: 'frame',
      name: 'Header Row',
      role: 'notification-header',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'space_between',
      gap: 8,
      children: headerRowChildren,
    },
  ];
  if (params.body) {
    bodyColChildren.push({
      type: 'text',
      name: 'Body Preview',
      role: 'notification-body',
      content: params.body,
      fontSize: 13,
      fontWeight: 400,
      lineHeight: 1.4,
      fill: [{ type: 'solid', color: '#475569' }],
    });
  }

  return {
    type: 'frame',
    name: 'Notification Row',
    role: 'notification-row',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'start',
    padding: [12, 16],
    gap: 12,
    children: [
      {
        type: 'icon_font',
        name: 'Leading Icon',
        role: 'notification-icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 20,
        height: 20,
      },
      {
        type: 'frame',
        name: 'Body Column',
        role: 'notification-body-column',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 2,
        children: bodyColChildren,
      },
    ],
  };
}
