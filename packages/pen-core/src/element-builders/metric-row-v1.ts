import { buildScrollWrapper, type ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';
import type { MetricRowItem } from './metric-row.js';

export interface MetricRowV1Params {
  items: MetricRowItem[];
  tile_width?: number;
  gap?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_metric_row_v0.
   * - `'dark'`: identical (v0 emits no hardcoded colors — tile has no
   *   fill; text inherits canvas default color).
   * - `'system'`: identical.
   * Accepts theme param for API consistency across all v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Horizontal scroll row of METRIC TILES (v1) — theme-aware variant of buildMetricRow.
 * No hardcoded colors in v0; light/dark/system modes are byte-identical.
 * Accepts theme param for API consistency.
 */
export function buildMetricRowV1(params: MetricRowV1Params): ElementTree {
  const tileWidth = params.tile_width ?? 120;
  const gap = params.gap ?? 12;
  const tiles = params.items.map((item) => buildTile(item, tileWidth));
  return buildScrollWrapper({ rowName: 'Metric Row', innerChildren: tiles, gap });
}

function buildTile(item: MetricRowItem, tileWidth: number): ElementTree {
  const children: ElementTree[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  children.push({
    type: 'text',
    name: 'Label',
    role: 'body',
    content: item.label,
    fontSize: 12,
    fontWeight: 500,
    width: 'fill_container',
  });
  children.push({
    type: 'text',
    name: 'Value',
    role: 'heading',
    content: item.value,
    fontSize: 28,
    fontWeight: 700,
    width: 'fill_container',
  });
  return {
    type: 'frame',
    name: 'Metric Tile',
    role: 'metric-tile',
    width: tileWidth,
    height: 100,
    cornerRadius: 16,
    padding: 16,
    layout: 'vertical',
    gap: 4,
    children,
  };
}
