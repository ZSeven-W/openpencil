import type { ElementTree } from './helpers.js';

export interface RadioParams {
  label: string;
  selected?: boolean;
}

/**
 * Radio button + label. Outer ring 20×20 cornerRadius=10. When
 * selected, inner dot 10×10 cornerRadius=5 centered.
 */
export function buildRadio(params: RadioParams): ElementTree {
  const selected = params.selected === true;
  const outer: ElementTree = {
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
    children: [] as ElementTree[],
  };
  if (selected) {
    (outer.children as ElementTree[]).push({
      type: 'frame',
      name: 'Dot',
      role: 'radio-dot',
      width: 10,
      height: 10,
      cornerRadius: 5,
      fill: [{ type: 'solid', color: '#2563EB' }],
    });
  }
  return {
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
}
