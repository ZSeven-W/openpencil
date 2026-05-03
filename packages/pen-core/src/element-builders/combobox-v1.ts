import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ComboboxV1Option {
  label: string;
  /** Mark a row as the currently highlighted suggestion (surface2 bg). */
  highlighted?: boolean;
}

export interface ComboboxV1Params {
  /** Placeholder shown when value is empty. */
  placeholder?: string;
  /** Pre-filled query text. */
  value?: string;
  /** Suggestion rows shown in the open dropdown. Required (1+). */
  options: ComboboxV1Option[];
  /** Optional label above the field (e.g. "Country"). */
  label?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_combobox_v0.
   * - `'dark'`: dark surface/text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Combobox / autocomplete — theme-aware version of buildCombobox.
 * Light mode is byte-equal to add_combobox_v0.
 *
 * Color mapping:
 *   label text (#0F172A)       → textPrimary
 *   input bg (#FFFFFF)         → surface
 *   input stroke (#2563EB)     → accent (focus ring — intentionally accent in all modes)
 *   search icon (#64748B)      → textMuted
 *   value text (#0F172A)       → textPrimary
 *   placeholder (#94A3B8)      → textSubtle
 *   dropdown bg (#FFFFFF)      → surface
 *   dropdown border (#E2E8F0)  → border
 *   option hover bg (#F1F5F9)  → surface2
 *   option text (#0F172A)      → textPrimary
 *   shadow color (#0F172A1A)   → kept as-is (shadow is theme-agnostic in v0)
 */
export function buildComboboxV1(params: ComboboxV1Params): ElementTree {
  const placeholder = params.placeholder ?? 'Search...';
  const value = params.value ?? '';
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  const labelTextColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const inputBg = isLight ? '#FFFFFF' : t.colors.surface;
  const inputStroke = isLight ? '#2563EB' : t.colors.accent;
  const searchIconColor = isLight ? '#64748B' : t.colors.textMuted;
  const valueColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const placeholderColor = isLight ? '#94A3B8' : t.colors.textSubtle;
  const dropdownBg = isLight ? '#FFFFFF' : t.colors.surface;
  const dropdownBorder = isLight ? '#E2E8F0' : t.colors.border;
  const optionHoverBg = isLight ? '#F1F5F9' : t.colors.surface2;
  const optionTextColor = isLight ? '#0F172A' : t.colors.textPrimary;

  const stackChildren: ElementTree[] = [];
  if (params.label) {
    stackChildren.push({
      type: 'text',
      name: 'Label',
      role: 'combobox-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: labelTextColor }],
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
    fill: [{ type: 'solid', color: inputBg }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: inputStroke }] },
    children: [
      {
        type: 'icon_font',
        name: 'Search Icon',
        iconFontName: 'search',
        iconFontFamily: 'lucide',
        width: 16,
        height: 16,
        fill: [{ type: 'solid', color: searchIconColor }],
      },
      {
        type: 'text',
        name: 'Value',
        role: 'combobox-value',
        content: value || placeholder,
        fontSize: 14,
        fontWeight: 400,
        fill: [{ type: 'solid', color: value ? valueColor : placeholderColor }],
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
    fill: [{ type: 'solid', color: dropdownBg }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: dropdownBorder }] },
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
      ...(opt.highlighted ? { fill: [{ type: 'solid', color: optionHoverBg }] } : {}),
      children: [
        {
          type: 'text',
          name: 'Label',
          role: 'combobox-option-label',
          content: opt.label,
          fontSize: 14,
          fontWeight: opt.highlighted ? 500 : 400,
          fill: [{ type: 'solid', color: optionTextColor }],
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
