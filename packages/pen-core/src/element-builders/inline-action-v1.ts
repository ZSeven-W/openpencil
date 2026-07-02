import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface InlineActionV1Params {
  /** Status / context message (e.g. "Comment deleted"). */
  message: string;
  /** Action label (e.g. "Undo", "Retry"). */
  action_label: string;
  /** Optional leading lucide icon slug (e.g. "info", "check-circle"). */
  icon?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_inline_action_v0.
   * - `'dark'`: icon/message → textBody, CTA → accent.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Inline status + action row — theme-aware variant of buildInlineAction.
 * Light mode is byte-equal to add_inline_action_v0.
 *
 * Color mapping:
 *   leading icon (#64748B slate-500) → textBody
 *   message text (#475569 slate-600) → textBody
 *   CTA label   (#2563EB blue-600)   → accent (brand-invariant: #2563EB in light,
 *                                       t.colors.accent in dark/system)
 */
export function buildInlineActionV1(params: InlineActionV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const iconColor = isLight ? '#64748B' : t.colors.textBody;
  const messageColor = isLight ? '#475569' : t.colors.textBody;
  const ctaColor = isLight ? '#2563EB' : t.colors.accent;

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
      fill: [{ type: 'solid', color: iconColor }],
    });
  }
  leftChildren.push({
    type: 'text',
    name: 'Message',
    role: 'inline-action-message',
    content: params.message,
    fontSize: 13,
    fontWeight: 400,
    fill: [{ type: 'solid', color: messageColor }],
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
        fill: [{ type: 'solid', color: ctaColor }],
      },
    ],
  };
}
