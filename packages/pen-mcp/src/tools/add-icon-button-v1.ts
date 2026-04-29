import {
  assignIdsRecursively,
  buildIconButtonV1,
  type IconButtonV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddIconButtonV1Params extends IconButtonV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Icon-only button (v1) — theme-aware variant of add_icon_button_v0.
 * No hardcoded colors in v0, so light/dark/system modes are identical
 * (byte-parity with v0 in all modes). Accepts theme param for API
 * consistency across all v1 tools.
 */
export async function handleAddIconButtonV1(
  params: AddIconButtonV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const btn = buildIconButtonV1(params);
  assignIdsRecursively(btn);
  return insertElementTree({ binding: 'btn', tree: btn, ...params });
}
