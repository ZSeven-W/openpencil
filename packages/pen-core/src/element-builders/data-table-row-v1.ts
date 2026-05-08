import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface DataTableRowV1Column {
  content: string;
}

export interface DataTableRowV1Params {
  columns: DataTableRowV1Column[];
  /** Render as the table header row (smaller, slate-500, slightly bolder). */
  header?: boolean;
  /** Mark as the currently selected/hover row (slate-50 fill). */
  selected?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_data_table_row_v0.
   * - `'dark'`: dark surface/text/border for all fill fields.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * Desktop data-table row — theme-aware version of buildDataTableRow.
 * Light mode is byte-equal to add_data_table_row_v0.
 *
 * Color mapping:
 *   header cell text (#64748B)   → textMuted
 *   body cell text (#0F172A)     → textPrimary
 *   selected row bg (#F8FAFC)    → bgDeep
 */
export function buildDataTableRowV1(params: DataTableRowV1Params): ElementTree {
  const isHeader = params.header === true;
  const isSelected = !isHeader && params.selected === true;
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const cellTextSize = isHeader ? 12 : 14;
  const cellTextWeight = isHeader ? 600 : 400;
  const cellTextColor = isHeader
    ? isLight
      ? '#64748B'
      : t.colors.textMuted
    : isLight
      ? '#0F172A'
      : t.colors.textPrimary;
  const rowHeight = isHeader ? 40 : 48;

  const node: ElementTree = {
    type: 'frame',
    name: isHeader ? 'Data Table Header Row' : 'Data Table Row',
    role: isHeader ? 'data-table-header-row' : 'data-table-row',
    width: 'fill_container',
    height: rowHeight,
    layout: 'horizontal',
    padding: [0, 16],
    gap: 16,
    alignItems: 'center',
    clipContent: true,
    children: params.columns.map((col, i) =>
      buildCellV1(col, i, isHeader, cellTextSize, cellTextWeight, cellTextColor),
    ),
  };

  if (isSelected) {
    node.fill = [{ type: 'solid', color: isLight ? '#F8FAFC' : t.colors.bgDeep }];
  }

  return node;
}

function buildCellV1(
  col: DataTableRowV1Column,
  index: number,
  isHeader: boolean,
  size: number,
  weight: number,
  color: string,
): ElementTree {
  return {
    type: 'frame',
    name: `Cell ${index + 1}`,
    role: isHeader ? 'data-table-header-cell' : 'data-table-cell',
    width: 'fill_container',
    height: 'fill_container',
    layout: 'horizontal',
    alignItems: 'center',
    clipContent: true,
    children: [
      {
        type: 'text',
        name: 'Cell Text',
        role: isHeader ? 'data-table-header-text' : 'data-table-cell-text',
        content: col.content,
        fontSize: size,
        fontWeight: weight,
        width: 'fill_container',
        textGrowth: 'fixed-width',
        fill: [{ type: 'solid', color }],
      },
    ],
  };
}
