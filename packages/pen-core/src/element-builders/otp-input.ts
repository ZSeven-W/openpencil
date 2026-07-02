import type { ElementTree } from './helpers.js';

export interface OtpInputParams {
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
}

/**
 * OTP / PIN code input — horizontal row of N square slots, each
 * holding one digit. Common in 2FA verification and PIN unlock
 * screens. Renders the caller-chosen state:
 *
 *   - blank: all N slots empty (awaiting input)
 *   - partial: first M slots filled, the focused one outlined, the
 *     rest empty
 *   - full: all N slots filled (final submittable state)
 *
 * Structure:
 *   frame(horizontal, gap, role='otp-input', fit_content)
 *     └ frame(slot_size × slot_size, cornerRadius=8, border,
 *             role='otp-slot' | 'otp-slot-focused' | 'otp-slot-filled',
 *             layout=horizontal, center/center)
 *         └ text(digit, 20/600) [only when filled]
 *
 * Focused slot gets the accent-color border; filled slots get a
 * solid slate-700 border; empty unfocused slots get a slate-300
 * border. Caller composes this with a labeled form wrapper if
 * needed (v0 purposefully does NOT embed the "Enter code" label
 * — callers vary widely).
 */
export function buildOtpInput(params: OtpInputParams): ElementTree {
  const length = Math.max(4, Math.min(8, Math.floor(params.length ?? 6)));
  const digits = params.digits ?? [];
  const focusedIndex = Math.max(0, Math.min(length - 1, Math.floor(params.focused_index ?? 0)));
  const slotSize = Math.max(32, Math.min(80, Math.floor(params.slot_size ?? 48)));
  const gap = Math.max(0, Math.min(24, Math.floor(params.gap ?? 12)));
  const accent = params.accent_color ?? '#2563EB';

  const children: ElementTree[] = [];
  for (let i = 0; i < length; i += 1) {
    const digit = digits[i];
    const isFilled = typeof digit === 'string' && digit.length > 0;
    const isFocused = i === focusedIndex && !isFilled;

    const borderColor = isFocused ? accent : isFilled ? '#334155' : '#CBD5E1';
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
            fill: [{ type: 'solid', color: '#0F172A' }],
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
      fill: [{ type: 'solid', color: '#FFFFFF' }],
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
