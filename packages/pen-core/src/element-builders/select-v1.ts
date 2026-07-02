import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface SelectV1Params {
  label: string;
  /** Currently selected value. Shows in the input area. Omit to show placeholder. */
  value?: string;
  /** Placeholder shown when no value is selected. Default "Select…". */
  placeholder?: string;
  /** Trailing affordance icon name. Default "chevron-down". */
  trailing_icon?: string;
  required?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_select_v0.
   * - `'dark'`: placeholder color → textSubtle; input border → border token.
   * - `'system'`: $color-* refs for placeholder and border.
   */
  theme?: V1Theme;
}

/**
 * Dropdown select — closed/display state (v1). Theme-aware variant of buildSelect.
 * Light mode is byte-equal to add_select_v0.
 *
 * Color mapping:
 *   placeholder text (#94A3B8 slate-400) → textSubtle token
 *   (input border is absent in v0; kept absent in light for byte-parity)
 */
export function buildSelectV1(params: SelectV1Params): ElementTree {
  const labelText = params.required ? `${params.label} *` : params.label;
  const placeholder = params.placeholder ?? 'Select…';
  const trailingIcon = params.trailing_icon ?? 'chevron-down';
  const displayText = params.value ?? placeholder;
  const isEmpty = params.value === undefined;

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Placeholder color: light → slate-400 (#94A3B8), dark/system → textSubtle
  const placeholderColor = isLight ? '#94A3B8' : t.colors.textSubtle;

  const textNode: ElementTree = {
    type: 'text',
    name: isEmpty ? 'Placeholder' : 'Selected Value',
    content: displayText,
    fontSize: 14,
    fontWeight: 400,
  };
  if (isEmpty) {
    textNode.fill = [{ type: 'solid', color: placeholderColor }];
  }

  return {
    type: 'frame',
    name: 'Select',
    role: 'select',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: labelText,
        fontSize: 14,
        fontWeight: 500,
      },
      {
        type: 'frame',
        name: 'Input',
        role: 'select-input',
        width: 'fill_container',
        height: 48,
        cornerRadius: 8,
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'space_between',
        gap: 8,
        padding: [12, 16],
        children: [
          textNode,
          {
            type: 'icon_font',
            name: 'Trailing Icon',
            iconFontName: trailingIcon,
            iconFontFamily: 'lucide',
            width: 20,
            height: 20,
          },
        ],
      },
    ],
  };
}
