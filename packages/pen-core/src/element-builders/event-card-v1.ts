import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface EventCardV1Params {
  /** Short month code shown in the date column (e.g. "OCT", "10月"). */
  month: string;
  /** Day-of-month (e.g. "15"). */
  day: string | number;
  /** Event title (15/600). */
  title: string;
  /** Optional time string (e.g. "2:00 PM – 3:30 PM", "14:00"). */
  time?: string;
  /** Optional location string (e.g. "Conference Room B"). */
  location?: string;
  /** Accent color for the date column header. Default "#2563EB". */
  accent?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_event_card_v0.
   * - `'dark'`: dark surface/text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Calendar event tile — theme-aware version of buildEventCard.
 * Light mode is byte-equal to add_event_card_v0.
 *
 * Color mapping:
 *   card bg (#FFFFFF)            → surface
 *   card stroke (#E2E8F0)        → border
 *   date column bg (#F1F5F9)     → surface2
 *   month strip bg (accent)      → caller-supplied accent (kept as-is, brand color)
 *   month text (#FFFFFF)         → kept as-is (white on accent, brand decision)
 *   day text (#0F172A)           → textPrimary
 *   title (#0F172A)              → textPrimary
 *   meta icons/text (#64748B)    → textMuted
 */
export function buildEventCardV1(params: EventCardV1Params): ElementTree {
  const accent = params.accent ?? '#2563EB';
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const cardBg = isLight ? '#FFFFFF' : t.colors.surface;
  const cardStroke = isLight ? '#E2E8F0' : t.colors.border;
  const dateBg = isLight ? '#F1F5F9' : t.colors.surface2;
  const titleColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const dayColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const metaColor = isLight ? '#64748B' : t.colors.textMuted;
  // accent + white-on-accent are caller/brand-supplied — pass through in all modes
  const monthStripBg = accent;
  const monthTextColor = '#FFFFFF';

  const dateColumn: ElementTree = {
    type: 'frame',
    name: 'Date Column',
    role: 'event-card-date',
    width: 64,
    height: 'fit_content',
    cornerRadius: 8,
    fill: [{ type: 'solid', color: dateBg }],
    layout: 'vertical',
    alignItems: 'center',
    children: [
      {
        type: 'frame',
        name: 'Month Strip',
        role: 'event-card-month-strip',
        width: 'fill_container',
        height: 'fit_content',
        fill: [{ type: 'solid', color: monthStripBg }],
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'center',
        padding: [4, 0],
        children: [
          {
            type: 'text',
            name: 'Month',
            role: 'event-card-month',
            content: params.month,
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: 1,
            fill: [{ type: 'solid', color: monthTextColor }],
          },
        ],
      },
      {
        type: 'frame',
        name: 'Day Strip',
        role: 'event-card-day-strip',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'center',
        padding: [8, 0],
        children: [
          {
            type: 'text',
            name: 'Day',
            role: 'event-card-day',
            content: String(params.day),
            fontSize: 22,
            fontWeight: 700,
            fill: [{ type: 'solid', color: dayColor }],
          },
        ],
      },
    ],
  };

  const stack: ElementTree[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'event-card-title',
      content: params.title,
      fontSize: 15,
      fontWeight: 600,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: titleColor }],
    },
  ];
  if (params.time) {
    stack.push({
      type: 'frame',
      name: 'Time Row',
      role: 'event-card-meta-row',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      gap: 6,
      children: [
        {
          type: 'icon_font',
          name: 'Time Icon',
          role: 'event-card-time-icon',
          iconFontName: 'clock',
          iconFontFamily: 'lucide',
          width: 14,
          height: 14,
          fill: [{ type: 'solid', color: metaColor }],
        },
        {
          type: 'text',
          name: 'Time',
          role: 'event-card-time',
          content: params.time,
          fontSize: 13,
          fontWeight: 400,
          fill: [{ type: 'solid', color: metaColor }],
        },
      ],
    });
  }
  if (params.location) {
    stack.push({
      type: 'frame',
      name: 'Location Row',
      role: 'event-card-meta-row',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      gap: 6,
      children: [
        {
          type: 'icon_font',
          name: 'Location Icon',
          role: 'event-card-location-icon',
          iconFontName: 'map-pin',
          iconFontFamily: 'lucide',
          width: 14,
          height: 14,
          fill: [{ type: 'solid', color: metaColor }],
        },
        {
          type: 'text',
          name: 'Location',
          role: 'event-card-location',
          content: params.location,
          fontSize: 13,
          fontWeight: 400,
          fill: [{ type: 'solid', color: metaColor }],
        },
      ],
    });
  }

  return {
    type: 'frame',
    name: 'Event Card',
    role: 'event-card',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'flex-start',
    gap: 14,
    padding: [12, 12],
    cornerRadius: 12,
    fill: [{ type: 'solid', color: cardBg }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: cardStroke }] },
    children: [
      dateColumn,
      {
        type: 'frame',
        name: 'Text Stack',
        role: 'event-card-text',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 6,
        padding: [4, 0],
        children: stack,
      },
    ],
  };
}
