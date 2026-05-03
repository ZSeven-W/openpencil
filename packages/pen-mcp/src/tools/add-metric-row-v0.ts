import { assignIdsRecursively, buildMetricRow, type MetricRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddMetricRowV0Params extends MetricRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { MetricRowItem as AddMetricRowV0Item } from '@zseven-w/pen-core';

/**
 * Horizontal scroll row of METRIC TILES (label + big value + optional icon).
 * Tree build delegated to `@zseven-w/pen-core`'s `buildMetricRow` for
 * drift-free parity with apps/web client shim.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddMetricRowV0(
  params: AddMetricRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const wrapper = buildMetricRow(params);
  assignIdsRecursively(wrapper);
  return insertElementTree({ binding: 'row', tree: wrapper, ...params });
}
