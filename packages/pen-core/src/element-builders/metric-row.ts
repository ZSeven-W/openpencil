import { buildScrollWrapper, type ElementTree } from './helpers.js';

export interface MetricRowItem {
  label: string;
  value: string;
  icon?: string;
}

export interface MetricRowParams {
  items: MetricRowItem[];
  tile_width?: number;
  gap?: number;
}

/**
 * Horizontal scroll row of METRIC TILES (small label + big value +
 * optional icon). Each tile: `tile_width`×100 frame, cornerRadius=16,
 * padding=16, vertical layout, gap=4.
 *
 * label = 12/500 (body role), value = 28/700 (heading role).
 */
export function buildMetricRow(params: MetricRowParams): ElementTree {
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
