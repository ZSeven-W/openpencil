import { buildScrollWrapper, type ElementTree } from './helpers.js';

export interface NavChipRowItem {
  label: string;
  icon?: string;
  active?: boolean;
}

export interface NavChipRowParams {
  items: NavChipRowItem[];
  chip_width?: number;
  gap?: number;
}

/**
 * Horizontal scroll row of NAV CHIPS (icon + small label, with
 * optional active state). 72×fit_content per chip, cornerRadius=12,
 * padding=[8,12], vertical layout, alignItems=center. Active
 * chips get role='nav-chip-active' + bolder label.
 */
export function buildNavChipRow(params: NavChipRowParams): ElementTree {
  const chipWidth = params.chip_width ?? 72;
  const gap = params.gap ?? 12;
  const chips = params.items.map((item) => buildChip(item, chipWidth));
  return buildScrollWrapper({ rowName: 'Nav Chip Row', innerChildren: chips, gap });
}

function buildChip(item: NavChipRowItem, chipWidth: number): ElementTree {
  const children: ElementTree[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
    });
  }
  children.push({
    type: 'text',
    name: 'Label',
    role: 'label',
    content: item.label,
    fontSize: 11,
    fontWeight: item.active ? 600 : 500,
  });
  return {
    type: 'frame',
    name: `Chip (${item.label})`,
    role: item.active ? 'nav-chip-active' : 'nav-chip',
    width: chipWidth,
    height: 'fit_content',
    cornerRadius: 12,
    padding: [8, 12],
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
