import type { ElementTree } from './helpers.js';

export interface RangeSliderParams {
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
   * the same row as the label (or standalone if no label). Default
   * false.
   */
  show_value?: boolean;
  /** Optional suffix for the rendered value (e.g. "%", "px", "°"). */
  value_suffix?: string;
  /** Slider track width in px. Default 320. Min 160. */
  width?: number;
}

const TRACK_HEIGHT = 6;
const THUMB_SIZE = 20;

/**
 * Single-thumb range slider — the "Volume 60%" / "Filter from 0 to N"
 * horizontal input. Emits a visual static representation of the
 * control at the given value (no interaction wiring; this is a design
 * element).
 *
 * Structure:
 *   frame(width, fit_content, layout=vertical, gap=8, role='range-slider')
 *     ├ frame(horizontal, space-between, role='range-slider-header')   ← if label OR show_value
 *     │   ├ text(label, 13/500 slate-900, role='range-slider-label')    ← if label
 *     │   └ text(value, 13/500 slate-600, role='range-slider-value')    ← if show_value
 *     └ frame(fill_container, height=THUMB_SIZE, horizontal, align=center, role='range-slider-track')
 *         ├ rectangle(filled portion, accent #2563EB, h=6, cr=3, role='range-slider-fill')
 *         ├ frame(thumb 20×20, white, stroke=accent, cr=10, role='range-slider-thumb')
 *         └ rectangle(remaining portion, slate #E2E8F0, h=6, cr=3, role='range-slider-remaining')
 *
 * Pixel math: the track is `width` px total. Thumb is 20 px wide and
 * centered on the value point. Filled rect is `value% * (width-thumb)`
 * wide (left of thumb), remaining rect is the mirrored width. Both
 * filled/remaining collapse to 0 when value is at either extreme.
 */
export function buildRangeSlider(params: RangeSliderParams): ElementTree {
  const width = Math.max(160, Math.floor(params.width ?? 320));
  const min = params.min ?? 0;
  const max = params.max ?? 100;
  const span = Math.max(1, max - min); // guard against max<=min
  const raw = params.value ?? (min + max) / 2;
  const value = Math.max(min, Math.min(max, raw));
  const pct = (value - min) / span;

  const trackWidth = width;
  const leftWidth = Math.max(0, Math.round((trackWidth - THUMB_SIZE) * pct));
  const rightWidth = Math.max(0, trackWidth - THUMB_SIZE - leftWidth);

  const trackChildren: ElementTree[] = [];
  if (leftWidth > 0) {
    trackChildren.push({
      type: 'rectangle',
      name: 'Fill',
      role: 'range-slider-fill',
      width: leftWidth,
      height: TRACK_HEIGHT,
      cornerRadius: TRACK_HEIGHT / 2,
      fill: [{ type: 'solid', color: '#2563EB' }],
    });
  }
  trackChildren.push({
    type: 'frame',
    name: 'Thumb',
    role: 'range-slider-thumb',
    width: THUMB_SIZE,
    height: THUMB_SIZE,
    cornerRadius: THUMB_SIZE / 2,
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 2, fill: [{ type: 'solid', color: '#2563EB' }] },
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
      fill: [{ type: 'solid', color: '#E2E8F0' }],
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
        fill: [{ type: 'solid', color: '#0F172A' }],
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
        fill: [{ type: 'solid', color: '#64748B' }],
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
