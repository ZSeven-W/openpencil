import {
  assignIdsRecursively,
  buildUploadDropzoneV1,
  type UploadDropzoneV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddUploadDropzoneV1Params extends UploadDropzoneV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * File upload dropzone (v1) — theme-aware variant of add_upload_dropzone_v0.
 * bg (bgDeep), dashed stroke (border), icon (textMuted), title (textBody),
 * subtitle (textMuted) are all tokenized.
 */
export async function handleAddUploadDropzoneV1(
  params: AddUploadDropzoneV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildUploadDropzoneV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'upload-dropzone', tree, ...params });
}
