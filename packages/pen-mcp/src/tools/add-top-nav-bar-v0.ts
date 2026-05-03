import { assignIdsRecursively, buildTopNavBar, type TopNavBarParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTopNavBarV0Params extends TopNavBarParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Mobile top navigation bar: optional leading icon + centered title +
 * optional trailing icon. Tree build delegated to `@zseven-w/pen-core`'s
 * `buildTopNavBar`. Empty icon slots become same-footprint spacers so
 * the title stays centered.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddTopNavBarV0(
  params: AddTopNavBarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const bar = buildTopNavBar(params);
  assignIdsRecursively(bar);
  return insertElementTree({ binding: 'nav', tree: bar, ...params });
}
