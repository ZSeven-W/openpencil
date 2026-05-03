import { assignIdsRecursively, buildCodeBlockV1, type CodeBlockV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCodeBlockV1Params extends CodeBlockV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Preformatted code block (v1) — theme-aware variant of add_code_block_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Background (#F3F4F6) maps to surface2 token in dark/system modes.
 * Tree build delegated to `buildCodeBlockV1` in `@zseven-w/pen-core`.
 */
export async function handleAddCodeBlockV1(
  params: AddCodeBlockV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCodeBlockV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cod', tree, ...params });
}
