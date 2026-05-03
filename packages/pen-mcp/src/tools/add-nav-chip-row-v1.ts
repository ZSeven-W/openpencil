import {
  assignIdsRecursively,
  buildNavChipRowV1,
  type NavChipRowV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddNavChipRowV1Params extends NavChipRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal scroll row of nav chips (v1) — theme-aware variant of add_nav_chip_row_v0.
 * No hardcoded colors in v0; light/dark/system modes are identical (byte-parity with v0
 * in all modes). Accepts theme param for API consistency across all v1 tools.
 */
export async function handleAddNavChipRowV1(
  params: AddNavChipRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildNavChipRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'navChipRow', tree, ...params });
}
