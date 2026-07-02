import type { ElementTree } from './helpers.js';

export interface FilterGroupOption {
  label: string;
  /** Optional count badge (e.g. "(42)"). */
  count?: number;
  /** Whether this facet is currently selected. Default false. */
  selected?: boolean;
}

export interface FilterGroupParams {
  /** Heading text above the option list (e.g. "Category", "Brand"). */
  title: string;
  /** Options. Each renders a checkbox-style row + optional count. */
  options: FilterGroupOption[];
}

const TITLE_FG = '#0F172A';
const LABEL_FG = '#334155';
const COUNT_FG = '#94A3B8';
const BOX_BORDER = '#CBD5E1';
const BOX_BG_SELECTED = '#2563EB';
const BOX_FG_SELECTED = '#FFFFFF';

function buildOptionRow(opt: FilterGroupOption): ElementTree {
  const selected = opt.selected === true;
  const box: ElementTree = {
    type: 'frame',
    name: 'Box',
    role: 'filter-group-checkbox',
    width: 16,
    height: 16,
    cornerRadius: 4,
    fill: [{ type: 'solid', color: selected ? BOX_BG_SELECTED : '#FFFFFF' }],
    stroke: {
      thickness: 1.5,
      fill: [{ type: 'solid', color: selected ? BOX_BG_SELECTED : BOX_BORDER }],
    },
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: selected
      ? [
          {
            type: 'icon_font',
            name: 'Check',
            role: 'filter-group-check',
            iconFontName: 'check',
            iconFontFamily: 'lucide',
            width: 12,
            height: 12,
            fill: [{ type: 'solid', color: BOX_FG_SELECTED }],
          },
        ]
      : [],
  };

  const rowChildren: ElementTree[] = [
    box,
    {
      type: 'text',
      name: 'Label',
      role: 'filter-group-label',
      content: opt.label,
      fontSize: 14,
      fontWeight: 400,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: LABEL_FG }],
    },
  ];
  if (opt.count !== undefined) {
    rowChildren.push({
      type: 'text',
      name: 'Count',
      role: 'filter-group-count',
      content: String(opt.count),
      fontSize: 13,
      fontWeight: 400,
      fill: [{ type: 'solid', color: COUNT_FG }],
    });
  }

  return {
    type: 'frame',
    name: `Option: ${opt.label}`,
    role: 'filter-group-option',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 10,
    padding: [6, 0],
    children: rowChildren,
  };
}

/**
 * Sidebar filter group / facet — heading text over a vertical list of
 * checkbox-style option rows (label + optional count). Distinct from
 * `add_nav_chip_row_v0` (horizontal scrolling chips, single-select),
 * `add_tag_v0` (single applied chip with × close), and
 * `add_segmented_control_v0` (mutex pill tabs).
 *
 * Use for "facet panel", "filter sidebar", "category checklist",
 * "brand filter", "applied filters group", "搜索筛选侧栏".
 */
export function buildFilterGroup(params: FilterGroupParams): ElementTree {
  const optionRows = params.options.map(buildOptionRow);
  return {
    type: 'frame',
    name: 'Filter Group',
    role: 'filter-group',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 8,
    children: [
      {
        type: 'text',
        name: 'Title',
        role: 'filter-group-title',
        content: params.title,
        fontSize: 13,
        fontWeight: 600,
        letterSpacing: 1,
        fill: [{ type: 'solid', color: TITLE_FG }],
      },
      ...optionRows,
    ],
  };
}
