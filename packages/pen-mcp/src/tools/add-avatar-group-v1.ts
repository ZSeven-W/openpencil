import {
  assignIdsRecursively,
  buildAvatarGroupV1,
  type AvatarGroupV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAvatarGroupV1Params extends AvatarGroupV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Stacked avatar group (v1) — theme-aware variant of add_avatar_group_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Ring color, overflow bg/text, and initial text respond to theme.
 * Brand avatar palette colors (#3B82F6, #10B981, …) stay hardcoded across all themes.
 * Tree build delegated to `buildAvatarGroupV1` in `@zseven-w/pen-core`.
 */
export async function handleAddAvatarGroupV1(
  params: AddAvatarGroupV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildAvatarGroupV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ag', tree, ...params });
}
