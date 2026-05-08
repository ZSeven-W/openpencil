import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface TimelineV1Item {
  title: string;
  subtitle?: string;
  active?: boolean;
}

export interface TimelineV1Params {
  items: TimelineV1Item[];
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_timeline_v0.
   * - `'dark'`: dark-mode hex fills for inactive dot, connector, subtitle.
   * - `'system'`: emits `$color-*` ref strings.
   */
  theme?: V1Theme;
}

const CONNECTOR_HEIGHT = 24;

// Active dot: #2563EB accent — brand token, hardcoded across all modes (spec §3.4).
const ACCENT = '#2563EB';

/**
 * Vertical timeline (v1) — theme-aware variant of buildTimeline.
 * Light mode is byte-equal to add_timeline_v0.
 *
 * Color slots:
 * - Active dot fill: #2563EB accent (hardcoded, spec §3.4)
 * - Inactive dot fill: #E5E7EB → tokenized (border)
 * - Connector fill: #E5E7EB → tokenized (border)
 * - Subtitle text fill: #6B7280 → tokenized (textMuted)
 */
export function buildTimelineV1(params: TimelineV1Params): ElementTree {
  const items = Array.isArray(params.items) ? params.items : [];
  if (items.length === 0) {
    throw new Error('buildTimelineV1: items must contain at least one entry');
  }
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);

  // Light mode uses v0 literals for byte-parity
  const inactiveDot = theme === 'light' ? '#E5E7EB' : t.colors.border;
  const connector = theme === 'light' ? '#E5E7EB' : t.colors.border;
  const subtitleColor = theme === 'light' ? '#6B7280' : t.colors.textMuted;

  const rows: ElementTree[] = items.map((item, i) => {
    const isLast = i === items.length - 1;
    const dotActive = item.active === true;
    const iconColumnChildren: ElementTree[] = [
      {
        type: 'frame',
        name: 'Dot',
        role: dotActive ? 'timeline-dot-active' : 'timeline-dot',
        width: 24,
        height: 24,
        cornerRadius: 12,
        fill: [{ type: 'solid', color: dotActive ? ACCENT : inactiveDot }],
      },
    ];
    if (!isLast) {
      iconColumnChildren.push({
        type: 'rectangle',
        name: 'Connector',
        role: 'timeline-connector',
        width: 2,
        height: CONNECTOR_HEIGHT,
        fill: [{ type: 'solid', color: connector }],
      });
    }
    const contentChildren: ElementTree[] = [
      {
        type: 'text',
        name: 'Title',
        role: 'timeline-title',
        content: item.title,
        fontSize: 14,
        fontWeight: 600,
      },
    ];
    if (item.subtitle) {
      contentChildren.push({
        type: 'text',
        name: 'Subtitle',
        role: 'timeline-subtitle',
        content: item.subtitle,
        fontSize: 12,
        fontWeight: 400,
        fill: [{ type: 'solid', color: subtitleColor }],
      });
    }
    return {
      type: 'frame',
      name: `Item ${i + 1}`,
      role: 'timeline-item',
      width: 'fill_container',
      layout: 'horizontal',
      alignItems: 'flex-start',
      gap: 12,
      children: [
        {
          type: 'frame',
          name: 'Icon Column',
          role: 'timeline-icon-column',
          width: 24,
          height: 'fit_content',
          layout: 'vertical',
          alignItems: 'center',
          gap: 0,
          children: iconColumnChildren,
        },
        {
          type: 'frame',
          name: 'Content',
          role: 'timeline-content',
          width: 'fill_container',
          height: 'fit_content',
          layout: 'vertical',
          gap: 4,
          children: contentChildren,
        },
      ],
    };
  });
  return {
    type: 'frame',
    name: 'Timeline',
    role: 'timeline',
    width: 'fill_container',
    layout: 'vertical',
    gap: 0,
    children: rows,
  };
}
