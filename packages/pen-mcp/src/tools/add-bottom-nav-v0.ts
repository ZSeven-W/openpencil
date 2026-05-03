import { assignIdsRecursively, buildBottomNav, type BottomNavParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBottomNavV0Params extends BottomNavParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { BottomNavItem as AddBottomNavV0Item } from '@zseven-w/pen-core';

/**
 * Inline bottom navigation bar with fixed schema.
 * Tree build delegated to `@zseven-w/pen-core`'s `buildBottomNav`.
 *
 * Solves two failure modes taught in
 * `packages/pen-ai-skills/skills/phases/generation/layout.md`:
 *   - NO FIXED-POSITION (don't emit empty spacer siblings after the nav)
 *   - bottom-nav is inline part of the page flow, not floating
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddBottomNavV0(
  params: AddBottomNavV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const nav = buildBottomNav(params);
  assignIdsRecursively(nav);
  return insertElementTree({ binding: 'nav', tree: nav, ...params });
}
