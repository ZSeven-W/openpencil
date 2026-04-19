import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddCheckboxV0Params {
  label: string;
  checked?: boolean;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Checkbox + label pair. Box is 20×20, cornerRadius=4.
 * checked=true → filled with primary-ish color + interior `check` icon.
 * checked=false → empty box with 1.5 stroke.
 *
 * roles: 'checkbox-row' / 'checkbox' | 'checkbox-checked' / 'label'.
 */
export async function handleAddCheckboxV0(
  params: AddCheckboxV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const checked = params.checked === true;
  const box: Record<string, unknown> = {
    type: 'frame',
    name: checked ? 'Checkbox (checked)' : 'Checkbox',
    role: checked ? 'checkbox-checked' : 'checkbox',
    width: 20,
    height: 20,
    cornerRadius: 4,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [],
  };
  if (checked) {
    box.fill = [{ type: 'solid', color: '#2563EB' }];
    (box.children as Record<string, unknown>[]).push({
      type: 'icon_font',
      name: 'Check',
      iconFontName: 'check',
      iconFontFamily: 'lucide',
      width: 14,
      height: 14,
    });
  } else {
    box.fill = [];
    box.stroke = { thickness: 1.5, fill: [{ type: 'solid', color: '#9CA3AF' }] };
  }
  const row = {
    type: 'frame',
    name: `Checkbox Row (${params.label})`,
    role: 'checkbox-row',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children: [
      box,
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: params.label,
        fontSize: 14,
        fontWeight: 400,
      },
    ],
  };
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'checkbox', tree: row, ...params });
}
