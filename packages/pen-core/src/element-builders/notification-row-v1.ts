import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface NotificationRowV1Params {
  title: string;
  body?: string;
  /** Relative timestamp ("2h ago", "Now"). */
  timestamp?: string;
  /** Lucide icon slug for the leading affordance. Default "bell". */
  icon?: string;
  /** When true, renders a small red dot next to the title as "unread" marker. */
  unread?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_notification_row_v0.
   * - `'dark'`: timestamp → textSubtle, body → textBody, unread dot → destructive.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Notification list row (v1) — theme-aware variant of buildNotificationRow.
 * Light mode is byte-equal to add_notification_row_v0.
 *
 * Color mapping:
 *   timestamp text (#94A3B8 slate-400) → textSubtle
 *   body text      (#475569 slate-600) → textBody
 *   unread dot     (#EF4444 red-500)   → destructive
 */
export function buildNotificationRowV1(params: NotificationRowV1Params): ElementTree {
  const icon = params.icon ?? 'bell';
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const timestampColor = isLight ? '#94A3B8' : t.colors.textSubtle;
  const bodyColor = isLight ? '#475569' : t.colors.textBody;
  const dotColor = isLight ? '#EF4444' : t.colors.destructive;

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
      fill: [{ type: 'solid', color: dotColor }],
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
      fill: [{ type: 'solid', color: timestampColor }],
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
      fill: [{ type: 'solid', color: bodyColor }],
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
