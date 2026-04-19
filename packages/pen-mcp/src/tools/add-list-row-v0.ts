import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddListRowV0Params {
  title: string;
  subtitle?: string;
  leading_icon?: string;
  trailing_icon?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * iOS / Material-style list row: [optional leading icon] + [vertical
 * text stack (title + optional subtitle)] + [optional trailing icon,
 * typically chevron-right].
 *
 * No-overlap invariant (same pattern as add_section_header_v0): the
 * text-stack sibling takes width:fill_container inside a VERTICAL
 * wrapper so title wraps correctly + wrap height propagates to the
 * row's fit_content height. Long titles CANNOT push past the trailing
 * icon; they wrap vertically and the row grows. text in horizontal
 * parents with fill_container+fixed-width does NOT propagate wrap
 * height per the layout engine's documented rule — the vertical
 * wrapper is why this tool needs it.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddListRowV0(
  params: AddListRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const rowChildren: Record<string, unknown>[] = [];
  if (params.leading_icon) {
    rowChildren.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
    });
  }
  // Text stack — MUST be vertical-layout wrapper so fill_container +
  // fixed-width text wraps correctly and the wrap height is measured.
  const textStackChildren: Record<string, unknown>[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'label',
      content: params.title,
      fontSize: 15,
      fontWeight: 500,
      width: 'fill_container',
      textGrowth: 'fixed-width',
    },
  ];
  if (params.subtitle) {
    textStackChildren.push({
      type: 'text',
      name: 'Subtitle',
      role: 'body',
      content: params.subtitle,
      fontSize: 13,
      fontWeight: 400,
      width: 'fill_container',
      textGrowth: 'fixed-width',
    });
  }
  rowChildren.push({
    type: 'frame',
    name: 'Text Stack',
    role: 'list-row-text',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 2,
    children: textStackChildren,
  });
  if (params.trailing_icon) {
    rowChildren.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  const row = {
    type: 'frame',
    name: 'List Row',
    role: 'list-row',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    padding: [12, 16],
    children: rowChildren,
  };
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'row', tree: row, ...params });
}
