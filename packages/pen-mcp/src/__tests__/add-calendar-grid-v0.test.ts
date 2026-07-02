import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddCalendarGridV0 } from '../tools/add-calendar-grid-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-calendar-grid-v0');
const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY, 'utf-8');
  return fp;
}
async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}
function getRoot(doc: Record<string, unknown>): Record<string, unknown> {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const top = doc['children'] as Record<string, unknown>[] | undefined;
  const root = (top ?? pages?.[0]?.children)?.[0];
  if (!root) throw new Error('no root');
  return root;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['a.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_calendar_grid_v0', () => {
  it('registered + no required args (all optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_calendar_grid_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_calendar_grid_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('default 30-day month: 1 header row + 5 week rows (6 total)', async () => {
    const fp = await fresh('a.op');
    await handleAddCalendarGridV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('calendar-grid');
    const rows = root.children as Record<string, unknown>[];
    // 30 days starting at offset 0 → 30/7 rounded up = 5 week rows
    expect(rows.length).toBe(1 + 5);
    expect(rows[0].role).toBe('calendar-header-row');
    expect((rows[0].children as Record<string, unknown>[]).length).toBe(7);
  });

  it('start_day_offset=3: first 3 cells in week 1 are blank', async () => {
    const fp = await fresh('a.op');
    await handleAddCalendarGridV0({ filePath: fp, days_in_month: 30, start_day_offset: 3 });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const week1 = rows[1].children as Record<string, unknown>[];
    expect(week1[0].role).toBe('calendar-day-empty');
    expect(week1[1].role).toBe('calendar-day-empty');
    expect(week1[2].role).toBe('calendar-day-empty');
    expect(week1[3].role).toBe('calendar-day');
    const firstDayText = (week1[3].children as Record<string, unknown>[])[0];
    expect(firstDayText.content).toBe('1');
  });

  it('selected_day wins over today on overlap', async () => {
    const fp = await fresh('a.op');
    await handleAddCalendarGridV0({
      filePath: fp,
      days_in_month: 30,
      today: 15,
      selected_day: 15,
    });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    // day 15 is at row index 2 (week 3), column 0 (Sunday offset 0)
    // offset 0 + day counter 15 = idx 14; idx 14 / 7 = row 2, col 0
    // But first row is header, so actual rows index = 1 + 2 = 3
    const week = rows[3].children as Record<string, unknown>[];
    expect(week[0].role).toBe('calendar-day-selected');
    const fill = week[0].fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#2563EB');
  });

  it('today-only gets light tint', async () => {
    const fp = await fresh('a.op');
    await handleAddCalendarGridV0({ filePath: fp, days_in_month: 30, today: 1 });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const week1 = rows[1].children as Record<string, unknown>[];
    expect(week1[0].role).toBe('calendar-day-today');
    const fill = week1[0].fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#DBEAFE');
  });

  it('31-day month with offset 6 → 6 week rows', async () => {
    const fp = await fresh('a.op');
    await handleAddCalendarGridV0({ filePath: fp, days_in_month: 31, start_day_offset: 6 });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    // 6 + 31 = 37 cells → ceil(37/7) = 6 week rows + 1 header = 7 total
    expect(rows.length).toBe(1 + 6);
  });

  it('clamps days_in_month to [1,31]', async () => {
    const fp = await fresh('a.op');
    await handleAddCalendarGridV0({ filePath: fp, days_in_month: 100 });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    // 31 + 0 = 31 → ceil(31/7) = 5 week rows + 1 header = 6
    expect(rows.length).toBe(1 + 5);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddCalendarGridV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
