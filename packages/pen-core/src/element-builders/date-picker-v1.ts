import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface DatePickerV1Params {
  /** Field label shown above the input. Required. */
  label: string;
  /**
   * Selected date text (e.g. "Jan 15, 2026"). When omitted or empty,
   * the placeholder renders instead.
   */
  value?: string;
  /** Placeholder shown when `value` is empty. Default "Select date". */
  placeholder?: string;
  /** When true, appends " *" to the label. */
  required?: boolean;
  /**
   * When true, renders a small "X" clear affordance on the right
   * (before the calendar icon). Default false.
   */
  clearable?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_date_picker_v0.
   * - `'dark'`: dark surface/text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Date picker CLOSED state — theme-aware version of buildDatePicker.
 * Light mode is byte-equal to add_date_picker_v0.
 *
 * Color mapping:
 *   input bg (#FFFFFF)           → surface
 *   input stroke (#E2E8F0)       → border
 *   value text (#0F172A)         → textPrimary
 *   placeholder text (#94A3B8)   → textSubtle
 *   clear icon (#94A3B8)         → textSubtle
 *   calendar icon (#64748B)      → textMuted
 *   label text: no fill in v0 (inherits from parent) — null in light, textPrimary in dark/system
 */
export function buildDatePickerV1(params: DatePickerV1Params): ElementTree {
  const labelText = params.required === true ? `${params.label} *` : params.label;
  const hasValue = typeof params.value === 'string' && params.value.length > 0;
  const shownText = hasValue ? (params.value as string) : (params.placeholder ?? 'Select date');
  const clearable = params.clearable === true;
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const inputBg = isLight ? '#FFFFFF' : t.colors.surface;
  const inputStroke = isLight ? '#E2E8F0' : t.colors.border;
  const valueColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const placeholderColor = isLight ? '#94A3B8' : t.colors.textSubtle;
  const clearIconColor = isLight ? '#94A3B8' : t.colors.textSubtle;
  const calendarIconColor = isLight ? '#64748B' : t.colors.textMuted;

  const rightSide: ElementTree[] = [];
  if (clearable && hasValue) {
    rightSide.push({
      type: 'icon_font',
      name: 'Clear',
      role: 'date-picker-clear',
      iconFontName: 'x',
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
      fill: [{ type: 'solid', color: clearIconColor }],
    });
  }
  rightSide.push({
    type: 'icon_font',
    name: 'Calendar Icon',
    role: 'date-picker-icon',
    iconFontName: 'calendar',
    iconFontFamily: 'lucide',
    width: 20,
    height: 20,
    fill: [{ type: 'solid', color: calendarIconColor }],
  });

  // Label: v0 emits no fill (inherits from parent). Keep null for light.
  const labelNode: ElementTree = {
    type: 'text',
    name: 'Label',
    role: 'date-picker-label',
    content: labelText,
    fontSize: 13,
    fontWeight: 500,
  };
  if (!isLight) {
    labelNode.fill = [{ type: 'solid', color: t.colors.textPrimary }];
  }

  return {
    type: 'frame',
    name: 'Date Picker',
    role: 'date-picker',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: [
      labelNode,
      {
        type: 'frame',
        name: 'Input',
        role: 'date-picker-input',
        width: 'fill_container',
        height: 48,
        cornerRadius: 8,
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: [0, 12],
        fill: [{ type: 'solid', color: inputBg }],
        stroke: { thickness: 1, fill: [{ type: 'solid', color: inputStroke }] },
        children: [
          {
            type: 'text',
            name: hasValue ? 'Value' : 'Placeholder',
            role: hasValue ? 'date-picker-value' : 'date-picker-placeholder',
            content: shownText,
            fontSize: 15,
            fontWeight: 400,
            fill: [{ type: 'solid', color: hasValue ? valueColor : placeholderColor }],
          },
          {
            type: 'frame',
            name: 'Right',
            height: 'fit_content',
            layout: 'horizontal',
            alignItems: 'center',
            gap: 8,
            children: rightSide,
          },
        ],
      },
    ],
  };
}
