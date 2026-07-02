import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ProgressBarV1Params {
  value?: number;
  bar_width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_progress_bar_v0.
   * - `'dark'`: track → surface2 (#334155), fill → accent invariant (brand color).
   * - `'system'`: $color-* refs for track; accent ref for fill.
   */
  theme?: V1Theme;
}

/**
 * Linear progress bar (v1) — theme-aware variant of buildProgressBar.
 * Light mode is byte-equal to add_progress_bar_v0.
 *
 * Color mapping:
 *   fill color   (#2563EB accent) — brand-invariant, kept across themes
 *   track bg     (#E5E7EB gray-200) → surface2 token (secondary surface)
 */
export function buildProgressBarV1(params: ProgressBarV1Params): ElementTree {
  const raw = params.value ?? 50;
  const value = Math.max(0, Math.min(100, raw));
  const barWidth = params.bar_width ?? 240;
  const fillWidth = Math.max(0, Math.round((barWidth * value) / 100));

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Accent (fill color) is brand-invariant across all themes
  const fillColor = '#2563EB';
  // Track bg: light → gray-200 (#E5E7EB), dark/system → surface2
  const trackColor = isLight ? '#E5E7EB' : t.colors.surface2;

  const children: ElementTree[] = [];
  if (fillWidth > 0) {
    children.push({
      type: 'rectangle',
      name: 'Fill',
      role: 'progress-bar-fill',
      width: fillWidth,
      height: 8,
      cornerRadius: 4,
      fill: [{ type: 'solid', color: fillColor }],
    });
  }
  return {
    type: 'frame',
    name: 'Progress Bar',
    role: 'progress-bar',
    width: barWidth,
    height: 8,
    cornerRadius: 4,
    fill: [{ type: 'solid', color: trackColor }],
    layout: 'horizontal',
    alignItems: 'center',
    children,
  };
}
