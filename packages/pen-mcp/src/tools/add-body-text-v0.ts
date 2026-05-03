import { assignIdsRecursively, buildBodyText, type BodyTextParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBodyTextV0Params extends BodyTextParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Body / description text — always fontFamily='Inter'; lineHeight +
 * letterSpacing script-sensitive (CJK 1.6 + letterSpacing=0; Latin 1.5).
 * Tree build delegated to `@zseven-w/pen-core`'s `buildBodyText`.
 * Always width=fill_container + textGrowth='fixed-width' so long
 * paragraphs wrap. Intended for VERTICAL-layout parents.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddBodyTextV0(
  params: AddBodyTextV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const body = buildBodyText(params);
  assignIdsRecursively(body);
  return insertElementTree({ binding: 'body', tree: body, ...params });
}
