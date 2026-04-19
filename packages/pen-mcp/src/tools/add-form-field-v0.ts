import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddFormFieldV0Params {
  label: string;
  placeholder?: string;
  leading_icon?: string;
  trailing_icon?: string;
  required?: boolean;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Form field: label above input box. Matches spec requirements:
 *   - DESIGN_GUIDELINES: "FORMS: ALL inputs AND primary button MUST
 *     use width='fill_container'. Vertical layout, gap=16-20."
 *   - ROLE_GUIDE: "input → height:48, layout:horizontal, padding:[12,16]"
 *   - "form-input → same as input + width:fill_container"
 *   - Affordance icons (from DESIGN_GUIDELINES):
 *       search→leading SearchIcon, password→trailing EyeIcon,
 *       email→leading MailIcon
 *     This tool accepts explicit `leading_icon` / `trailing_icon` so
 *     callers pick the right affordance; no type enum (keeps schema
 *     narrow per "应拆尽拆").
 *
 * Required marker: pass required=true and "*" is appended to the label.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddFormFieldV0(
  params: AddFormFieldV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const labelText = params.required ? `${params.label} *` : params.label;
  const inputChildren: Record<string, unknown>[] = [];
  if (params.leading_icon) {
    inputChildren.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  inputChildren.push({
    type: 'text',
    name: 'Placeholder',
    content: params.placeholder ?? '',
    fontSize: 14,
    fontWeight: 400,
  });
  if (params.trailing_icon) {
    inputChildren.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  const field = {
    type: 'frame',
    name: 'Form Field',
    role: 'form-field',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: labelText,
        fontSize: 14,
        fontWeight: 500,
      },
      {
        type: 'frame',
        name: 'Input',
        role: 'form-input',
        width: 'fill_container',
        height: 48,
        cornerRadius: 8,
        layout: 'horizontal',
        alignItems: 'center',
        gap: 8,
        padding: [12, 16],
        children: inputChildren,
      },
    ],
  };
  assignIdsRecursively(field);
  return insertElementTree({ binding: 'field', tree: field, ...params });
}
