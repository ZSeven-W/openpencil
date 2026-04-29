import {
  assignIdsRecursively,
  buildColorSwatchV1,
  type ColorSwatchV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddColorSwatchV1Params extends ColorSwatchV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Design-system color swatch (v1) — theme-aware variant of add_color_swatch_v0.
 * Supports 'light', 'dark', and 'system' theme modes (all produce identical trees —
 * the swatch color is caller-supplied and not tokenized).
 * Tree build delegated to `buildColorSwatchV1` in `@zseven-w/pen-core`.
 */
export async function handleAddColorSwatchV1(
  params: AddColorSwatchV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildColorSwatchV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cs', tree, ...params });
}
