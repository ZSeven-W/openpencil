import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddStatGridV0Item {
  value: string;
  label: string;
  icon?: string;
}

export interface AddStatGridV0Params {
  items: AddStatGridV0Item[];
  gap?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Fixed-column stat grid where every item gets `width: "fill_container"` so
 * the row auto-distributes available space between 2-5 items without
 * overflowing the parent.
 *
 * Solves the documented "activity-rings overflow" anti-pattern in
 * packages/pen-ai-skills/skills/phases/generation/layout.md: three
 * fixed-100px rings with 24px gap inside a 279px inner card OVERFLOW;
 * the third is silently clipped on the right edge. Forcing fill_container
 * makes the renderer share space mathematically — no clipping possible.
 *
 * Different from add_metric_row_v0 which is HORIZONTAL SCROLL (fit_content
 * + clipContent wrapper) with fixed-px items: stat_grid is the
 * NON-scrolling in-card variant.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddStatGridV0(
  params: AddStatGridV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const gap = params.gap ?? 16;
  const cells = params.items.map((item) => buildCell(item));
  const grid = {
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
  assignIdsRecursively(grid);
  return insertElementTree({ binding: 'grid', tree: grid, ...params });
}

function buildCell(item: AddStatGridV0Item): Record<string, unknown> {
  const children: Record<string, unknown>[] = [];
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
    width: 'fill_container', // critical: shares space with siblings, no overflow
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
