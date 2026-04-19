import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddBreadcrumbV0Item {
  label: string;
  active?: boolean;
}

export interface AddBreadcrumbV0Params {
  items: AddBreadcrumbV0Item[];
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Breadcrumb trail: Home › Settings › Billing. Interleaves item text
 * with `chevron-right` separators. The last item (or any item marked
 * active=true) gets fontWeight=600.
 *
 * roles: 'breadcrumb' / 'breadcrumb-item' | 'breadcrumb-item-active' /
 * 'breadcrumb-separator'.
 */
export async function handleAddBreadcrumbV0(
  params: AddBreadcrumbV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [];
  const lastIdx = params.items.length - 1;
  params.items.forEach((item, i) => {
    const isActive = item.active === true || i === lastIdx;
    children.push({
      type: 'text',
      name: `Item (${item.label})`,
      role: isActive ? 'breadcrumb-item-active' : 'breadcrumb-item',
      content: item.label,
      fontSize: 13,
      fontWeight: isActive ? 600 : 400,
    });
    if (i < lastIdx) {
      children.push({
        type: 'icon_font',
        name: 'Separator',
        role: 'breadcrumb-separator',
        iconFontName: 'chevron-right',
        iconFontFamily: 'lucide',
        width: 14,
        height: 14,
      });
    }
  });
  const bc = {
    type: 'frame',
    name: 'Breadcrumb',
    role: 'breadcrumb',
    width: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 6,
    children,
  };
  assignIdsRecursively(bc);
  return insertElementTree({ binding: 'breadcrumb', tree: bc, ...params });
}
