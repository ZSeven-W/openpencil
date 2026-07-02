import { assignIdsRecursively, buildStatGrid, type StatGridParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStatGridV0Params extends StatGridParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { StatGridItem as AddStatGridV0Item } from '@zseven-w/pen-core';

/**
 * Non-scrolling stat grid. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildStatGrid` — 2-5 items share the row
 * via fill_container cells so the row never overflows. Different
 * from add_metric_row_v0 which is HORIZONTAL SCROLL.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddStatGridV0(
  params: AddStatGridV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const grid = buildStatGrid(params);
  assignIdsRecursively(grid);
  return insertElementTree({ binding: 'grid', tree: grid, ...params });
}
