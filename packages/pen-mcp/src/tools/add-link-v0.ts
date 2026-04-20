import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddLinkV0Params {
  label: string;
  trailing_icon?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Inline text link with optional trailing icon — the "Learn more →"
 * pattern. Emits a horizontal fit_content frame. Omit `trailing_icon` for
 * a pure-text link; the default lucide glyph is `arrow-right` when the
 * caller asks for the common CTA-style variant by passing an empty
 * string (reserved for future shorthand; today you just pass the name
 * you want).
 *
 * Colors + underline styling stay Style-Guide orthogonal — the role
 * `link` / `link-label` / `link-icon` is enough for a follow-up
 * batch_design U-op to theme.
 */
export async function handleAddLinkV0(
  params: AddLinkV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [
    {
      type: 'text',
      name: 'Label',
      role: 'link-label',
      content: params.label,
      fontSize: 14,
      fontWeight: 500,
    },
  ];
  if (params.trailing_icon) {
    children.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      role: 'link-icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 14,
      height: 14,
    });
  }
  const link = {
    type: 'frame',
    name: 'Link',
    role: 'link',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
  assignIdsRecursively(link);
  return insertElementTree({ binding: 'link', tree: link, ...params });
}
