import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface FilterGroupV1Option {
  label: string;
  /** Optional count badge (e.g. "(42)"). */
  count?: number;
  /** Whether this facet is currently selected. Default false. */
  selected?: boolean;
}

export interface FilterGroupV1Params {
  /** Heading text above the option list (e.g. "Category", "Brand"). */
  title: string;
  /** Options. Each renders a checkbox-style row + optional count. */
  options: FilterGroupV1Option[];
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_filter_group_v0.
   * - `'dark'`: dark surface/text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Sidebar filter group / facet — theme-aware version of buildFilterGroup.
 * Light mode is byte-equal to add_filter_group_v0.
 *
 * Color mapping:
 *   title (#0F172A)              → textPrimary
 *   label (#334155)              → textBody
 *   count (#94A3B8)              → textSubtle
 *   unselected box bg (#FFFFFF)  → surface
 *   unselected box stroke (#CBD5E1) → border
 *   selected box bg (#2563EB)    → accent
 *   selected box stroke (#2563EB) → accent
 *   check icon (#FFFFFF)         → kept as-is (white on accent)
 */
export function buildFilterGroupV1(params: FilterGroupV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const titleColor = isLight ? '#0F172A' : t.colors.textPrimary;
  const labelColor = isLight ? '#334155' : t.colors.textBody;
  const countColor = isLight ? '#94A3B8' : t.colors.textSubtle;
  const boxBgUnselected = isLight ? '#FFFFFF' : t.colors.surface;
  const boxStrokeUnselected = isLight ? '#CBD5E1' : t.colors.border;
  const accentColor = isLight ? '#2563EB' : t.colors.accent;

  const optionRows = params.options.map((opt) =>
    buildOptionRowV1(
      opt,
      labelColor,
      countColor,
      boxBgUnselected,
      boxStrokeUnselected,
      accentColor,
    ),
  );

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
        fill: [{ type: 'solid', color: titleColor }],
      },
      ...optionRows,
    ],
  };
}

function buildOptionRowV1(
  opt: FilterGroupV1Option,
  labelColor: string,
  countColor: string,
  boxBgUnselected: string,
  boxStrokeUnselected: string,
  accentColor: string,
): ElementTree {
  const selected = opt.selected === true;
  const box: ElementTree = {
    type: 'frame',
    name: 'Box',
    role: 'filter-group-checkbox',
    width: 16,
    height: 16,
    cornerRadius: 4,
    fill: [{ type: 'solid', color: selected ? accentColor : boxBgUnselected }],
    stroke: {
      thickness: 1.5,
      fill: [{ type: 'solid', color: selected ? accentColor : boxStrokeUnselected }],
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
            fill: [{ type: 'solid', color: '#FFFFFF' }],
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
      fill: [{ type: 'solid', color: labelColor }],
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
      fill: [{ type: 'solid', color: countColor }],
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
