import { assignIdsRecursively, buildSearchBar, type SearchBarParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSearchBarV0Params extends SearchBarParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Search bar (height=44, cornerRadius=22). Tree build delegated to
 * `@zseven-w/pen-core`'s `buildSearchBar` — rounded horizontal frame
 * with leading icon (default 'search') + placeholder text.
 * width=fill_container so the bar stretches in a form / header.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddSearchBarV0(
  params: AddSearchBarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const bar = buildSearchBar(params);
  assignIdsRecursively(bar);
  return insertElementTree({ binding: 'search', tree: bar, ...params });
}
