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
 * Horizontal top tabs with underline on the active tab. PenStroke only
 * supports `thickness: number | [number,number,number,number]` — no
 * per-side stroke rendering (paint-utils.ts::resolveStrokeWidth returns 0
 * for object shapes, thickness[0] for arrays). So the active underline is
 * a sibling rectangle, NOT a directional stroke. Each tab becomes a
 * vertical frame: [content wrapper with padding] + [underline rect when
 * active].
 *
 * roles: 'tabs' / 'tab' | 'tab-active' / 'tab-underline'.
 * Use for "tabs with underline", "secondary navigation".
 */
export async function handleAddTabsV0(
  params: AddTabsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tabs = params.items.map((item) => {
    const inner = {
      type: 'frame',
      name: 'Tab Content',
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
    const children: Record<string, unknown>[] = [inner];
    if (item.active) {
      children.push({
        type: 'rectangle',
        name: 'Underline',
        role: 'tab-underline',
        width: 'fill_container',
        height: 2,
        fill: [{ type: 'solid', color: '#2563EB' }],
      });
    }
    return {
      type: 'frame',
      name: `Tab (${item.label})`,
      role: item.active ? 'tab-active' : 'tab',
      layout: 'vertical',
      alignItems: 'stretch',
      children,
    };
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
