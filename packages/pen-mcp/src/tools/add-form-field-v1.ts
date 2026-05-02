import { assignIdsRecursively, buildFormFieldV1, type FormFieldV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFormFieldV1Params extends FormFieldV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Form field (v1) — theme-aware variant of add_form_field_v0.
 * No hardcoded colors in v0, so light/dark/system modes are identical
 * (byte-parity with v0 in all modes). Accepts theme param for API
 * consistency across all v1 tools.
 */
export async function handleAddFormFieldV1(
  params: AddFormFieldV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildFormFieldV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ff', tree, ...params });
}
