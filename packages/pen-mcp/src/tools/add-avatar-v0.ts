import { assignIdsRecursively, buildAvatar, type AvatarParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAvatarV0Params extends AvatarParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Circular avatar with optional centered initial. Tree build
 * delegated to `@zseven-w/pen-core`'s `buildAvatar` — emits
 * frame+cornerRadius=size/2 with flex centering (NEVER ellipse +
 * sibling text per layout.md §RING / CIRCLE WITH CENTER CONTENT).
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddAvatarV0(
  params: AddAvatarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const avatar = buildAvatar(params);
  assignIdsRecursively(avatar);
  return insertElementTree({ binding: 'avatar', tree: avatar, ...params });
}
