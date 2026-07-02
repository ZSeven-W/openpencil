import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface RangeSliderV1Params {
  /** Current value. Clamped to [min, max]. Default 50. */
  value?: number;
  /** Min value. Default 0. */
  min?: number;
  /** Max value. Default 100. */
  max?: number;
  /** Optional label shown above the track (left-aligned). */
  label?: string;
  /**
   * When true, show the current value as a right-aligned text on
   * the same row as the label (or standalone if no label). Default false.
   */
  show_value?: boolean;
  /** Optional suffix for the rendered value (e.g. "%", "px", "°"). */
  value_suffix?: string;
  /** Slider track width in px. Default 320. Min 160. */
  width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_range_slider_v0.
   * - `'dark'`: accent stays brand-invariant; thumb bg → surface; remaining track → border;
   *   label → textPrimary; value text → textMuted; shadow stays (theme-agnostic).
   * - `'system'`: $color-* refs for non-accent fields.
   */
  theme?: V1Theme;
}

const TRACK_HEIGHT = 6;
const THUMB_SIZE = 20;

/**
 * Single-thumb range slider (v1) — theme-aware variant of buildRangeSlider.
 * Light mode is byte-equal to add_range_slider_v0.
 *
 * Color mapping:
 *   fill bar  (#2563EB accent) — brand-invariant, kept across themes
 *   thumb bg  (#FFFFFF white) → surface token
 *   thumb stroke (#2563EB accent) — brand-invariant
 *   remaining (#E2E8F0 border) → border token
 *   label text (#0F172A slate-950) → textPrimary
 *   value text (#64748B slate-500) → textMuted
 *   shadow    (#0F172A1F) — theme-agnostic (kept as-is)
 */
export function buildRangeSliderV1(params: RangeSliderV1Params): ElementTree {
  const width = Math.max(160, Math.floor(params.width ?? 320));
  const min = params.min ?? 0;
  const max = params.max ?? 100;
  const span = Math.max(1, max - min);
  const raw = params.value ?? (min + max) / 2;
  const value = Math.max(min, Math.min(max, raw));
  const pct = (value - min) / span;

  const trackWidth = width;
  const leftWidth = Math.max(0, Math.round((trackWidth - THUMB_SIZE) * pct));
  const rightWidth = Math.max(0, trackWidth - THUMB_SIZE - leftWidth);

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Accent is brand-invariant
  const accentColor = '#2563EB';
  // Thumb bg: light → white (#FFFFFF), dark/system → surface
  const thumbBg = isLight ? '#FFFFFF' : t.colors.surface;
  // Remaining track: light → border-ish (#E2E8F0), dark/system → border
  const remainingColor = isLight ? '#E2E8F0' : t.colors.border;
  // Label text: light → slate-950 (#0F172A), dark/system → textPrimary
  const labelColor = isLight ? '#0F172A' : t.colors.textPrimary;
  // Value text: light → slate-500 (#64748B), dark/system → textMuted
  const valueColor = isLight ? '#64748B' : t.colors.textMuted;

  const trackChildren: ElementTree[] = [];
  if (leftWidth > 0) {
    trackChildren.push({
      type: 'rectangle',
      name: 'Fill',
      role: 'range-slider-fill',
      width: leftWidth,
      height: TRACK_HEIGHT,
      cornerRadius: TRACK_HEIGHT / 2,
      fill: [{ type: 'solid', color: accentColor }],
    });
  }
  trackChildren.push({
    type: 'frame',
    name: 'Thumb',
    role: 'range-slider-thumb',
    width: THUMB_SIZE,
    height: THUMB_SIZE,
    cornerRadius: THUMB_SIZE / 2,
    fill: [{ type: 'solid', color: thumbBg }],
    stroke: { thickness: 2, fill: [{ type: 'solid', color: accentColor }] },
    effects: [
      {
        type: 'shadow',
        offsetX: 0,
        offsetY: 2,
        blur: 4,
        spread: 0,
        color: '#0F172A1F',
      },
    ],
  });
  if (rightWidth > 0) {
    trackChildren.push({
      type: 'rectangle',
      name: 'Remaining',
      role: 'range-slider-remaining',
      width: rightWidth,
      height: TRACK_HEIGHT,
      cornerRadius: TRACK_HEIGHT / 2,
      fill: [{ type: 'solid', color: remainingColor }],
    });
  }

  const children: ElementTree[] = [];

  if (params.label || params.show_value) {
    const headerChildren: ElementTree[] = [];
    if (params.label) {
      headerChildren.push({
        type: 'text',
        name: 'Label',
        role: 'range-slider-label',
        content: params.label,
        fontSize: 13,
        fontWeight: 500,
        fill: [{ type: 'solid', color: labelColor }],
      });
    }
    if (params.show_value) {
      const suffix = params.value_suffix ?? '';
      const rendered = `${Math.round(value * 100) / 100}${suffix}`;
      headerChildren.push({
        type: 'text',
        name: 'Value',
        role: 'range-slider-value',
        content: rendered,
        fontSize: 13,
        fontWeight: 500,
        fill: [{ type: 'solid', color: valueColor }],
      });
    }
    children.push({
      type: 'frame',
      name: 'Header',
      role: 'range-slider-header',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: params.label && params.show_value ? 'space-between' : 'flex-start',
      children: headerChildren,
    });
  }

  children.push({
    type: 'frame',
    name: 'Track',
    role: 'range-slider-track',
    width: trackWidth,
    height: THUMB_SIZE,
    layout: 'horizontal',
    alignItems: 'center',
    children: trackChildren,
  });

  return {
    type: 'frame',
    name: 'Range Slider',
    role: 'range-slider',
    width,
    height: 'fit_content',
    layout: 'vertical',
    gap: 8,
    children,
  };
}
