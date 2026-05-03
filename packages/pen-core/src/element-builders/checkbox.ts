import type { ElementTree } from './helpers.js';

export interface CheckboxParams {
  label: string;
  checked?: boolean;
}

/**
 * Checkbox + label pair. 20×20 box, cornerRadius=4. checked=true
 * → primary fill + interior `check` icon; checked=false → empty
 * with 1.5px stroke.
 */
export function buildCheckbox(params: CheckboxParams): ElementTree {
  const checked = params.checked === true;
  const box: ElementTree = {
    type: 'frame',
    name: checked ? 'Checkbox (checked)' : 'Checkbox',
    role: checked ? 'checkbox-checked' : 'checkbox',
    width: 20,
    height: 20,
    cornerRadius: 4,
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [] as ElementTree[],
  };
  if (checked) {
    box.fill = [{ type: 'solid', color: '#2563EB' }];
    (box.children as ElementTree[]).push({
      type: 'icon_font',
      name: 'Check',
      iconFontName: 'check',
      iconFontFamily: 'lucide',
      width: 14,
      height: 14,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    });
  } else {
    box.fill = [];
    box.stroke = { thickness: 1.5, fill: [{ type: 'solid', color: '#9CA3AF' }] };
  }
  return {
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
}
