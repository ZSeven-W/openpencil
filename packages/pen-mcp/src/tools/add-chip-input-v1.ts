import { assignIdsRecursively, buildChipInputV1, type ChipInputV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChipInputV1Params extends ChipInputV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Chip / tag input (v1) — theme-aware variant of add_chip_input_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Chip bg → surface2, field bg → surface, border → border token.
 * Tree build delegated to `buildChipInputV1` in `@zseven-w/pen-core`.
 */
export async function handleAddChipInputV1(
  params: AddChipInputV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildChipInputV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ci', tree, ...params });
}
