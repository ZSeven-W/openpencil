import {
  assignIdsRecursively,
  buildProfileHeaderV1,
  type ProfileHeaderV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddProfileHeaderV1Params extends ProfileHeaderV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Large profile header (v1) — theme-aware variant of add_profile_header_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * name → textPrimary, handle → textMuted, bio → textBody in dark/system;
 * avatar bg (#3B82F6) and initial text (white) stay brand-invariant.
 */
export async function handleAddProfileHeaderV1(
  params: AddProfileHeaderV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildProfileHeaderV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'profileHeader', tree, ...params });
}
