import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface TabsV1Item {
  label: string;
  active?: boolean;
}

export interface TabsV1Params {
  items: TabsV1Item[];
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_tabs_v0.
   * - `'dark'`: identical — #2563EB underline is brand accent (spec §3.4).
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Horizontal top tabs with underline (v1) — theme-aware variant of buildTabs.
 * Light mode is byte-equal to add_tabs_v0.
 *
 * The only hardcoded color is #2563EB (accent blue underline).
 * Per spec §3.4, accent is a brand token, not a theme token — kept
 * hardcoded across all theme modes. All modes produce identical trees.
 */
export function buildTabsV1(params: TabsV1Params): ElementTree {
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
