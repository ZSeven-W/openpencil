import { assignIdsRecursively, buildHeading, type HeadingParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export type AddHeadingV0Level = 'display' | 'h1' | 'h2' | 'h3';

export interface AddHeadingV0Params extends HeadingParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Typographic heading — single text node with Pencil-demo-derived
 * size / weight / lineHeight presets. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildHeading` (also used by apps/web's
 * client shim so shape is drift-free).
 *
 * Latin vs CJK dispatch:
 *   - CJK: lineHeight 1.3-1.4, letterSpacing 0, fontFamily per script
 *   - Latin: lineHeight 1.0-1.25 per level, optional negative letterSpacing
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddHeadingV0(
  params: AddHeadingV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const heading = buildHeading(params);
  assignIdsRecursively(heading);
  return insertElementTree({ binding: 'h', tree: heading, ...params });
}
