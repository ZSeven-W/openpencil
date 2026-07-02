import type { ElementTree } from './helpers.js';

export interface TimelineItem {
  title: string;
  subtitle?: string;
  active?: boolean;
}

export interface TimelineParams {
  items: TimelineItem[];
}

const CONNECTOR_HEIGHT = 24;

/**
 * Vertical timeline. 24×24 dot in icon column + title/subtitle in
 * content column. Connector is a fixed 24px rectangle (NOT
 * fill_container — pen-core has no minHeight/stretch, so short
 * content would collapse fill_container connectors to 0).
 *
 * Spacing: no row padding, no outer gap, no icon-col gap —
 * connector IS the full 24px inter-item distance so dots land
 * flush. Known limit: wrap-content >48px leaves a small gap
 * before the next dot (pen-core no stretch).
 */
export function buildTimeline(params: TimelineParams): ElementTree {
  const items = Array.isArray(params.items) ? params.items : [];
  if (items.length === 0) {
    throw new Error('buildTimeline: items must contain at least one entry');
  }
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
        fill: [{ type: 'solid', color: dotActive ? '#2563EB' : '#E5E7EB' }],
      },
    ];
    if (!isLast) {
      iconColumnChildren.push({
        type: 'rectangle',
        name: 'Connector',
        role: 'timeline-connector',
        width: 2,
        height: CONNECTOR_HEIGHT,
        fill: [{ type: 'solid', color: '#E5E7EB' }],
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
        fill: [{ type: 'solid', color: '#6B7280' }],
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
