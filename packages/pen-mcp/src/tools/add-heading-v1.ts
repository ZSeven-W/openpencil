import { assignIdsRecursively, buildHeadingV1, type HeadingV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddHeadingV1Params extends HeadingV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Typographic heading (v1) — theme-aware variant of add_heading_v0.
 * Supports three theme modes: 'light' (v0 byte-parity), 'dark'
 * (dark-theme hex), 'system' (emits $type-* + $color-text-primary refs).
 * Tree build delegated to `buildHeadingV1` in `@zseven-w/pen-core`.
 */
export async function handleAddHeadingV1(
  params: AddHeadingV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const heading = buildHeadingV1(params);
  assignIdsRecursively(heading);
  return insertElementTree({ binding: 'h', tree: heading, ...params });
}
