import type { ElementTree } from './helpers.js';

export interface SelectParams {
  label: string;
  /** Currently selected value. Shows in the input area. Omit to show placeholder. */
  value?: string;
  /** Placeholder shown when no value is selected (gray). Default "Select…". */
  placeholder?: string;
  /** Trailing affordance icon name. Default "chevron-down" (closed state). */
  trailing_icon?: string;
  required?: boolean;
}

/**
 * Dropdown select — CLOSED/display state only. Same label-above-
 * input pattern as form-field with:
 *   - no leading icon slot (not typical for selects)
 *   - trailing chevron-down icon (always present; override via
 *     trailing_icon if the caller wants a custom affordance)
 *   - when `value` is set: input text renders black (fontWeight=400)
 *     as a normal filled input
 *   - when `value` is absent: input text renders the placeholder
 *     and is styled gray (color #94A3B8) so users see it as a hint
 *
 * The OPEN state (menu + options list) is NOT modeled here. An open
 * dropdown is a different design concern (absolute positioning, scrim
 * layer, per-option hover/selected) that belongs in its own builder
 * if/when needed. Users compose an open state via batch_design.
 */
export function buildSelect(params: SelectParams): ElementTree {
  const labelText = params.required ? `${params.label} *` : params.label;
  const placeholder = params.placeholder ?? 'Select…';
  const trailingIcon = params.trailing_icon ?? 'chevron-down';
  const displayText = params.value ?? placeholder;
  const isEmpty = params.value === undefined;

  const textNode: ElementTree = {
    type: 'text',
    name: isEmpty ? 'Placeholder' : 'Selected Value',
    content: displayText,
    fontSize: 14,
    fontWeight: 400,
  };
  if (isEmpty) {
    // Placeholder gray color — same token used by form-field's placeholder
    textNode.fill = [{ type: 'solid', color: '#94A3B8' }];
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
