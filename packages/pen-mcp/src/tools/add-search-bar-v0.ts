import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddSearchBarV0Params {
  placeholder?: string;
  leading_icon?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Search bar pattern matching the role spec in design-prompt.ts ROLE_GUIDE:
 *   "search-bar → height:44, cornerRadius:22"
 * And the affordance rule from DESIGN_GUIDELINES:
 *   "search→leading SearchIcon"
 *
 * Output: rounded horizontal frame + leading icon (default 'search') +
 * placeholder text. width=fill_container so the bar stretches in a
 * form / header. Short placeholder stays fit_content inside the
 * horizontal parent (overflow.md: "Short labels in horizontal rows:
 * width='fit_content'").
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddSearchBarV0(
  params: AddSearchBarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const icon = params.leading_icon ?? 'search';
  const placeholder = params.placeholder ?? 'Search...';
  const bar = {
    type: 'frame',
    name: 'Search Bar',
    role: 'search-bar',
    width: 'fill_container',
    height: 44,
    cornerRadius: 22,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    padding: [0, 16],
    children: [
      {
        type: 'icon_font',
        name: 'Leading Icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 20,
        height: 20,
      },
      {
        type: 'text',
        name: 'Placeholder',
        content: placeholder,
        fontSize: 14,
        fontWeight: 400,
      },
    ],
  };
  assignIdsRecursively(bar);
  return insertElementTree({ binding: 'search', tree: bar, ...params });
}
