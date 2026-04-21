import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddChartBarsV0Params {
  values: number[];
  bar_width?: number;
  gap?: number;
  chart_height?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Bar-chart skeleton — one rectangle per `values` entry, bottom-aligned
 * via alignItems=flex-end so the bars read as axis-anchored. Each bar's
 * height is proportional to max(values); zero-valued bars get a 2px
 * floor so pen-core doesn't collapse them (fit_content on 0 height
 * disappears).
 *
 * Negative / non-finite values are clamped to 0 — this is a visual
 * skeleton, not a data pipeline; caller sanitizes before passing.
 * No axes, labels, or grid; caller stitches those via batch_design
 * U-op when needed.
 *
 * roles: 'chart-bars' (container) / 'chart-bar' (each bar).
 */
export async function handleAddChartBarsV0(
  params: AddChartBarsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const raw = Array.isArray(params.values) ? params.values : [];
  if (raw.length === 0) {
    throw new Error('add_chart_bars_v0: values must contain at least one number');
  }
  const values = raw.map((v) => (Number.isFinite(v) ? Math.max(0, v) : 0));
  const max = Math.max(1, ...values);
  const barWidth = Math.max(4, Math.floor(params.bar_width ?? 24));
  const gap = Math.max(0, Math.floor(params.gap ?? 12));
  const chartHeight = Math.max(40, Math.floor(params.chart_height ?? 160));
  const children = values.map((v, i) => ({
    type: 'rectangle',
    name: `Bar ${i + 1}`,
    role: 'chart-bar',
    width: barWidth,
    height: Math.max(2, Math.round((v / max) * chartHeight)),
    cornerRadius: 4,
    fill: [{ type: 'solid', color: '#2563EB' }],
  }));
  const tree = {
    type: 'frame',
    name: 'Chart Bars',
    role: 'chart-bars',
    width: 'fit_content',
    height: chartHeight,
    layout: 'horizontal',
    alignItems: 'flex-end',
    gap,
    children,
  };
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'chart', tree, ...params });
}
