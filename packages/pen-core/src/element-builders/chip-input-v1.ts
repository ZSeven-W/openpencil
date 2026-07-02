import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ChipInputV1Params {
  /** Field label shown above the input. Required. */
  label: string;
  /**
   * Current chip values. Each becomes a pill with a small × icon.
   * Empty array is valid (placeholder-only state).
   */
  chips?: string[];
  /** Placeholder text shown when no chips OR after the last chip. */
  placeholder?: string;
  /** When true, appends " *" to the label. */
  required?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_chip_input_v0.
   * - `'dark'`: dark fills for chip bg, field bg, border, icon, placeholder.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Chip / tag input — theme-aware version of buildChipInput.
 * Light mode is byte-equal to add_chip_input_v0.
 *
 * Color mapping:
 *   chip bg (#F1F5F9)   → surface2
 *   chip icon (#64748B) → textMuted
 *   field bg (#FFFFFF)  → surface
 *   field border (#E2E8F0) → border
 *   placeholder (#94A3B8) → textSubtle
 */
export function buildChipInputV1(params: ChipInputV1Params): ElementTree {
  const labelText = params.required === true ? `${params.label} *` : params.label;
  const chips = params.chips ?? [];
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  const chipBg = isLight ? '#F1F5F9' : t.colors.surface2;
  const chipIconColor = isLight ? '#64748B' : t.colors.textMuted;
  const fieldBg = isLight ? '#FFFFFF' : t.colors.surface;
  const fieldBorder = isLight ? '#E2E8F0' : t.colors.border;
  const placeholderColor = isLight ? '#94A3B8' : t.colors.textSubtle;

  const fieldChildren: ElementTree[] = chips.map((value) => ({
    type: 'frame',
    name: `Chip (${value})`,
    role: 'chip',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    padding: [6, 4, 6, 10],
    cornerRadius: 16,
    fill: [{ type: 'solid', color: chipBg }],
    children: [
      {
        type: 'text',
        name: 'Value',
        content: value,
        fontSize: 13,
        fontWeight: 500,
      },
      {
        type: 'icon_font',
        name: 'Remove',
        iconFontName: 'x',
        iconFontFamily: 'lucide',
        width: 14,
        height: 14,
        fill: [{ type: 'solid', color: chipIconColor }],
      },
    ],
  }));

  fieldChildren.push({
    type: 'text',
    name: 'Caret',
    role: 'chip-input-caret',
    content: params.placeholder ?? (chips.length === 0 ? 'Add tag…' : ''),
    fontSize: 14,
    fontWeight: 400,
    fill: [{ type: 'solid', color: placeholderColor }],
  });

  return {
    type: 'frame',
    name: 'Chip Input',
    role: 'chip-input',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'chip-input-label',
        content: labelText,
        fontSize: 13,
        fontWeight: 500,
      },
      {
        type: 'frame',
        name: 'Field',
        role: 'chip-input-field',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'horizontal',
        alignItems: 'center',
        layoutWrap: 'wrap',
        gap: 6,
        padding: [8, 12],
        cornerRadius: 8,
        fill: [{ type: 'solid', color: fieldBg }],
        stroke: { thickness: 1, fill: [{ type: 'solid', color: fieldBorder }] },
        children: fieldChildren,
      },
    ],
  };
}
