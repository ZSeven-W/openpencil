import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddRadioV0Params {
  label: string;
  selected?: boolean;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Radio button + label. Outer ring 20×20 cornerRadius=10. When selected,
 * inner dot 10×10 cornerRadius=5 (centered via justifyContent+alignItems).
 *
 * roles: 'radio-row' / 'radio' | 'radio-selected' / 'label'.
 */
export async function handleAddRadioV0(
  params: AddRadioV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const selected = params.selected === true;
  const outer: Record<string, unknown> = {
    type: 'frame',
    name: selected ? 'Radio (selected)' : 'Radio',
    role: selected ? 'radio-selected' : 'radio',
    width: 20,
    height: 20,
    cornerRadius: 10,
    fill: [],
    stroke: {
      thickness: 1.5,
      fill: [{ type: 'solid', color: selected ? '#2563EB' : '#9CA3AF' }],
    },
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [],
  };
  if (selected) {
    (outer.children as Record<string, unknown>[]).push({
      type: 'frame',
      name: 'Dot',
      role: 'radio-dot',
      width: 10,
      height: 10,
      cornerRadius: 5,
      fill: [{ type: 'solid', color: '#2563EB' }],
    });
  }
  const row = {
    type: 'frame',
    name: `Radio Row (${params.label})`,
    role: 'radio-row',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children: [
      outer,
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
  return insertElementTree({ binding: 'radio', tree: row, ...params });
}
