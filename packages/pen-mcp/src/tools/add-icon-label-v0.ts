import { assignIdsRecursively, buildIconLabel, type IconLabelParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddIconLabelV0Params extends IconLabelParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Atomic icon + label pair (horizontal). Tree build delegated to
 * `@zseven-w/pen-core`'s `buildIconLabel` — alignItems=center for
 * baseline alignment; icon always leads; defaults 16/14/500.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddIconLabelV0(
  params: AddIconLabelV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const node = buildIconLabel(params);
  assignIdsRecursively(node);
  return insertElementTree({ binding: 'icon_label', tree: node, ...params });
}
