import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddPaginationV0 } from '../tools/add-pagination-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-pagination-v0');
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

describe('add_pagination_v0', () => {
  it('registered; required=[total]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_pagination_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_pagination_v0');
    expect(def?.inputSchema.required).toEqual(['total']);
  });

  it('small range (total=3) → all pages shown, no ellipsis, arrows present', async () => {
    const fp = await fresh('a.op');
    await handleAddPaginationV0({ filePath: fp, total: 3, current: 2 });
    const p = getRoot(await readDoc(fp));
    expect(p.role).toBe('pagination');
    const kids = p.children as Record<string, unknown>[];
    // prev + 3 pages + next = 5
    expect(kids.length).toBe(5);
    expect(kids[0].role).toBe('pagination-prev');
    expect(kids[4].role).toBe('pagination-next');
    // middle pill is active
    const active = kids[2];
    expect(active.role).toBe('pagination-page-active');
    const activeFill = active.fill as Array<{ color: string }>;
    expect(activeFill[0].color).toBe('#0F172A');
  });

  it('large range with ellipsis: total=10, current=5 → 1 … 4 [5] 6 … 10', async () => {
    const fp = await fresh('a.op');
    await handleAddPaginationV0({ filePath: fp, total: 10, current: 5, show_arrows: false });
    const p = getRoot(await readDoc(fp));
    const kids = p.children as Record<string, unknown>[];
    // no arrows → 1, ellipsis, 4, 5, 6, ellipsis, 10 = 7
    expect(kids.length).toBe(7);
    const ellipsisCount = kids.filter((k) => k.role === 'pagination-ellipsis').length;
    expect(ellipsisCount).toBe(2);
    const activeCount = kids.filter((k) => k.role === 'pagination-page-active').length;
    expect(activeCount).toBe(1);
  });

  it('current at start: total=10, current=1 → no left ellipsis', async () => {
    const fp = await fresh('a.op');
    await handleAddPaginationV0({ filePath: fp, total: 10, current: 1, show_arrows: false });
    const p = getRoot(await readDoc(fp));
    const kids = p.children as Record<string, unknown>[];
    // 1, 2, ellipsis, 10 = 4
    expect(kids.length).toBe(4);
    expect(kids[0].role).toBe('pagination-page-active');
    expect(kids[2].role).toBe('pagination-ellipsis');
  });

  it('accent_color overrides active fill', async () => {
    const fp = await fresh('a.op');
    await handleAddPaginationV0({ filePath: fp, total: 3, current: 1, accent_color: '#4F46E5' });
    const p = getRoot(await readDoc(fp));
    const kids = p.children as Record<string, unknown>[];
    const active = kids.find((k) => k.role === 'pagination-page-active')!;
    const fill = active.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#4F46E5');
  });

  it('show_arrows=false drops prev/next', async () => {
    const fp = await fresh('a.op');
    await handleAddPaginationV0({ filePath: fp, total: 3, current: 1, show_arrows: false });
    const p = getRoot(await readDoc(fp));
    const kids = p.children as Record<string, unknown>[];
    expect(kids.find((k) => k.role === 'pagination-prev')).toBeUndefined();
    expect(kids.find((k) => k.role === 'pagination-next')).toBeUndefined();
  });

  it('total=1 → single active pill (+ arrows if default)', async () => {
    const fp = await fresh('a.op');
    await handleAddPaginationV0({ filePath: fp, total: 1 });
    const p = getRoot(await readDoc(fp));
    const kids = p.children as Record<string, unknown>[];
    // prev + 1 active + next
    expect(kids.length).toBe(3);
    expect(kids[1].role).toBe('pagination-page-active');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddPaginationV0({ filePath: fp, total: 5, parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
