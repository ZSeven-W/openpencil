import { assignIdsRecursively, buildListRow, type ListRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddListRowV0Params extends ListRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * iOS / Material-style list row. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildListRow` — [optional leading icon] +
 * [vertical text stack (title + optional subtitle)] + [optional
 * trailing icon]. Text stack wraps correctly because it's inside a
 * vertical wrapper (documented layout-engine requirement for
 * fill_container + fixed-width text wrap-height propagation).
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddListRowV0(
  params: AddListRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildListRow(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'row', tree: row, ...params });
}
