import {
  assignIdsRecursively,
  buildAttachmentRow,
  type AttachmentRowParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddAttachmentRowV0Params extends AttachmentRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** File attachment row. Tree build delegated to `buildAttachmentRow`. */
export async function handleAddAttachmentRowV0(
  params: AddAttachmentRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const r = buildAttachmentRow(params);
  assignIdsRecursively(r);
  return insertElementTree({ binding: 'attachment', tree: r, ...params });
}
