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
 * `width=fill_container` so the bar splits evenly across all tabs — this
 * matches the common Twitter/Material underline-tabs pattern AND avoids
 * a layout-engine trap: a `fill_container` child inside a `fit_content`
 * parent resolves to the grandparent's width (pen-core engine.ts:182-187),
 * which would blow up only the active tab.
 *
 * The underline is a sibling rectangle (NOT a directional stroke — PenStroke
 * doesn't support per-side rendering; see project_stroke_contract.md).
 * Each tab is a vertical frame: [padded content wrapper] + [underline rect
 * when active].
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
      width: 'fill_container',
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
      width: 'fill_container',
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
