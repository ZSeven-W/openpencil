import type { ElementTree } from './helpers.js';

export interface EventCardParams {
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
}

const TITLE_FG = '#0F172A';
const META_FG = '#64748B';
const DATE_BG = '#F1F5F9';

/**
 * Calendar event tile — left date column (month + day) + right text
 * stack (title + optional time + optional location). Distinct from
 * `add_calendar_grid_v0` (the full month grid) and `add_card_row_v0`
 * (a horizontally scrolling card row with title + subtitle + image).
 *
 * Use for "upcoming event card", "meeting tile", "agenda item",
 * "calendar event row", "日程卡片", "会议条目".
 */
export function buildEventCard(params: EventCardParams): ElementTree {
  const accent = params.accent ?? '#2563EB';

  const dateColumn: ElementTree = {
    type: 'frame',
    name: 'Date Column',
    role: 'event-card-date',
    width: 64,
    height: 'fit_content',
    cornerRadius: 8,
    fill: [{ type: 'solid', color: DATE_BG }],
    layout: 'vertical',
    alignItems: 'center',
    children: [
      {
        type: 'frame',
        name: 'Month Strip',
        role: 'event-card-month-strip',
        width: 'fill_container',
        height: 'fit_content',
        fill: [{ type: 'solid', color: accent }],
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
            fill: [{ type: 'solid', color: '#FFFFFF' }],
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
            fill: [{ type: 'solid', color: TITLE_FG }],
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
      fill: [{ type: 'solid', color: TITLE_FG }],
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
          fill: [{ type: 'solid', color: META_FG }],
        },
        {
          type: 'text',
          name: 'Time',
          role: 'event-card-time',
          content: params.time,
          fontSize: 13,
          fontWeight: 400,
          fill: [{ type: 'solid', color: META_FG }],
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
          fill: [{ type: 'solid', color: META_FG }],
        },
        {
          type: 'text',
          name: 'Location',
          role: 'event-card-location',
          content: params.location,
          fontSize: 13,
          fontWeight: 400,
          fill: [{ type: 'solid', color: META_FG }],
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
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
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
