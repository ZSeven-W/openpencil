import {
  assignIdsRecursively,
  buildSectionHeaderV1,
  type SectionHeaderV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSectionHeaderV1Params extends SectionHeaderV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Section header (v1) — theme-aware variant of add_section_header_v0.
 * No hardcoded colors in v0; all theme modes produce identical output.
 * Accepts theme param for API consistency across all v1 tools.
 */
export async function handleAddSectionHeaderV1(
  params: AddSectionHeaderV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSectionHeaderV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'sectionHeader', tree, ...params });
}
