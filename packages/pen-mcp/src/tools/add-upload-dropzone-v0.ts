import {
  assignIdsRecursively,
  buildUploadDropzone,
  type UploadDropzoneParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddUploadDropzoneV0Params extends UploadDropzoneParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** File upload dropzone. Tree build delegated to `buildUploadDropzone`. */
export async function handleAddUploadDropzoneV0(
  params: AddUploadDropzoneV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const d = buildUploadDropzone(params);
  assignIdsRecursively(d);
  return insertElementTree({ binding: 'dropzone', tree: d, ...params });
}
