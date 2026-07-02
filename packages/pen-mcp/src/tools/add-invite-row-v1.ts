import { assignIdsRecursively, buildInviteRowV1, type InviteRowV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInviteRowV1Params extends InviteRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Pending invite list row (v1) — theme-aware variant of add_invite_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Avatar bg → surface2, text → textPrimary/textMuted, action → accent;
 * status pills use alertColors (pending→warning, expired→danger,
 * accepted→success) in dark/system modes.
 */
export async function handleAddInviteRowV1(
  params: AddInviteRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildInviteRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'inviteRow', tree, ...params });
}
