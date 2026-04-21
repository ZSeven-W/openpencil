import { assignIdsRecursively, buildColorSwatch, type ColorSwatchParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddColorSwatchV0Params extends ColorSwatchParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Design-system color swatch. Tree build delegated to `buildColorSwatch`. */
export async function handleAddColorSwatchV0(
  params: AddColorSwatchV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const swatch = buildColorSwatch(params);
  assignIdsRecursively(swatch);
  return insertElementTree({ binding: 'color_swatch', tree: swatch, ...params });
}
