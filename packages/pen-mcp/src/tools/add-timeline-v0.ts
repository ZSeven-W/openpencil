import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface TimelineItemV0 {
  title: string;
  subtitle?: string;
  active?: boolean;
}

export interface AddTimelineV0Params {
  items: TimelineItemV0[];
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

const CONNECTOR_HEIGHT = 24;

/**
 * Vertical timeline. Each item: 24×24 dot in a narrow icon column
 * plus title/subtitle in a fill_container content column. Dots are
 * joined by a 2px vertical rectangle of **fixed** 24px height.
 *
 * Why fixed, not fill_container: pen-core has no minHeight and no
 * alignItems=stretch. If content col is shorter than the dot
 * (e.g. one-word title), row cross-axis = max(content=~17px,
 * dot=24) = 24, icon col gets 24, dot consumes all 24, and a
 * fill_container connector computes to 0 — the timeline visually
 * breaks. A fixed-height connector + icon col sized fit_content
 * (24 dot + 24 connector = 48, **gap=0** between them) drives the
 * row's cross-axis size so the connector survives every row
 * independent of content length.
 *
 * Spacing model: **no row padding, no outer timeline gap, no
 * icon-col gap**. The connector IS the full inter-item spacing —
 * its 24px length is the exact visual distance from dot-N-bottom
 * to dot-(N+1)-top, with the connector flush against both
 * neighboring dots. Any icon-col gap between dot and connector
 * would add to the inter-item distance but leave a dot-adjacent
 * blank stripe, breaking the "dots strung on a line" read.
 *
 * Known limitation: when content col is TALLER than icon col
 * fit_content (e.g. wrapped 3-line title), row grows to content
 * height but icon col stays at 48 (pen-core has no stretch), so
 * the connector ends before reaching the next dot — creating a
 * small visual gap proportional to the content overflow. This is
 * unavoidable without stretch; for that case fall back to
 * batch_design and position connectors manually.
 *
 * Last item drops the connector. `active` items get primary fill
 * on the dot; the rest stay gray. Semantic colors apply post-hoc
 * via batch_design U-op (roles are stable).
 *
 * roles: 'timeline' / 'timeline-item' / 'timeline-icon-column' /
 * 'timeline-dot' | 'timeline-dot-active' / 'timeline-connector' /
 * 'timeline-content' / 'timeline-title' / 'timeline-subtitle'.
 */
export async function handleAddTimelineV0(
  params: AddTimelineV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const items = Array.isArray(params.items) ? params.items : [];
  if (items.length === 0) {
    throw new Error('add_timeline_v0: items must contain at least one entry');
  }

  const rows = items.map((item, i) => {
    const isLast = i === items.length - 1;
    const dotActive = item.active === true;
    const iconColumnChildren: Record<string, unknown>[] = [
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
    const contentChildren: Record<string, unknown>[] = [
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

  const tree = {
    type: 'frame',
    name: 'Timeline',
    role: 'timeline',
    width: 'fill_container',
    layout: 'vertical',
    gap: 0,
    children: rows,
  };
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'timeline', tree, ...params });
}
