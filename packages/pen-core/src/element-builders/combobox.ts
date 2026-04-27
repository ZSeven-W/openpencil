import type { ElementTree } from './helpers.js';

export interface ComboboxOption {
  label: string;
  /** Mark a row as the currently highlighted suggestion (slate-100 bg). */
  highlighted?: boolean;
}

export interface ComboboxParams {
  /** Placeholder shown when value is empty. */
  placeholder?: string;
  /** Pre-filled query text. */
  value?: string;
  /** Suggestion rows shown in the open dropdown. Required (1+). */
  options: ComboboxOption[];
  /** Optional label above the field (e.g. "Country"). */
  label?: string;
}

/**
 * Combobox / autocomplete — open-state input with a dropdown of
 * suggestion rows below. Distinct from `add_select_v0` (CLOSED-state
 * dropdown — value + chevron only) and `add_search_bar_v0` (no
 * suggestion list). Use this when the AI must show the open dropdown
 * with results, e.g. country picker mid-typing, command palette,
 * autocomplete search.
 */
export function buildCombobox(params: ComboboxParams): ElementTree {
  const placeholder = params.placeholder ?? 'Search...';
  const value = params.value ?? '';
  const stackChildren: ElementTree[] = [];
  if (params.label) {
    stackChildren.push({
      type: 'text',
      name: 'Label',
      role: 'combobox-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#0F172A' }],
    });
  }
  stackChildren.push({
    type: 'frame',
    name: 'Input',
    role: 'combobox-input',
    width: 'fill_container',
    height: 44,
    cornerRadius: 8,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    padding: [0, 12],
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#2563EB' }] },
    children: [
      {
        type: 'icon_font',
        name: 'Search Icon',
        iconFontName: 'search',
        iconFontFamily: 'lucide',
        width: 16,
        height: 16,
        fill: [{ type: 'solid', color: '#64748B' }],
      },
      {
        type: 'text',
        name: 'Value',
        role: 'combobox-value',
        content: value || placeholder,
        fontSize: 14,
        fontWeight: 400,
        fill: [{ type: 'solid', color: value ? '#0F172A' : '#94A3B8' }],
      },
    ],
  });
  stackChildren.push({
    type: 'frame',
    name: 'Dropdown',
    role: 'combobox-dropdown',
    width: 'fill_container',
    height: 'fit_content',
    cornerRadius: 8,
    layout: 'vertical',
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
    effects: [
      {
        type: 'shadow',
        offsetX: 0,
        offsetY: 4,
        blur: 12,
        spread: 0,
        color: '#0F172A1A',
      },
    ],
    children: params.options.map((opt, i) => ({
      type: 'frame',
      name: `Option ${i + 1}`,
      role: opt.highlighted ? 'combobox-option-active' : 'combobox-option',
      width: 'fill_container',
      height: 36,
      layout: 'horizontal',
      alignItems: 'center',
      padding: [0, 12],
      ...(opt.highlighted ? { fill: [{ type: 'solid', color: '#F1F5F9' }] } : {}),
      children: [
        {
          type: 'text',
          name: 'Label',
          role: 'combobox-option-label',
          content: opt.label,
          fontSize: 14,
          fontWeight: opt.highlighted ? 500 : 400,
          fill: [{ type: 'solid', color: '#0F172A' }],
        },
      ],
    })),
  });
  return {
    type: 'frame',
    name: 'Combobox',
    role: 'combobox',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: stackChildren,
  };
}
