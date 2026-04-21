import { assignIdsRecursively, buildIconButton, type IconButtonParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddIconButtonV0Params extends IconButtonParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Icon-only button. Tree build delegated to `@zseven-w/pen-core`'s
 * `buildIconButton` — 44×44 default (Apple HIG + Material
 * min-hit-target) with flex-centered icon. Never layout=none +
 * manual x/y (renderer unreliability).
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddIconButtonV0(
  params: AddIconButtonV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const button = buildIconButton(params);
  assignIdsRecursively(button);
  return insertElementTree({ binding: 'btn', tree: button, ...params });
}
