import { assignIdsRecursively, buildAvatarGroup, type AvatarGroupParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAvatarGroupV0Params extends AvatarGroupParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { AvatarGroupItem as AddAvatarGroupV0Item } from '@zseven-w/pen-core';

/**
 * Stacked / overlapping avatar tile group. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildAvatarGroup`. Distinct from the single
 * `add_avatar_v0` tile — use this for "team online", "5 contributors",
 * "presence indicators" style affordances.
 */
export async function handleAddAvatarGroupV0(
  params: AddAvatarGroupV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const group = buildAvatarGroup(params);
  assignIdsRecursively(group);
  return insertElementTree({ binding: 'avatarGroup', tree: group, ...params });
}
