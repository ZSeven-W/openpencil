import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface PhoneInputV1Params {
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
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_phone_input_v0.
   * - `'dark'`: field bg → surface, stroke → border, divider → border,
   *   label → textBody, code text → textPrimary, chevron → textMuted,
   *   placeholder → textSubtle, filled value → textPrimary.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

const FIELD_HEIGHT = 44;

/**
 * Phone-number input (v1) — theme-aware variant of buildPhoneInput.
 * Light mode is byte-equal to add_phone_input_v0.
 *
 * Color mapping:
 *   field bg       (#FFFFFF)            → surface
 *   stroke         (#CBD5E1 slate-300)  → border
 *   divider        (#E2E8F0 slate-200)  → border
 *   label text     (#334155 slate-700)  → textBody
 *   code text      (#0F172A slate-950)  → textPrimary
 *   chevron        (#64748B slate-500)  → textMuted
 *   placeholder    (#94A3B8 slate-400)  → textSubtle
 *   filled value   (#0F172A slate-950)  → textPrimary
 */
export function buildPhoneInputV1(params: PhoneInputV1Params): ElementTree {
  const width = Math.max(240, Math.floor(params.width ?? 320));
  const code = params.country_code ?? '+1';
  const placeholder = params.placeholder ?? '(555) 555-5555';
  const isFilled = params.value !== undefined && params.value !== '';
  const digitsContent = isFilled ? params.value! : placeholder;

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const fieldBg = isLight ? '#FFFFFF' : t.colors.surface;
  const strokeColor = isLight ? '#CBD5E1' : t.colors.border;
  const dividerColor = isLight ? '#E2E8F0' : t.colors.border;
  const labelColor = isLight ? '#334155' : t.colors.textBody;
  const codeColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const chevronColor = isLight ? '#64748B' : t.colors.textMuted;
  const digitsColor = isFilled
    ? isLight
      ? '#0F172A'
      : t.colors.textPrimary
    : isLight
      ? '#94A3B8'
      : t.colors.textSubtle;

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
    fill: [{ type: 'solid', color: codeColor }],
  });
  countryChildren.push({
    type: 'icon_font',
    name: 'Chevron',
    role: 'phone-input-chevron',
    iconFontName: 'chevron-down',
    iconFontFamily: 'lucide',
    width: 14,
    height: 14,
    fill: [{ type: 'solid', color: chevronColor }],
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
    fill: [{ type: 'solid', color: fieldBg }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: strokeColor }] },
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
        fill: [{ type: 'solid', color: dividerColor }],
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
      fill: [{ type: 'solid', color: labelColor }],
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
