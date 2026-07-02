import type { ElementTree } from './helpers.js';

export interface LegendItemParams {
  /** Label text (e.g. "Revenue", "Expenses"). */
  label: string;
  /** Marker fill hex (e.g. "#2563EB"). */
  color: string;
  /** Optional value shown to the right of the label (e.g. "$12,480"). */
  value?: string;
}

/**
 * Chart legend entry — colored marker + label (+ optional right-aligned
 * value). Pair multiple of these under a vertical or horizontal parent
 * to build a full legend group. Distinct from `add_status_badge_v0`
 * (semantic "● Online" status, fixed tone enum) and `add_color_swatch_v0`
 * (a single big square + token label, design-system context).
 */
export function buildLegendItem(params: LegendItemParams): ElementTree {
  const children: ElementTree[] = [
    {
      type: 'frame',
      name: 'Marker',
      role: 'legend-item-marker',
      width: 10,
      height: 10,
      cornerRadius: 2,
      fill: [{ type: 'solid', color: params.color }],
      children: [],
    },
    {
      type: 'text',
      name: 'Label',
      role: 'legend-item-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 400,
      fill: [{ type: 'solid', color: '#475569' }],
    },
  ];
  if (params.value) {
    children.push({
      type: 'text',
      name: 'Value',
      role: 'legend-item-value',
      content: params.value,
      fontSize: 13,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#0F172A' }],
    });
  }
  return {
    type: 'frame',
    name: 'Legend Item',
    role: 'legend-item',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children,
  };
}
