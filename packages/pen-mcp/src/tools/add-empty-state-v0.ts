import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddEmptyStateV0Params {
  title: string;
  subtitle?: string;
  icon?: string;
  cta_label?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Empty-state block (for "no results", "nothing yet", "you're all caught up").
 * Vertical centered stack: [icon?] + title + [subtitle?] + [CTA button?].
 * Container: width=fill_container, alignItems=center, padding=[48,24], gap=16.
 *
 * roles: 'empty-state' / 'empty-state-icon' / 'empty-state-title' /
 * 'empty-state-subtitle' / 'button'.
 *
 * Use for "empty list", "no results", "nothing here yet", "first-run state".
 * Weak models frequently emit sparse layouts or drop the CTA — the tool
 * locks the 4-piece structure so the generated page always composes.
 */
export async function handleAddEmptyStateV0(
  params: AddEmptyStateV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      role: 'empty-state-icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 48,
      height: 48,
    });
  }
  children.push({
    type: 'text',
    name: 'Title',
    role: 'empty-state-title',
    content: params.title,
    fontSize: 18,
    fontWeight: 600,
  });
  if (params.subtitle) {
    children.push({
      type: 'text',
      name: 'Subtitle',
      role: 'empty-state-subtitle',
      content: params.subtitle,
      fontSize: 14,
      fontWeight: 400,
    });
  }
  if (params.cta_label) {
    children.push({
      type: 'frame',
      name: 'CTA',
      role: 'button',
      cornerRadius: 24,
      padding: [12, 24],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'CTA Label',
          role: 'label',
          content: params.cta_label,
          fontSize: 14,
          fontWeight: 500,
        },
      ],
    });
  }
  const frame = {
    type: 'frame',
    name: 'Empty State',
    role: 'empty-state',
    width: 'fill_container',
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 16,
    padding: [48, 24],
    children,
  };
  assignIdsRecursively(frame);
  return insertElementTree({ binding: 'empty', tree: frame, ...params });
}
