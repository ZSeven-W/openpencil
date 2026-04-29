import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface FaqItemV1Params {
  /** Question text (bold, shown in the header row). */
  question: string;
  /** Answer text (only emitted when `expanded: true`). */
  answer?: string;
  /**
   * When true, chevron points down and the answer paragraph renders
   * beneath the header. When false (default), chevron points right
   * and only the header renders.
   */
  expanded?: boolean;
  /** Optional divider below the row. Default false. */
  show_divider?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_faq_item_v0.
   * - `'dark'`: dark text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * FAQ / accordion item — theme-aware version of buildFaqItem.
 * Light mode is byte-equal to add_faq_item_v0.
 *
 * Color mapping:
 *   chevron icon (#64748B)       → textMuted
 *   answer text (#475569)        → textMuted
 *   divider fill (#E2E8F0)       → border
 *   question text: no fill in v0 (inherits) — null in light, textPrimary in dark/system
 */
export function buildFaqItemV1(params: FaqItemV1Params): ElementTree {
  const expanded = params.expanded === true;
  const showDivider = params.show_divider === true;
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const chevronColor = isLight ? '#64748B' : t.colors.textMuted;
  const answerColor = isLight ? '#475569' : t.colors.textMuted;
  const dividerColor = isLight ? '#E2E8F0' : t.colors.border;

  // Question text: v0 emits no fill — keep null for light, add fill for dark/system
  const questionNode: ElementTree = {
    type: 'text',
    name: 'Question',
    role: 'faq-question',
    content: params.question,
    fontSize: 15,
    fontWeight: 600,
    width: 'fill_container',
  };
  if (!isLight) {
    questionNode.fill = [{ type: 'solid', color: t.colors.textPrimary }];
  }

  const children: ElementTree[] = [
    {
      type: 'frame',
      name: 'Header',
      role: 'faq-header',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: 12,
      children: [
        questionNode,
        {
          type: 'icon_font',
          name: 'Chevron',
          role: expanded ? 'faq-chevron-open' : 'faq-chevron-closed',
          iconFontName: expanded ? 'chevron-down' : 'chevron-right',
          iconFontFamily: 'lucide',
          width: 20,
          height: 20,
          fill: [{ type: 'solid', color: chevronColor }],
        },
      ],
    },
  ];

  if (expanded && params.answer) {
    children.push({
      type: 'text',
      name: 'Answer',
      role: 'faq-answer',
      content: params.answer,
      fontSize: 14,
      fontWeight: 400,
      width: 'fill_container',
      fill: [{ type: 'solid', color: answerColor }],
    });
  }

  if (showDivider) {
    children.push({
      type: 'rectangle',
      name: 'Divider',
      role: 'faq-divider',
      width: 'fill_container',
      height: 1,
      fill: [{ type: 'solid', color: dividerColor }],
    });
  }

  return {
    type: 'frame',
    name: 'FAQ Item',
    role: 'faq-item',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 12,
    paddingY: 16,
    children,
  };
}
