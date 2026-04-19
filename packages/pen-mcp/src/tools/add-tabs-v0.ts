import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddTabsV0Item {
  label: string;
  active?: boolean;
}

export interface AddTabsV0Params {
  items: AddTabsV0Item[];
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal top tabs with underline on the active tab. Each tab is
 * fit_content width, padding=[12,16]. Active tab gets a 2px bottom stroke
 * via directional `stroke.thickness={bottom:2}` (per Pencil divider pattern)
 * and fontWeight=600.
 *
 * roles: 'tabs' / 'tab' | 'tab-active'.
 * Use for "tabs with underline", "secondary navigation".
 */
export async function handleAddTabsV0(
  params: AddTabsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tabs = params.items.map((item) => {
    const tab: Record<string, unknown> = {
      type: 'frame',
      name: `Tab (${item.label})`,
      role: item.active ? 'tab-active' : 'tab',
      padding: [12, 16],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'Label',
          role: 'label',
          content: item.label,
          fontSize: 14,
          fontWeight: item.active ? 600 : 500,
        },
      ],
    };
    if (item.active) {
      tab.stroke = {
        thickness: { bottom: 2 },
        fill: [{ type: 'solid', color: '#2563EB' }],
      };
    }
    return tab;
  });
  const bar = {
    type: 'frame',
    name: 'Tabs',
    role: 'tabs',
    width: 'fill_container',
    layout: 'horizontal',
    gap: 4,
    alignItems: 'stretch',
    children: tabs,
  };
  assignIdsRecursively(bar);
  return insertElementTree({ binding: 'tabs', tree: bar, ...params });
}
