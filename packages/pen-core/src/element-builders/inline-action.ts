import type { ElementTree } from './helpers.js';

export interface InlineActionParams {
  /** Status / context message (e.g. "Comment deleted"). */
  message: string;
  /** Action label (e.g. "Undo", "Retry"). */
  action_label: string;
  /** Optional leading lucide icon slug (e.g. "info", "check-circle"). */
  icon?: string;
}

/**
 * Inline status + action row — message text on the left, blue text
 * button on the right. Distinct from `add_toast_v0` (floating pill
 * notification, NOT inline) and `add_alert_v0` (full-width banner
 * with dismiss ×). Use for "Comment deleted • Undo", "Saved offline
 * • Retry", inline micro-feedback rows that sit at the bottom of an
 * editor / list.
 */
export function buildInlineAction(params: InlineActionParams): ElementTree {
  const leftChildren: ElementTree[] = [];
  if (params.icon) {
    leftChildren.push({
      type: 'icon_font',
      name: 'Icon',
      role: 'inline-action-icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
      fill: [{ type: 'solid', color: '#64748B' }],
    });
  }
  leftChildren.push({
    type: 'text',
    name: 'Message',
    role: 'inline-action-message',
    content: params.message,
    fontSize: 13,
    fontWeight: 400,
    fill: [{ type: 'solid', color: '#475569' }],
  });
  return {
    type: 'frame',
    name: 'Inline Action',
    role: 'inline-action',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    children: [
      {
        type: 'frame',
        name: 'Message Group',
        role: 'inline-action-message-group',
        width: 'fit_content',
        height: 'fit_content',
        layout: 'horizontal',
        alignItems: 'center',
        gap: 6,
        children: leftChildren,
      },
      {
        type: 'text',
        name: 'Action',
        role: 'inline-action-cta',
        content: params.action_label,
        fontSize: 13,
        fontWeight: 600,
        fill: [{ type: 'solid', color: '#2563EB' }],
      },
    ],
  };
}
