import { assignIdsRecursively, buildBadge, type BadgeParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBadgeV0Params extends BadgeParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Short inline badge / pill / tag. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildBadge` — standard pill pattern with
 * padding=[4,10], cornerRadius=999, alignItems=center, fontSize
 * 11/600. Colors neutral; override via batch_design U-op.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddBadgeV0(
  params: AddBadgeV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const badge = buildBadge(params);
  assignIdsRecursively(badge);
  return insertElementTree({ binding: 'badge', tree: badge, ...params });
}
