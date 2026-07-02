import type { ElementTree } from './helpers.js';
import { type V1Theme } from './resolve-theme.js';

export interface FormFieldV1Params {
  label: string;
  placeholder?: string;
  leading_icon?: string;
  trailing_icon?: string;
  required?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_form_field_v0.
   * - `'dark'` / `'system'`: identical output (no hardcoded colors in v0).
   * Accepts theme param for API consistency across all v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Form field — theme-aware version of buildFormField.
 * No color fills are hardcoded in v0, so all three modes produce
 * identical output (byte-parity with v0 in all modes). The `theme`
 * parameter is accepted for API consistency.
 */
export function buildFormFieldV1(params: FormFieldV1Params): ElementTree {
  const labelText = params.required ? `${params.label} *` : params.label;
  const inputChildren: ElementTree[] = [];
  if (params.leading_icon) {
    inputChildren.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  inputChildren.push({
    type: 'text',
    name: 'Placeholder',
    content: params.placeholder ?? '',
    fontSize: 14,
    fontWeight: 400,
  });
  if (params.trailing_icon) {
    inputChildren.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  return {
    type: 'frame',
    name: 'Form Field',
    role: 'form-field',
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
        role: 'form-input',
        width: 'fill_container',
        height: 48,
        cornerRadius: 8,
        layout: 'horizontal',
        alignItems: 'center',
        gap: 8,
        padding: [12, 16],
        children: inputChildren,
      },
    ],
  };
}
