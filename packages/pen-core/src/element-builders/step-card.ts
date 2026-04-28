import type { ElementTree } from './helpers.js';

export interface StepCardParams {
  /**
   * Step index — short label rendered inside the 36px marker circle.
   * Keep it 1–3 characters (e.g. 1, "01", "1.1"). Longer strings like
   * "Step 1" overflow the circle; put that prose in `title` instead.
   * Marker has `clipContent: true` as a safety net but the real fix
   * is to keep this short.
   */
  number: string | number;
  /** Step title (16/600). */
  title: string;
  /** Description body (14/400, muted). */
  description: string;
  /** Whether the step is completed. Default false. Filled circle vs ring. */
  completed?: boolean;
}

const TITLE_FG = '#0F172A';
const DESC_FG = '#475569';
const ACCENT = '#2563EB';
const RING_BG = '#FFFFFF';

/**
 * Onboarding / how-it-works step card — numbered circle (or check
 * when completed) on the left, title over description on the right.
 * Designed to stack vertically under a section header to form an
 * onboarding column. Distinct from `add_stepper_v0` (horizontal
 * progress nav with connectors), `add_list_row_v0` (no number /
 * description split), and `add_faq_item_v0` (collapsible Q&A).
 *
 * Use for "onboarding step", "how-it-works step", "tutorial step
 * card", "setup checklist item", "操作步骤卡片", "教程步骤".
 */
export function buildStepCard(params: StepCardParams): ElementTree {
  const completed = params.completed === true;
  const numberText = String(params.number);

  const circle: ElementTree = {
    type: 'frame',
    name: 'Number Circle',
    role: 'step-card-circle',
    width: 36,
    height: 36,
    cornerRadius: 18,
    fill: [{ type: 'solid', color: completed ? ACCENT : RING_BG }],
    stroke: completed ? undefined : { thickness: 2, fill: [{ type: 'solid', color: ACCENT }] },
    clipContent: true,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: completed
      ? [
          {
            type: 'icon_font',
            name: 'Check',
            role: 'step-card-check',
            iconFontName: 'check',
            iconFontFamily: 'lucide',
            width: 18,
            height: 18,
            fill: [{ type: 'solid', color: '#FFFFFF' }],
          },
        ]
      : [
          {
            type: 'text',
            name: 'Number',
            role: 'step-card-number',
            content: numberText,
            fontSize: 15,
            fontWeight: 700,
            fill: [{ type: 'solid', color: ACCENT }],
          },
        ],
  };

  return {
    type: 'frame',
    name: 'Step Card',
    role: 'step-card',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'flex-start',
    gap: 14,
    padding: [4, 0],
    children: [
      circle,
      {
        type: 'frame',
        name: 'Body',
        role: 'step-card-body',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 4,
        padding: [4, 0, 0, 0],
        children: [
          {
            type: 'text',
            name: 'Title',
            role: 'step-card-title',
            content: params.title,
            fontSize: 16,
            fontWeight: 600,
            width: 'fill_container',
            textGrowth: 'fixed-width',
            fill: [{ type: 'solid', color: TITLE_FG }],
          },
          {
            type: 'text',
            name: 'Description',
            role: 'step-card-description',
            content: params.description,
            fontSize: 14,
            fontWeight: 400,
            lineHeight: 1.5,
            width: 'fill_container',
            textGrowth: 'fixed-width',
            fill: [{ type: 'solid', color: DESC_FG }],
          },
        ],
      },
    ],
  };
}
