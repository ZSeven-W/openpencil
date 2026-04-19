import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddIconLabelV0Params {
  icon: string;
  label: string;
  gap?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Atomic icon + label pair (horizontal). Common building block for menu
 * items, breadcrumbs, status indicators, "with icon" inline text.
 *
 * Forces the alignItems=center pattern so icon and text baseline-align
 * visually. Narrow schema: no alignment enum (icons always lead), no
 * size/weight overrides (defaults 16/14/500). Callers wanting variants
 * (trailing icon, large size) compose via batch_design instead.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddIconLabelV0(
  params: AddIconLabelV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const gap = params.gap ?? 8;
  const node = {
    type: 'frame',
    name: 'Icon Label',
    role: 'icon-label',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap,
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: params.icon,
        iconFontFamily: 'lucide',
        width: 16,
        height: 16,
      },
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: params.label,
        fontSize: 14,
        fontWeight: 500,
      },
    ],
  };
  assignIdsRecursively(node);
  return insertElementTree({ binding: 'icon_label', tree: node, ...params });
}
