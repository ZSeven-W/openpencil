import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddDividerV0Params {
  orientation?: 'horizontal' | 'vertical';
  thickness?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Hairline divider. Memory-documented pattern (Pencil reverse engineering):
 *   "Dividers: rectangle(h=1, fill_container) or directional stroke
 *    {thickness:{bottom:1}}"
 * This tool emits the rectangle form (more portable; stroke variant requires
 * parent to support directional stroke fields). Fills inherited from
 * ambient theme / Style Guide via a follow-up batch_design U-op if needed.
 *
 * Horizontal (default) = width:fill_container, height:thickness
 * Vertical             = width:thickness,       height:fill_container
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddDividerV0(
  params: AddDividerV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const orientation = params.orientation ?? 'horizontal';
  const thickness = params.thickness ?? 1;
  const divider: Record<string, unknown> = {
    type: 'rectangle',
    name: 'Divider',
    role: 'divider',
    width: orientation === 'horizontal' ? 'fill_container' : thickness,
    height: orientation === 'horizontal' ? thickness : 'fill_container',
  };
  assignIdsRecursively(divider);
  return insertElementTree({ binding: 'div', tree: divider, ...params });
}
