import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddTextButtonV0Params {
  label: string;
  leading_icon?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Padding-based text button. Matches the Pencil demo pattern documented
 * in memory: `frame(padding=[12,24], justifyContent=center) > text` —
 * height auto-derives from padding + text metrics, so no explicit fixed
 * height. Optional leading icon (e.g. chevron, plus) at 16px.
 *
 * Narrow by design: no size enum (single `md` preset). For other sizes,
 * callers use batch_design U-op to tune padding/fontSize.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddTextButtonV0(
  params: AddTextButtonV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [];
  if (params.leading_icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  children.push({
    type: 'text',
    name: 'Label',
    role: 'label',
    content: params.label,
    fontSize: 14,
    fontWeight: 500,
  });
  const button = {
    type: 'frame',
    name: 'Text Button',
    role: 'button',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
    padding: [12, 20],
    cornerRadius: 8,
    children,
  };
  assignIdsRecursively(button);
  return insertElementTree({ binding: 'btn', tree: button, ...params });
}
