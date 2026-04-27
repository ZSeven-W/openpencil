import type { ElementTree } from './helpers.js';

export interface DataTableRowColumn {
  content: string;
}

export interface DataTableRowParams {
  columns: DataTableRowColumn[];
  /** Render as the table header row (smaller, slate-500, slightly bolder). */
  header?: boolean;
  /** Mark as the currently selected/hover row (slate-50 fill). */
  selected?: boolean;
}

/**
 * Desktop / dashboard data-table row — N evenly-fill_container cells
 * laid out horizontally with a 16px gap (no vertical dividers, matches
 * modern Linear / Stripe styling). Distinct from `add_list_row_v0`,
 * which is the iOS / mobile leading-icon + title-stack + trailing-
 * chevron pattern.
 *
 * Header rows use 12/600 slate-500 typography and a 40px row height.
 * Body rows use 14/400 slate-900 typography and a 48px row height,
 * optionally tinted slate-50 when `selected` is true.
 *
 * For row separators stack `add_divider_v0` between successive rows;
 * the tool does not emit its own bottom border.
 *
 * Overflow contract: long cell content must not bleed into adjacent
 * columns. Each cell is `fill_container` width / height with
 * `clipContent: true`, and the text child rides `width=fill_container`
 * + `textGrowth='fixed-width'` so the layout engine wraps the string
 * inside the cell bounds and the cell visually clips anything past
 * the row's fixed height. The row itself also clips so a runaway
 * cell can't push siblings past the right edge.
 */
export function buildDataTableRow(params: DataTableRowParams): ElementTree {
  const isHeader = params.header === true;
  const isSelected = !isHeader && params.selected === true;
  const cellTextSize = isHeader ? 12 : 14;
  const cellTextWeight = isHeader ? 600 : 400;
  const cellTextColor = isHeader ? '#64748B' : '#0F172A';
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
      buildCell(col, i, isHeader, cellTextSize, cellTextWeight, cellTextColor),
    ),
  };

  if (isSelected) {
    node.fill = [{ type: 'solid', color: '#F8FAFC' }];
  }

  return node;
}

function buildCell(
  col: DataTableRowColumn,
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
