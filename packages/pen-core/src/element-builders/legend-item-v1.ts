import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface LegendItemV1Params {
  /** Label text (e.g. "Revenue", "Expenses"). */
  label: string;
  /** Marker fill hex (e.g. "#2563EB"). Passed through unchanged in all modes. */
  color: string;
  /** Optional value shown to the right of the label (e.g. "$12,480"). */
  value?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_legend_item_v0.
   * - `'dark'`: label → textBody, value → textPrimary.
   * - `'system'`: emits `$color-*` refs for label and value fills.
   *
   * Note: `color` (the marker fill) is caller-supplied and kept as-is
   * in all modes — it represents a chart data series color, not a
   * semantic surface color.
   */
  theme?: V1Theme;
}

/**
 * Chart legend entry — theme-aware variant of buildLegendItem.
 * Light mode is byte-equal to add_legend_item_v0.
 *
 * Color mapping:
 *   label (#475569 slate-600)  → textBody
 *   value (#0F172A slate-950)  → textPrimary
 *   marker (params.color)      → unchanged (caller-supplied chart series color)
 */
export function buildLegendItemV1(params: LegendItemV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const labelColor = isLight ? '#475569' : t.colors.textBody;
  const valueColor = isLight ? '#0F172A' : t.colors.textPrimary;

  const children: ElementTree[] = [
    {
      type: 'frame',
      name: 'Marker',
      role: 'legend-item-marker',
      width: 10,
      height: 10,
      cornerRadius: 2,
      fill: [{ type: 'solid', color: params.color }],
      children: [],
    },
    {
      type: 'text',
      name: 'Label',
      role: 'legend-item-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 400,
      fill: [{ type: 'solid', color: labelColor }],
    },
  ];
  if (params.value) {
    children.push({
      type: 'text',
      name: 'Value',
      role: 'legend-item-value',
      content: params.value,
      fontSize: 13,
      fontWeight: 600,
      fill: [{ type: 'solid', color: valueColor }],
    });
  }
  return {
    type: 'frame',
    name: 'Legend Item',
    role: 'legend-item',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children,
  };
}
