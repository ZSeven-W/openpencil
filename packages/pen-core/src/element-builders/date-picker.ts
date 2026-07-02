import type { ElementTree } from './helpers.js';

export interface DatePickerParams {
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
}

/**
 * Date picker CLOSED state — a labeled input that shows the selected
 * date text (or a placeholder) on the left and a small calendar icon
 * on the right. The actual OPEN calendar panel is `add_calendar_grid_v0`
 * (keep them as separate tools: closed-state often stands alone in a
 * form, and stacking them together would be an anti-pattern when the
 * design spec only wants the closed trigger).
 *
 * Structure:
 *   frame(vertical, gap=6, role='date-picker')
 *     ├ text(label [+ " *"], 13/500, role='date-picker-label')
 *     └ frame(horizontal, h=48, cornerRadius=8, stroke=#E2E8F0,
 *             padding=[0,12], justify=space-between, align=center,
 *             role='date-picker-input')
 *         ├ text(value OR placeholder, 15/400, role='date-picker-value' / 'date-picker-placeholder')
 *         └ frame(horizontal, gap=8, align=center)
 *             ├ icon_font('x', 16×16, muted)  ← only if clearable AND value present
 *             └ icon_font('calendar', 20×20, role='date-picker-icon')
 */
export function buildDatePicker(params: DatePickerParams): ElementTree {
  const labelText = params.required === true ? `${params.label} *` : params.label;
  const hasValue = typeof params.value === 'string' && params.value.length > 0;
  const shownText = hasValue ? (params.value as string) : (params.placeholder ?? 'Select date');
  const clearable = params.clearable === true;

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
      fill: [{ type: 'solid', color: '#94A3B8' }],
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
    fill: [{ type: 'solid', color: '#64748B' }],
  });

  return {
    type: 'frame',
    name: 'Date Picker',
    role: 'date-picker',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'date-picker-label',
        content: labelText,
        fontSize: 13,
        fontWeight: 500,
      },
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
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
        children: [
          {
            type: 'text',
            name: hasValue ? 'Value' : 'Placeholder',
            role: hasValue ? 'date-picker-value' : 'date-picker-placeholder',
            content: shownText,
            fontSize: 15,
            fontWeight: 400,
            fill: [{ type: 'solid', color: hasValue ? '#0F172A' : '#94A3B8' }],
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
