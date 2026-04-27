import type { ElementTree } from './helpers.js';

export interface PhoneInputParams {
  /** Optional label above the input (e.g. "Phone number"). */
  label?: string;
  /** Country code shown in the leading button. Default "+1". */
  country_code?: string;
  /** Optional flag emoji or country abbreviation shown next to the dial code. */
  country_flag?: string;
  /** Placeholder for the digits input. Default "(555) 555-5555". */
  placeholder?: string;
  /**
   * Pre-filled phone digits value (without country code). When set,
   * renders as the populated state (slate-900 text); when omitted,
   * renders as placeholder state (slate-400 text).
   */
  value?: string;
  /** When true, appends " *" to the label. */
  required?: boolean;
  /** Total field width in px. Default 320. Min 240. */
  width?: number;
}

const FIELD_HEIGHT = 44;

/**
 * Phone-number input with leading country-code selector — the
 * "+1 (555) …" pattern from every modern signup / login screen.
 * Different from `add_form_field_v0` (single text input, no
 * prefix); use this when the spec calls for an international
 * phone field with country selector.
 *
 * Structure:
 *   frame(width, fit_content, vertical, gap=6, role='phone-input-field')
 *     ├ text(label, 13/500, role='form-label')                    ← if label
 *     └ frame(horizontal, height=44, cornerRadius=10, stroke=slate-300, role='phone-input-row')
 *         ├ frame(country selector, fixed-width, role='phone-input-country')
 *         │   ├ text(flag, 16, role='phone-input-flag')           ← if flag
 *         │   ├ text(code, 14/500, role='phone-input-code')
 *         │   └ icon_font(chevron-down, 14, role='phone-input-chevron')
 *         ├ rectangle(divider, w=1, fill_container_height, role='phone-input-divider')
 *         └ frame(digits, fill_container, role='phone-input-digits')
 *             └ text(value-or-placeholder, 14/400, role='phone-input-digits-text')
 *
 * Country selector renders as a button-shape (no actual menu); the
 * caller is expected to handle the picker UX as a separate concern.
 */
export function buildPhoneInput(params: PhoneInputParams): ElementTree {
  const width = Math.max(240, Math.floor(params.width ?? 320));
  const code = params.country_code ?? '+1';
  const placeholder = params.placeholder ?? '(555) 555-5555';
  const isFilled = params.value !== undefined && params.value !== '';
  const digitsContent = isFilled ? params.value! : placeholder;
  const digitsColor = isFilled ? '#0F172A' : '#94A3B8';

  const countryChildren: ElementTree[] = [];
  if (params.country_flag) {
    countryChildren.push({
      type: 'text',
      name: 'Flag',
      role: 'phone-input-flag',
      content: params.country_flag,
      fontSize: 16,
      fontWeight: 400,
    });
  }
  countryChildren.push({
    type: 'text',
    name: 'Code',
    role: 'phone-input-code',
    content: code,
    fontSize: 14,
    fontWeight: 500,
    fill: [{ type: 'solid', color: '#0F172A' }],
  });
  countryChildren.push({
    type: 'icon_font',
    name: 'Chevron',
    role: 'phone-input-chevron',
    iconFontName: 'chevron-down',
    iconFontFamily: 'lucide',
    width: 14,
    height: 14,
    fill: [{ type: 'solid', color: '#64748B' }],
  });

  const inputRow: ElementTree = {
    type: 'frame',
    name: 'Input Row',
    role: 'phone-input-row',
    width: 'fill_container',
    height: FIELD_HEIGHT,
    cornerRadius: 10,
    layout: 'horizontal',
    alignItems: 'center',
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#CBD5E1' }] },
    children: [
      {
        type: 'frame',
        name: 'Country',
        role: 'phone-input-country',
        width: 'fit_content',
        height: 'fill_container',
        layout: 'horizontal',
        alignItems: 'center',
        gap: 6,
        padding: [0, 12, 0, 14],
        children: countryChildren,
      },
      {
        type: 'rectangle',
        name: 'Divider',
        role: 'phone-input-divider',
        width: 1,
        height: 28,
        fill: [{ type: 'solid', color: '#E2E8F0' }],
      },
      {
        type: 'frame',
        name: 'Digits',
        role: 'phone-input-digits',
        width: 'fill_container',
        height: 'fill_container',
        layout: 'horizontal',
        alignItems: 'center',
        padding: [0, 14],
        children: [
          {
            type: 'text',
            name: 'Digits Text',
            role: 'phone-input-digits-text',
            content: digitsContent,
            fontSize: 14,
            fontWeight: 400,
            fill: [{ type: 'solid', color: digitsColor }],
          },
        ],
      },
    ],
  };

  const children: ElementTree[] = [];
  if (params.label) {
    const labelText = params.required ? `${params.label} *` : params.label;
    children.push({
      type: 'text',
      name: 'Label',
      role: 'form-label',
      content: labelText,
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#334155' }],
    });
  }
  children.push(inputRow);

  return {
    type: 'frame',
    name: 'Phone Input Field',
    role: 'phone-input-field',
    width,
    height: 'fit_content',
    layout: 'vertical',
    gap: 6,
    children,
  };
}
