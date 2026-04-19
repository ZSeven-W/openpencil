import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  buildScrollWrapper,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddMetricRowV0Item {
  label: string;
  value: string;
  icon?: string;
}

export interface AddMetricRowV0Params {
  items: AddMetricRowV0Item[];
  tile_width?: number;
  gap?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal scroll row of METRIC TILES (small label + big value + optional icon).
 * Each tile: 120×100 frame, cornerRadius=16, padding=16, vertical layout,
 * label=12/500 (body role), value=28/700 (heading role).
 *
 * Narrow tool. Use when the spec says "dashboard stats", "metric tiles",
 * "KPI cards", or shows a row of statistics (Steps, Kcal, Sleep, Revenue, etc.).
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddMetricRowV0(
  params: AddMetricRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tileWidth = params.tile_width ?? 120;
  const gap = params.gap ?? 12;
  const tiles = params.items.map((item) => buildTile(item, tileWidth));
  const wrapper = buildScrollWrapper({
    rowName: 'Metric Row',
    innerChildren: tiles,
    gap,
  });
  assignIdsRecursively(wrapper);
  return insertElementTree({ binding: 'row', tree: wrapper, ...params });
}

function buildTile(item: AddMetricRowV0Item, tileWidth: number): Record<string, unknown> {
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
    name: 'Label',
    role: 'body',
    content: item.label,
    fontSize: 12,
    fontWeight: 500,
    width: 'fill_container',
  });
  children.push({
    type: 'text',
    name: 'Value',
    role: 'heading',
    content: item.value,
    fontSize: 28,
    fontWeight: 700,
    width: 'fill_container',
  });
  return {
    type: 'frame',
    name: 'Metric Tile',
    role: 'metric-tile',
    width: tileWidth,
    height: 100,
    cornerRadius: 16,
    padding: 16,
    layout: 'vertical',
    gap: 4,
    children,
  };
}
