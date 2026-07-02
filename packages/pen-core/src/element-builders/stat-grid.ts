import type { ElementTree } from './helpers.js';

export interface StatGridItem {
  value: string;
  label: string;
  icon?: string;
}

export interface StatGridParams {
  items: StatGridItem[];
  gap?: number;
}

/**
 * Non-scrolling stat grid — 2-5 items share the row via
 * width=fill_container on every cell. Solves the "activity-rings
 * overflow" anti-pattern (fixed-px items inside a constrained
 * card: the last one clips on the right edge). Difference from
 * add_metric_row_v0 which uses HORIZONTAL SCROLL + fixed-px items.
 */
export function buildStatGrid(params: StatGridParams): ElementTree {
  const gap = params.gap ?? 16;
  const cells = params.items.map((item) => buildCell(item));
  return {
    type: 'frame',
    name: 'Stat Grid',
    role: 'stat-grid',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    gap,
    alignItems: 'center',
    justifyContent: 'space_between',
    children: cells,
  };
}

function buildCell(item: StatGridItem): ElementTree {
  const children: ElementTree[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  children.push({
    type: 'text',
    name: 'Value',
    role: 'heading',
    content: item.value,
    fontSize: 24,
    fontWeight: 700,
    width: 'fill_container',
  });
  children.push({
    type: 'text',
    name: 'Label',
    role: 'body',
    content: item.label,
    fontSize: 12,
    fontWeight: 500,
    width: 'fill_container',
  });
  return {
    type: 'frame',
    name: 'Stat Cell',
    role: 'stat-cell',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
