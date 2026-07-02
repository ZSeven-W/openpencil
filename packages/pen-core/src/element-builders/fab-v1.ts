import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface FabV1Params {
  icon: string;
  size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_fab_v0.
   * - `'dark'`: accent bg + white icon (accent is brand-invariant, white on accent is brand decision).
   * - `'system'`: emits `$color-accent` ref for the button fill.
   */
  theme?: V1Theme;
}

/**
 * Floating action button — theme-aware version of buildFab.
 * Light mode is byte-equal to add_fab_v0.
 *
 * Color mapping:
 *   FAB bg (#2563EB)             → accent (brand-primary, same visual in all themes)
 *   icon fg (#FFFFFF)            → kept as-is (white on accent — brand decision)
 */
export function buildFabV1(params: FabV1Params): ElementTree {
  const size = params.size ?? 56;
  const iconSize = Math.round(size * 0.43);
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // accent is brand-primary — same visual intent in all modes
  const fabBg = isLight ? '#2563EB' : t.colors.accent;

  return {
    type: 'frame',
    name: 'FAB',
    role: 'fab',
    width: size,
    height: size,
    cornerRadius: size / 2,
    fill: [{ type: 'solid', color: fabBg }],
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: params.icon,
        iconFontFamily: 'lucide',
        width: iconSize,
        height: iconSize,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
      },
    ],
  };
}
