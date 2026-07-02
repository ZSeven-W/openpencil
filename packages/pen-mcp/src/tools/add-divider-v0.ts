import { assignIdsRecursively, buildDivider, type DividerParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDividerV0Params extends DividerParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Hairline divider. Tree build delegated to `@zseven-w/pen-core`'s
 * `buildDivider`. Horizontal (default) = fill_container × thickness;
 * vertical = thickness × fill_container.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddDividerV0(
  params: AddDividerV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const divider = buildDivider(params);
  assignIdsRecursively(divider);
  return insertElementTree({ binding: 'div', tree: divider, ...params });
}
