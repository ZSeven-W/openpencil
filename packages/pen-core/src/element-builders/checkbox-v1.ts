import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface CheckboxV1Params {
  label: string;
  checked?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_checkbox_v0.
   * - `'dark'`: accent fill for checked, dark border for unchecked.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Checkbox + label pair — theme-aware version of buildCheckbox.
 * Light mode is byte-equal to add_checkbox_v0.
 *
 * Checked state: accent fill + white check icon.
 * Unchecked state: transparent fill + border stroke (border token).
 * v0's unchecked stroke was #9CA3AF (gray-400); mapped to border token in dark/system.
 */
export function buildCheckboxV1(params: CheckboxV1Params): ElementTree {
  const checked = params.checked === true;
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  // Light mode: byte-parity with v0
  const accentColor = isLight ? '#2563EB' : t.colors.accent;
  const borderColor = isLight ? '#9CA3AF' : t.colors.border;

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
    box.fill = [{ type: 'solid', color: accentColor }];
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
    box.stroke = { thickness: 1.5, fill: [{ type: 'solid', color: borderColor }] };
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
