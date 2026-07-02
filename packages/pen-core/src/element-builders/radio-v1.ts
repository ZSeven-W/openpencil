import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface RadioV1Params {
  label: string;
  selected?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_radio_v0.
   * - `'dark'`: accent stays brand color; unselected ring → border token.
   * - `'system'`: accent ref for selected; border ref for unselected ring.
   */
  theme?: V1Theme;
}

/**
 * Radio button + label (v1) — theme-aware variant of buildRadio.
 * Light mode is byte-equal to add_radio_v0.
 *
 * Color mapping:
 *   accent fill  (#2563EB) — brand-invariant, kept across themes
 *   unselected ring stroke (#9CA3AF gray-400) → border token
 */
export function buildRadioV1(params: RadioV1Params): ElementTree {
  const selected = params.selected === true;
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Accent is brand-invariant
  const accentColor = '#2563EB';
  // Unselected ring: light → gray-400 (#9CA3AF), dark/system → border
  const ringColor = isLight ? '#9CA3AF' : t.colors.border;

  const outer: ElementTree = {
    type: 'frame',
    name: selected ? 'Radio (selected)' : 'Radio',
    role: selected ? 'radio-selected' : 'radio',
    width: 20,
    height: 20,
    cornerRadius: 10,
    fill: [],
    stroke: {
      thickness: 1.5,
      fill: [{ type: 'solid', color: selected ? accentColor : ringColor }],
    },
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [] as ElementTree[],
  };
  if (selected) {
    (outer.children as ElementTree[]).push({
      type: 'frame',
      name: 'Dot',
      role: 'radio-dot',
      width: 10,
      height: 10,
      cornerRadius: 5,
      fill: [{ type: 'solid', color: accentColor }],
    });
  }
  return {
    type: 'frame',
    name: `Radio Row (${params.label})`,
    role: 'radio-row',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children: [
      outer,
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: params.label,
        fontSize: 14,
        fontWeight: 400,
      },
    ],
  };
}
