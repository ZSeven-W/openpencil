import {
  assignIdsRecursively,
  buildAttachmentRowV1,
  type AttachmentRowV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAttachmentRowV1Params extends AttachmentRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * File attachment row (v1) — theme-aware variant of add_attachment_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Surface, text hierarchy, and icon colors respond to theme.
 * Tree build delegated to `buildAttachmentRowV1` in `@zseven-w/pen-core`.
 */
export async function handleAddAttachmentRowV1(
  params: AddAttachmentRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildAttachmentRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'att', tree, ...params });
}
