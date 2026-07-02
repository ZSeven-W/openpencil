import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddStatGridV0 } from '../tools/add-stat-grid-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-stat-grid-v0');
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

describe('add_stat_grid_v0', () => {
  it('registered + required items', () => {
    expect(DESIGN_TOOL_NAMES.has('add_stat_grid_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_stat_grid_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
  });

  it('EACH CELL uses width=fill_container (critical for no-overflow invariant)', async () => {
    const fp = await fresh('a.op');
    await handleAddStatGridV0({
      filePath: fp,
      items: [
        { value: '8,432', label: 'Steps', icon: 'activity' },
        { value: '512', label: 'Kcal' },
        { value: '7h', label: 'Sleep' },
      ],
    });
    const grid = getRoot(await readDoc(fp));
    expect(grid.role).toBe('stat-grid');
    expect(grid.width).toBe('fill_container');
    expect(grid.layout).toBe('horizontal');
    expect(grid.justifyContent).toBe('space_between');
    const cells = grid.children as Record<string, unknown>[];
    expect(cells.length).toBe(3);
    for (const cell of cells) {
      expect(cell.role).toBe('stat-cell');
      // THIS is the core invariant: fill_container on every cell = no overflow
      expect(cell.width).toBe('fill_container');
      expect(cell.layout).toBe('vertical');
      expect(cell.alignItems).toBe('center');
    }
  });

  it('emits value + label (heading + body), optional icon', async () => {
    const fp = await fresh('a.op');
    await handleAddStatGridV0({
      filePath: fp,
      items: [
        { value: '75%', label: 'Goal', icon: 'target' },
        { value: '42', label: 'Workouts' },
      ],
    });
    const cells = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    // with icon: icon + value + label = 3 kids
    const k0 = cells[0].children as Record<string, unknown>[];
    expect(k0.length).toBe(3);
    expect(k0[0].type).toBe('icon_font');
    expect(k0[0].iconFontName).toBe('target');
    expect(k0[1].content).toBe('75%');
    expect(k0[1].role).toBe('heading');
    expect(k0[2].content).toBe('Goal');
    expect(k0[2].role).toBe('body');
    // without icon: value + label = 2 kids
    const k1 = cells[1].children as Record<string, unknown>[];
    expect(k1.length).toBe(2);
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddStatGridV0({
      filePath: fp,
      items: [
        { value: '1', label: 'A' },
        { value: '2', label: 'B' },
      ],
    });
    const ids: string[] = [];
    function walk(n: Record<string, unknown>): void {
      if (typeof n.id === 'string') ids.push(n.id);
      if (Array.isArray(n.children))
        (n.children as Record<string, unknown>[]).forEach(
          (c) => c && typeof c === 'object' && walk(c),
        );
    }
    walk(getRoot(await readDoc(fp)));
    // grid + 2 cells + 4 children (2 each: value + label, no icon) = 7
    expect(ids.length).toBe(7);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddStatGridV0({
        filePath: fp,
        items: [{ value: '1', label: 'A' }],
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
