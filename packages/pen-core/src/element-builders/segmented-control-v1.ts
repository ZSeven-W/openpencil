import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface SegmentedControlV1Item {
  label: string;
  active?: boolean;
}

export interface SegmentedControlV1Params {
  items: SegmentedControlV1Item[];
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_segmented_control_v0.
   * - `'dark'`: track bg → surface2; active seg bg → surface; active label → textPrimary;
   *   inactive label → textMuted.
   * - `'system'`: $color-* refs for track, active seg bg, and label colors.
   */
  theme?: V1Theme;
}

/**
 * Segmented control (v1) — theme-aware variant of buildSegmentedControl.
 * Light mode is byte-equal to add_segmented_control_v0.
 *
 * Color mapping:
 *   track bg         (#F3F4F6 gray-100)  → surface2 token (secondary surface)
 *   active seg bg    (#FFFFFF white)     → surface token (primary surface)
 *   active label     (#111827 gray-900)  → textPrimary token
 *   inactive label   (#4B5563 gray-600)  → textMuted token
 */
export function buildSegmentedControlV1(params: SegmentedControlV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Track bg: light → gray-100 (#F3F4F6), dark/system → surface2
  const trackBg = isLight ? '#F3F4F6' : t.colors.surface2;
  // Active segment bg: light → white (#FFFFFF), dark/system → surface
  const activeSegBg = isLight ? '#FFFFFF' : t.colors.surface;
  // Active label: light → gray-900 (#111827), dark/system → textPrimary
  const activeLabelColor = isLight ? '#111827' : t.colors.textPrimary;
  // Inactive label: light → gray-600 (#4B5563), dark/system → textMuted
  const inactiveLabelColor = isLight ? '#4B5563' : t.colors.textMuted;

  const segments: ElementTree[] = params.items.map((item) => {
    const seg: ElementTree = {
      type: 'frame',
      name: `Segment (${item.label})`,
      role: item.active ? 'segment-active' : 'segment',
      width: 'fill_container',
      height: 'fill_container',
      cornerRadius: 6,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'Label',
          role: 'label',
          content: item.label,
          fontSize: 13,
          fontWeight: item.active ? 600 : 500,
          fill: [{ type: 'solid', color: item.active ? activeLabelColor : inactiveLabelColor }],
        },
      ],
    };
    seg.fill = item.active ? [{ type: 'solid', color: activeSegBg }] : [];
    return seg;
  });
  return {
    type: 'frame',
    name: 'Segmented Control',
    role: 'segmented-control',
    width: 'fill_container',
    height: 32,
    cornerRadius: 8,
    fill: [{ type: 'solid', color: trackBg }],
    layout: 'horizontal',
    alignItems: 'stretch',
    gap: 4,
    padding: [4],
    children: segments,
  };
}
