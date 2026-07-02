/**
 * Unit tests for add_data_table_row_v0 — desktop tabular row.
 * Distinct surface from `add_list_row_v0` (iOS / mobile list cell).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddDataTableRowV0 } from '../tools/add-data-table-row-v0';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-add-data-table-row-v0-tests');
const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP_DIR, name);
  await writeFile(fp, EMPTY_DOC, 'utf-8');
  return fp;
}

async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}

function getRoot(doc: Record<string, unknown>): Record<string, unknown> {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const pageChildren = pages?.[0]?.children;
  const topChildren = doc['children'] as Record<string, unknown>[] | undefined;
  const root = pageChildren?.[0] ?? topChildren?.[0];
  if (!root) throw new Error('expected root');
  return root;
}

beforeEach(async () => {
  await mkdir(TMP_DIR, { recursive: true });
});

afterEach(async () => {
  for (const f of ['body.op', 'header.op', 'selected.op', 'parent.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_data_table_row_v0 — registration', () => {
  it('is registered in DESIGN_TOOL_DEFINITIONS + NAMES', () => {
    expect(DESIGN_TOOL_DEFINITIONS.map((t) => t.name)).toContain('add_data_table_row_v0');
    expect(DESIGN_TOOL_NAMES.has('add_data_table_row_v0')).toBe(true);
  });
});

describe('add_data_table_row_v0 — structure', () => {
  it('emits body row: data-table-row, height=48, padding=[0,16], gap=16, no fill, slate-900 14/400 cells', async () => {
    const fp = await fresh('body.op');
    await handleAddDataTableRowV0({
      filePath: fp,
      columns: [{ content: 'Sarah' }, { content: 'sarah@acme.com' }, { content: 'Active' }],
    });
    const row = getRoot(await readDoc(fp));
    expect(row.role).toBe('data-table-row');
    expect(row.height).toBe(48);
    expect(row.padding).toEqual([0, 16]);
    expect(row.gap).toBe(16);
    expect(row.layout).toBe('horizontal');
    expect(row.fill).toBeUndefined();
    // Overflow contract: row + each cell must clip + text must ride
    // fill_container with fixed-width growth so long content can't
    // bleed into the next column. Locking these in the assertion
    // catches the trap that Codex caught on the first ship.
    expect(row.clipContent).toBe(true);
    const cells = row.children as Record<string, unknown>[];
    expect(cells.length).toBe(3);
    for (const cell of cells) {
      expect(cell.role).toBe('data-table-cell');
      expect(cell.width).toBe('fill_container');
      expect(cell.height).toBe('fill_container');
      expect(cell.clipContent).toBe(true);
      const text = (cell.children as Record<string, unknown>[])[0];
      expect(text.role).toBe('data-table-cell-text');
      expect(text.fontSize).toBe(14);
      expect(text.fontWeight).toBe(400);
      expect(text.width).toBe('fill_container');
      expect(text.textGrowth).toBe('fixed-width');
      expect((text.fill as Array<{ color: string }>)[0].color).toBe('#0F172A');
    }
  });

  it('emits header row: data-table-header-row, height=40, slate-500 12/600 cells', async () => {
    const fp = await fresh('header.op');
    await handleAddDataTableRowV0({
      filePath: fp,
      header: true,
      columns: [{ content: 'Customer' }, { content: 'Status' }],
    });
    const row = getRoot(await readDoc(fp));
    expect(row.role).toBe('data-table-header-row');
    expect(row.height).toBe(40);
    expect(row.fill).toBeUndefined();
    const cells = row.children as Record<string, unknown>[];
    for (const cell of cells) {
      expect(cell.role).toBe('data-table-header-cell');
      const text = (cell.children as Record<string, unknown>[])[0];
      expect(text.role).toBe('data-table-header-text');
      expect(text.fontSize).toBe(12);
      expect(text.fontWeight).toBe(600);
      expect((text.fill as Array<{ color: string }>)[0].color).toBe('#64748B');
    }
  });

  it('selected body row is tinted slate-50; selected on header is ignored', async () => {
    const fp = await fresh('selected.op');
    await handleAddDataTableRowV0({
      filePath: fp,
      selected: true,
      columns: [{ content: 'Alex' }, { content: 'Pending' }],
    });
    const tinted = getRoot(await readDoc(fp));
    expect((tinted.fill as Array<{ color: string }>)[0].color).toBe('#F8FAFC');

    invalidateCache(fp);
    await writeFile(fp, EMPTY_DOC, 'utf-8');
    await handleAddDataTableRowV0({
      filePath: fp,
      header: true,
      selected: true,
      columns: [{ content: 'Customer' }],
    });
    const headerRow = getRoot(await readDoc(fp));
    expect(headerRow.fill).toBeUndefined();
  });

  it('throws on bogus parent_id AND leaves file untouched (side-effect invariant)', async () => {
    const fp = await fresh('parent.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddDataTableRowV0({
        filePath: fp,
        columns: [{ content: 'X' }],
        parent_id: 'bogus-parent',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
