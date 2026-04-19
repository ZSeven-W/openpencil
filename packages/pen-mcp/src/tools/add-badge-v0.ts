import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddBadgeV0Params {
  label: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Short inline badge / pill / tag (e.g. "NEW", "BETA", "42").
 *
 * Documented constraint from memory / overflow.md:
 *   "Badges: short labels only (CJK ≤8 chars / Latin ≤16 chars)."
 *
 * Forces the standard pill layout: frame(horizontal, padding=[4,10],
 * cornerRadius=999 for full pill, alignItems=center, gap=4) + inner text.
 * Colors are hardcoded neutral to stay Style-Guide-orthogonal — override
 * via batch_design U-op.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddBadgeV0(
  params: AddBadgeV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const badge = {
    type: 'frame',
    name: 'Badge',
    role: 'badge',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    padding: [4, 10],
    cornerRadius: 999,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: params.label,
        fontSize: 11,
        fontWeight: 600,
      },
    ],
  };
  assignIdsRecursively(badge);
  return insertElementTree({ binding: 'badge', tree: badge, ...params });
}
