import type { ElementTree } from './helpers.js';

export interface SegmentedControlItem {
  label: string;
  active?: boolean;
}

export interface SegmentedControlParams {
  items: SegmentedControlItem[];
}

/**
 * iOS pill-style segmented control. 32px high container with
 * cornerRadius=8 + gray-100 fill; each segment fills equal width
 * (overflow-safe). Active segment floats white on top.
 */
export function buildSegmentedControl(params: SegmentedControlParams): ElementTree {
  const segments: ElementTree[] = params.items.map((item) => {
    const seg: ElementTree = {
      type: 'frame',
      name: `Segment (${item.label})`,
      role: item.active ? 'segment-active' : 'segment',
      width: 'fill_container',
      height: 'fill_container',
      cornerRadius: 6,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'Label',
          role: 'label',
          content: item.label,
          fontSize: 13,
          fontWeight: item.active ? 600 : 500,
          fill: [{ type: 'solid', color: item.active ? '#111827' : '#4B5563' }],
        },
      ],
    };
    seg.fill = item.active ? [{ type: 'solid', color: '#FFFFFF' }] : [];
    return seg;
  });
  return {
    type: 'frame',
    name: 'Segmented Control',
    role: 'segmented-control',
    width: 'fill_container',
    height: 32,
    cornerRadius: 8,
    fill: [{ type: 'solid', color: '#F3F4F6' }],
    layout: 'horizontal',
    alignItems: 'stretch',
    gap: 4,
    padding: [4],
    children: segments,
  };
}
