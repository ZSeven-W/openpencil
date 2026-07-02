import { assignIdsRecursively, buildCodeBlock, type CodeBlockParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCodeBlockV0Params extends CodeBlockParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Preformatted code block. Tree build delegated to `buildCodeBlock`. */
export async function handleAddCodeBlockV0(
  params: AddCodeBlockV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const block = buildCodeBlock(params);
  assignIdsRecursively(block);
  return insertElementTree({ binding: 'code_block', tree: block, ...params });
}
