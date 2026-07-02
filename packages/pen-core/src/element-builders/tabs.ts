import type { ElementTree } from './helpers.js';

export interface TabsItem {
  label: string;
  active?: boolean;
}

export interface TabsParams {
  items: TabsItem[];
}

/**
 * Horizontal top tabs with underline on the active tab. Each tab
 * uses width=fill_container so the bar splits evenly — avoids the
 * fill_container-in-fit_content trap that blows up only the active
 * tab. Underline is a sibling rect (PenStroke lacks per-side).
 */
export function buildTabs(params: TabsParams): ElementTree {
  const tabs: ElementTree[] = params.items.map((item) => {
    const inner: ElementTree = {
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
    const children: ElementTree[] = [inner];
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
  return {
    type: 'frame',
    name: 'Tabs',
    role: 'tabs',
    width: 'fill_container',
    layout: 'horizontal',
    gap: 4,
    alignItems: 'stretch',
    children: tabs,
  };
}
