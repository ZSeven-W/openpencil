import type { ElementTree } from './helpers.js';

export interface ChipInputParams {
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
}

/**
 * Chip / tag input — a labeled form control that holds a variable
 * number of pill-shaped tags plus an inline text cursor. The classic
 * "add emails to a recipient list" / "tag this post" / "multi-select
 * categories" widget.
 *
 * Structure:
 *   frame(vertical, gap=6, role='chip-input', fill_container)
 *     ├ text(label [ + " *"], 13/500, role='chip-input-label')
 *     └ frame(horizontal wrap, gap=6, padding=[8,12], cornerRadius=8,
 *             stroke={#E2E8F0,1}, fill_container, role='chip-input-field')
 *         ├ frame(pill, cornerRadius=16, padding=[6,4,6,10], bg=#F1F5F9,
 *                 gap=4, role='chip') — for each chip
 *         │   ├ text(chip value, 13/500)
 *         │   └ icon_font('x', 14×14, muted)
 *         └ text(placeholder OR "", 14/400, #94A3B8, role='chip-input-caret')
 *
 * `wrap: true` on the field is INTENTIONAL — the whole point of a
 * chip input is to flow onto multiple rows as chips accumulate. Pen's
 * horizontal layout with wrap supports this natively.
 */
export function buildChipInput(params: ChipInputParams): ElementTree {
  const labelText = params.required === true ? `${params.label} *` : params.label;
  const chips = params.chips ?? [];

  const fieldChildren: ElementTree[] = chips.map((value) => ({
    type: 'frame',
    name: `Chip (${value})`,
    role: 'chip',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    paddingTop: 6,
    paddingBottom: 6,
    paddingLeft: 10,
    paddingRight: 4,
    cornerRadius: 16,
    fill: [{ type: 'solid', color: '#F1F5F9' }],
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
        fill: [{ type: 'solid', color: '#64748B' }],
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
    fill: [{ type: 'solid', color: '#94A3B8' }],
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
        paddingTop: 8,
        paddingBottom: 8,
        paddingLeft: 12,
        paddingRight: 12,
        cornerRadius: 8,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
        children: fieldChildren,
      },
    ],
  };
}
