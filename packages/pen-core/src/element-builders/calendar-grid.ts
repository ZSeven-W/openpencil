import type { ElementTree } from './helpers.js';

export interface CalendarGridParams {
  days_in_month?: number;
  start_day_offset?: number;
  today?: number;
  selected_day?: number;
}

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const CELL_SIZE = 40;

/**
 * Month calendar grid. Weekday header (Sun..Sat) + up to 6 week
 * rows × 7 cells. `start_day_offset` blanks leading cells so day
 * 1 lands on the correct weekday (0=Sun..6=Sat). Trailing cells
 * blanked to round to multiple of 7.
 *
 * `today` gets light-blue tint; `selected_day` gets solid primary
 * fill (selected wins on overlap). Fixed 40px cells (emitted as
 * nested vertical-of-horizontal frames since pen-core lacks grid).
 */
export function buildCalendarGrid(params: CalendarGridParams): ElementTree {
  const daysInMonth = Math.max(1, Math.min(31, Math.floor(params.days_in_month ?? 30)));
  const startOffset = Math.max(0, Math.min(6, Math.floor(params.start_day_offset ?? 0)));
  const today = typeof params.today === 'number' ? Math.floor(params.today) : undefined;
  const selected =
    typeof params.selected_day === 'number' ? Math.floor(params.selected_day) : undefined;

  const rows: ElementTree[] = [];

  rows.push({
    type: 'frame',
    name: 'Weekday Header',
    role: 'calendar-header-row',
    width: 'fit_content',
    height: CELL_SIZE,
    layout: 'horizontal',
    gap: 0,
    children: WEEKDAYS.map((d) => ({
      type: 'frame',
      name: `Header ${d}`,
      role: 'calendar-header',
      width: CELL_SIZE,
      height: CELL_SIZE,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'Label',
          content: d,
          fontSize: 12,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#6B7280' }],
        },
      ],
    })),
  });

  const totalCells = Math.ceil((startOffset + daysInMonth) / 7) * 7;
  let dayCounter = 1;
  for (let r = 0; r < totalCells / 7; r += 1) {
    const cells: ElementTree[] = [];
    for (let c = 0; c < 7; c += 1) {
      const idx = r * 7 + c;
      if (idx < startOffset || dayCounter > daysInMonth) {
        cells.push({
          type: 'frame',
          name: 'Empty',
          role: 'calendar-day-empty',
          width: CELL_SIZE,
          height: CELL_SIZE,
        });
        continue;
      }
      const d = dayCounter;
      const isSelected = selected === d;
      const isToday = !isSelected && today === d;
      const role = isSelected
        ? 'calendar-day-selected'
        : isToday
          ? 'calendar-day-today'
          : 'calendar-day';
      const cell: ElementTree = {
        type: 'frame',
        name: `Day ${d}`,
        role,
        width: CELL_SIZE,
        height: CELL_SIZE,
        cornerRadius: CELL_SIZE / 2,
        layout: 'horizontal',
        alignItems: 'center',
        justifyContent: 'center',
        children: [
          {
            type: 'text',
            name: 'Number',
            content: String(d),
            fontSize: 14,
            fontWeight: isSelected || isToday ? 600 : 400,
            fill: [{ type: 'solid', color: isSelected ? '#FFFFFF' : '#111827' }],
          },
        ],
      };
      if (isSelected) {
        cell.fill = [{ type: 'solid', color: '#2563EB' }];
      } else if (isToday) {
        cell.fill = [{ type: 'solid', color: '#DBEAFE' }];
      }
      cells.push(cell);
      dayCounter += 1;
    }
    rows.push({
      type: 'frame',
      name: `Week ${r + 1}`,
      role: 'calendar-week',
      width: 'fit_content',
      height: CELL_SIZE,
      layout: 'horizontal',
      gap: 0,
      children: cells,
    });
  }

  return {
    type: 'frame',
    name: 'Calendar',
    role: 'calendar-grid',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'vertical',
    gap: 0,
    children: rows,
  };
}
