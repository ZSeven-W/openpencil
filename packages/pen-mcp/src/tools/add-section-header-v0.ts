import {
  assignIdsRecursively,
  buildSectionHeader,
  type SectionHeaderParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSectionHeaderV0Params extends SectionHeaderParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { SectionHeaderAction as AddSectionHeaderV0Action } from '@zseven-w/pen-core';

/**
 * Dashboard / landing section header: big title on the left, optional
 * trailing action. Tree build delegated to `@zseven-w/pen-core`'s
 * `buildSectionHeader` — title wrapped in vertical container with
 * textGrowth=fixed-width so multi-line titles push downstream content
 * correctly (see overflow.md §"Text in VERTICAL layout").
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddSectionHeaderV0(
  params: AddSectionHeaderV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const header = buildSectionHeader(params);
  assignIdsRecursively(header);
  return insertElementTree({ binding: 'header', tree: header, ...params });
}
