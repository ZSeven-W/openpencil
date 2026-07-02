import { assignIdsRecursively, buildTextButton, type TextButtonParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTextButtonV0Params extends TextButtonParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Padding-based text button. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildTextButton` — matches the Pencil demo
 * pattern `frame(padding=[12,20], justifyContent=center) > [icon? + text]`
 * with height auto-derived from padding + text metrics.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddTextButtonV0(
  params: AddTextButtonV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const button = buildTextButton(params);
  assignIdsRecursively(button);
  return insertElementTree({ binding: 'btn', tree: button, ...params });
}
