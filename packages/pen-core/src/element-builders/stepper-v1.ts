import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface StepperV1Params {
  total: number;
  current?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_stepper_v0.
   * - `'dark'`: pending circle fill → border (#334155); pending number → textMuted;
   *   pending connector → border. Accent (#2563EB) and white (#FFFFFF on done)
   *   stay hardcoded — brand/UI constants.
   * - `'system'`: $color-* refs for pending fill, pending number color, pending connector.
   */
  theme?: V1Theme;
}

/**
 * Stepper (v1) — theme-aware variant of buildStepper.
 * Light mode is byte-equal to add_stepper_v0.
 *
 * Color mapping:
 *   done circle fill      (#2563EB blue-600)  → kept hardcoded (accent/brand)
 *   done number text      (#FFFFFF white)      → kept hardcoded (on-accent)
 *   done connector        (#2563EB blue-600)  → kept hardcoded (accent/brand)
 *   pending circle fill   (#E5E7EB gray-200)  → border token
 *   pending number text   (#6B7280 gray-500)  → textMuted token
 *   pending connector     (#E5E7EB gray-200)  → border token
 */
export function buildStepperV1(params: StepperV1Params): ElementTree {
  const total = Math.max(1, Math.floor(params.total));
  const current = Math.max(0, Math.min(total - 1, Math.floor(params.current ?? 0)));
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Pending circle fill: light → gray-200 (#E5E7EB), dark/system → border
  const pendingFill = isLight ? '#E5E7EB' : t.colors.border;
  // Pending number text: light → gray-500 (#6B7280), dark/system → textMuted
  const pendingText = isLight ? '#6B7280' : t.colors.textMuted;

  const children: ElementTree[] = [];
  for (let i = 0; i < total; i += 1) {
    const done = i <= current;
    children.push({
      type: 'frame',
      name: `Step ${i + 1}`,
      role: done ? 'step-active' : 'step',
      width: 24,
      height: 24,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: done ? '#2563EB' : pendingFill }],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'Number',
          content: String(i + 1),
          fontSize: 13,
          fontWeight: 600,
          fill: [{ type: 'solid', color: done ? '#FFFFFF' : pendingText }],
        },
      ],
    });
    if (i < total - 1) {
      const doneConnector = i < current;
      children.push({
        type: 'rectangle',
        name: `Connector ${i}`,
        role: doneConnector ? 'step-connector-active' : 'step-connector',
        width: 'fill_container',
        height: 2,
        fill: [{ type: 'solid', color: doneConnector ? '#2563EB' : pendingFill }],
      });
    }
  }
  return {
    type: 'frame',
    name: 'Stepper',
    role: 'stepper',
    width: 'fill_container',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 0,
    children,
  };
}
