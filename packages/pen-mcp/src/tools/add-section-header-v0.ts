import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddSectionHeaderV0Action {
  label: string;
  icon?: string;
}

export interface AddSectionHeaderV0Params {
  title: string;
  action?: AddSectionHeaderV0Action;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Dashboard / landing section header: big title on the left, optional
 * trailing action (e.g. "See all →", "View More", "Edit").
 * Forces horizontal + space_between + alignItems=center layout so the
 * action always sits flush-right regardless of title length.
 *
 * Common failure on non-Claude models: title and action stack vertically
 * (wrong layout direction) or action overlaps title (missing
 * space_between). Tool encodes the correct pattern.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddSectionHeaderV0(
  params: AddSectionHeaderV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'heading',
      content: params.title,
      fontSize: 20,
      fontWeight: 700,
    },
  ];
  if (params.action) {
    children.push(buildActionGroup(params.action));
  }
  const header = {
    type: 'frame',
    name: 'Section Header',
    role: 'section-header',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    justifyContent: 'space_between',
    alignItems: 'center',
    children,
  };
  assignIdsRecursively(header);
  return insertElementTree({ binding: 'header', tree: header, ...params });
}

function buildActionGroup(action: AddSectionHeaderV0Action): Record<string, unknown> {
  const children: Record<string, unknown>[] = [
    {
      type: 'text',
      name: 'Action Label',
      role: 'label',
      content: action.label,
      fontSize: 14,
      fontWeight: 500,
    },
  ];
  if (action.icon) {
    children.push({
      type: 'icon_font',
      name: 'Action Icon',
      iconFontName: action.icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  return {
    type: 'frame',
    name: 'Action',
    role: 'section-header-action',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
