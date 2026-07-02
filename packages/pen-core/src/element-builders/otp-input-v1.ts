import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface OtpInputV1Params {
  /** Number of code digits (slots). Clamped 4..8. Default 6. */
  length?: number;
  /**
   * Optional digits to render inside filled slots. When provided,
   * `digits[i]` fills slot `i`; omitted / shorter-than-length
   * arrays leave the remaining slots empty. Pass an empty array
   * (or omit) to render the blank "awaiting input" state.
   */
  digits?: string[];
  /** Index of the currently-focused slot (0-based). Default 0. */
  focused_index?: number;
  /** Slot size in px (square). Default 48. Clamped 32..80. */
  slot_size?: number;
  /** Gap between slots. Default 12. Clamped 0..24. */
  gap?: number;
  /** Primary accent color for the focused-slot border. Default #2563EB. */
  accent_color?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_otp_input_v0.
   * - `'dark'`: slot bg → surface, filled border → borderStrong,
   *   empty border → border, digit text → textPrimary.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * OTP / PIN code input (v1) — theme-aware variant of buildOtpInput.
 * Light mode is byte-equal to add_otp_input_v0.
 *
 * Color mapping:
 *   slot bg         (#FFFFFF)          → surface
 *   filled border   (#334155 slate-700)→ borderStrong
 *   empty border    (#CBD5E1 slate-300)→ border
 *   digit text      (#0F172A slate-950)→ textPrimary
 *   focused border  = accent_color (brand-invariant, kept as-is)
 */
export function buildOtpInputV1(params: OtpInputV1Params): ElementTree {
  const length = Math.max(4, Math.min(8, Math.floor(params.length ?? 6)));
  const digits = params.digits ?? [];
  const focusedIndex = Math.max(0, Math.min(length - 1, Math.floor(params.focused_index ?? 0)));
  const slotSize = Math.max(32, Math.min(80, Math.floor(params.slot_size ?? 48)));
  const gap = Math.max(0, Math.min(24, Math.floor(params.gap ?? 12)));
  const accent = params.accent_color ?? '#2563EB';

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const slotBg = isLight ? '#FFFFFF' : t.colors.surface;
  const filledBorder = isLight ? '#334155' : t.colors.borderStrong;
  const emptyBorder = isLight ? '#CBD5E1' : t.colors.border;
  const digitColor = isLight ? '#0F172A' : t.colors.textPrimary;

  const children: ElementTree[] = [];
  for (let i = 0; i < length; i += 1) {
    const digit = digits[i];
    const isFilled = typeof digit === 'string' && digit.length > 0;
    const isFocused = i === focusedIndex && !isFilled;

    const borderColor = isFocused ? accent : isFilled ? filledBorder : emptyBorder;
    const role = isFocused ? 'otp-slot-focused' : isFilled ? 'otp-slot-filled' : 'otp-slot';

    const slotChildren: ElementTree[] = isFilled
      ? [
          {
            type: 'text',
            name: 'Digit',
            role: 'otp-digit',
            content: digit,
            fontSize: 20,
            fontWeight: 600,
            fill: [{ type: 'solid', color: digitColor }],
          },
        ]
      : [];

    children.push({
      type: 'frame',
      name: `Slot ${i + 1}`,
      role,
      width: slotSize,
      height: slotSize,
      cornerRadius: 8,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: [{ type: 'solid', color: slotBg }],
      stroke: {
        thickness: isFocused ? 2 : 1,
        fill: [{ type: 'solid', color: borderColor }],
      },
      children: slotChildren,
    });
  }

  return {
    type: 'frame',
    name: 'OTP Input',
    role: 'otp-input',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap,
    children,
  };
}
