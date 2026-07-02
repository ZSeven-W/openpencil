import { assignIdsRecursively, buildAvatarV1, type AvatarV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAvatarV1Params extends AvatarV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Circular avatar with optional initial (v1) — theme-aware variant of add_avatar_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Since buildAvatar emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildAvatarV1` in `@zseven-w/pen-core`.
 */
export async function handleAddAvatarV1(
  params: AddAvatarV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildAvatarV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'av', tree, ...params });
}
