import type { ElementTree } from './helpers.js';

export interface FaqItemParams {
  /** Question text (bold, shown in the header row). */
  question: string;
  /** Answer text (only emitted when `expanded: true`). */
  answer?: string;
  /**
   * When true, chevron points down and the answer paragraph renders
   * beneath the header. When false (default), chevron points right
   * and only the header renders — matching a classic accordion's
   * collapsed state.
   */
  expanded?: boolean;
  /** Optional divider below the row. Default false (caller composes dividers). */
  show_divider?: boolean;
}

/**
 * FAQ / accordion item — one row in a frequently-asked-questions
 * list. Collapsed: header-only (question + chevron-right). Expanded:
 * header (chevron-down) + answer paragraph below. Caller stacks
 * multiple in a vertical parent to build the full list. When the
 * spec needs dividers between items, set `show_divider: true` OR
 * compose `add_divider_v0` between items via batch_design — both
 * patterns are common in the wild.
 *
 * Structure (expanded):
 *   frame(vertical, gap=12, role='faq-item', fill_container)
 *     ├ frame(horizontal, justify=space-between, align=center, role='faq-header')
 *     │   ├ text(question, 15/600, role='faq-question')
 *     │   └ icon_font(chevron-down, 20×20, role='faq-chevron-open')
 *     ├ text(answer, 14/400, role='faq-answer', fill_container)
 *     └ [divider rect if show_divider]
 *
 * Collapsed variant drops the answer and uses chevron-right.
 */
export function buildFaqItem(params: FaqItemParams): ElementTree {
  const expanded = params.expanded === true;
  const showDivider = params.show_divider === true;

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
        {
          type: 'text',
          name: 'Question',
          role: 'faq-question',
          content: params.question,
          fontSize: 15,
          fontWeight: 600,
          width: 'fill_container',
        },
        {
          type: 'icon_font',
          name: 'Chevron',
          role: expanded ? 'faq-chevron-open' : 'faq-chevron-closed',
          iconFontName: expanded ? 'chevron-down' : 'chevron-right',
          iconFontFamily: 'lucide',
          width: 20,
          height: 20,
          fill: [{ type: 'solid', color: '#64748B' }],
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
      fill: [{ type: 'solid', color: '#475569' }],
    });
  }

  if (showDivider) {
    children.push({
      type: 'rectangle',
      name: 'Divider',
      role: 'faq-divider',
      width: 'fill_container',
      height: 1,
      fill: [{ type: 'solid', color: '#E2E8F0' }],
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
