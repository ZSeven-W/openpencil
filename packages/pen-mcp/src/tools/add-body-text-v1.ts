import { assignIdsRecursively, buildBodyTextV1, type BodyTextV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBodyTextV1Params extends BodyTextV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Body / description text (v1) — theme-aware variant of add_body_text_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * CJK auto-detection is preserved in all modes.
 * Since buildBodyText emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildBodyTextV1` in `@zseven-w/pen-core`.
 */
export async function handleAddBodyTextV1(
  params: AddBodyTextV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildBodyTextV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'bt', tree, ...params });
}
