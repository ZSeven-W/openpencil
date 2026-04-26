import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSkeletonV0 } from '../tools/add-skeleton-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-skeleton-v0');
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

describe('add_skeleton_v0', () => {
  it('registered; no required field (all optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_skeleton_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_skeleton_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('default: 3 rows, first two fill_container, last ~60% (220px)', async () => {
    const fp = await fresh('a.op');
    await handleAddSkeletonV0({ filePath: fp });
    const sk = getRoot(await readDoc(fp));
    expect(sk.role).toBe('skeleton');
    expect(sk.layout).toBe('vertical');
    expect(sk.gap).toBe(12);
    const rows = sk.children as Record<string, unknown>[];
    expect(rows.length).toBe(3);
    expect(rows[0].type).toBe('rectangle');
    expect(rows[0].width).toBe('fill_container');
    expect(rows[0].height).toBe(16);
    expect(rows[1].width).toBe('fill_container');
    // Last row short
    expect(rows[2].width).toBe(220);
    // Every row has the same gray fill and cornerRadius
    for (const r of rows) {
      expect(r.cornerRadius).toBe(4);
      const fill = r.fill as Array<{ color: string }>;
      expect(fill[0].color).toBe('#E2E8F0');
    }
  });

  it('rows / row_height / row_gap respected', async () => {
    const fp = await fresh('a.op');
    await handleAddSkeletonV0({ filePath: fp, rows: 5, row_height: 20, row_gap: 8 });
    const sk = getRoot(await readDoc(fp));
    expect(sk.gap).toBe(8);
    const rows = sk.children as Record<string, unknown>[];
    expect(rows.length).toBe(5);
    for (const r of rows) expect(r.height).toBe(20);
  });

  it('clamping: rows=0 → 1, rows=99 → 20; row_height=2 → 4, row_gap=99 → 32', async () => {
    const fp = await fresh('a.op');

    await handleAddSkeletonV0({ filePath: fp, rows: 0 });
    let sk = getRoot(await readDoc(fp));
    expect((sk.children as unknown[]).length).toBe(1);

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddSkeletonV0({ filePath: fp, rows: 99 });
    sk = getRoot(await readDoc(fp));
    expect((sk.children as unknown[]).length).toBe(20);

    await writeFile(fp, EMPTY, 'utf-8');
    invalidateCache(fp);
    await handleAddSkeletonV0({ filePath: fp, row_height: 2, row_gap: 99 });
    sk = getRoot(await readDoc(fp));
    expect(sk.gap).toBe(32);
    const firstRow = (sk.children as Record<string, unknown>[])[0];
    expect(firstRow.height).toBe(4);
  });

  it('last_row_short=false → last row is fill_container too', async () => {
    const fp = await fresh('a.op');
    await handleAddSkeletonV0({ filePath: fp, rows: 3, last_row_short: false });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(rows[2].width).toBe('fill_container');
  });

  it('single-row mode skips the "last short" behavior (looks wrong otherwise)', async () => {
    const fp = await fresh('a.op');
    await handleAddSkeletonV0({ filePath: fp, rows: 1 });
    const rows = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(rows.length).toBe(1);
    expect(rows[0].width).toBe('fill_container');
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddSkeletonV0({ filePath: fp, rows: 4 });
    const ids: string[] = [];
    function walk(n: Record<string, unknown>): void {
      if (typeof n.id === 'string') ids.push(n.id);
      if (Array.isArray(n.children))
        (n.children as Record<string, unknown>[]).forEach(
          (c) => c && typeof c === 'object' && walk(c),
        );
    }
    walk(getRoot(await readDoc(fp)));
    // 1 wrapper + 4 rows = 5
    expect(ids.length).toBe(5);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddSkeletonV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
