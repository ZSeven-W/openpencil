import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface StepCardV1Params {
  /**
   * Step index — short label rendered inside the 36px marker circle.
   * Keep it 1–3 characters (e.g. 1, "01", "1.1").
   */
  number: string | number;
  /** Step title (16/600). */
  title: string;
  /** Description body (14/400, muted). */
  description: string;
  /** Whether the step is completed. Default false. Filled circle vs ring. */
  completed?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_step_card_v0.
   * - `'dark'`: title text → textPrimary; description → textMuted;
   *   incomplete circle bg → surface (instead of #FFFFFF). Accent stays
   *   hardcoded (#2563EB) — brand color, theme-independent.
   * - `'system'`: $color-* refs for title, description, and incomplete circle bg.
   */
  theme?: V1Theme;
}

const ACCENT = '#2563EB';

/**
 * Step card (v1) — theme-aware variant of buildStepCard.
 * Light mode is byte-equal to add_step_card_v0.
 *
 * Color mapping:
 *   title text   (#0F172A slate-900)  → textPrimary token
 *   desc text    (#475569 slate-600)  → textMuted token
 *   ring bg      (#FFFFFF white)      → surface token (incomplete state only)
 *   accent       (#2563EB blue-600)   → kept hardcoded (brand color)
 *   check icon   (#FFFFFF white)      → kept hardcoded (on-accent always white)
 */
export function buildStepCardV1(params: StepCardV1Params): ElementTree {
  const completed = params.completed === true;
  const numberText = String(params.number);
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Title text: light → slate-900 (#0F172A), dark/system → textPrimary
  const titleColor = isLight ? '#0F172A' : t.colors.textPrimary;
  // Description text: light → slate-600 (#475569), dark/system → textMuted
  const descColor = isLight ? '#475569' : t.colors.textMuted;
  // Incomplete circle bg: light → #FFFFFF, dark/system → surface
  const ringBg = isLight ? '#FFFFFF' : t.colors.surface;

  const circle: ElementTree = {
    type: 'frame',
    name: 'Number Circle',
    role: 'step-card-circle',
    width: 36,
    height: 36,
    cornerRadius: 18,
    fill: [{ type: 'solid', color: completed ? ACCENT : ringBg }],
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
            fill: [{ type: 'solid', color: titleColor }],
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
            fill: [{ type: 'solid', color: descColor }],
          },
        ],
      },
    ],
  };
}
