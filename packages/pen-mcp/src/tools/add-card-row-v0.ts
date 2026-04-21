import { assignIdsRecursively, buildCardRow, type CardRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCardRowV0Params extends CardRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

// Re-export types from pen-core so existing callers using
// AddCardRowV0Item keep compiling.
export type { CardRowItem as AddCardRowV0Item } from '@zseven-w/pen-core';

/**
 * Horizontal scroll row of CARDS (title + subtitle + optional icon).
 *
 * Tree build is delegated to `@zseven-w/pen-core`'s `buildCardRow` so
 * the exact same shape lands on the live canvas whether the AI calls
 * this MCP tool (external clients) or the embedded orchestrator's
 * client shim (apps/web, Phase 2 M2+). Server wraps it with the
 * parent-validation + rollback guarantees provided by
 * `ensureParentExists` + `insertElementTree`.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddCardRowV0(
  params: AddCardRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const wrapper = buildCardRow(params);
  assignIdsRecursively(wrapper);
  return insertElementTree({ binding: 'row', tree: wrapper, ...params });
}
