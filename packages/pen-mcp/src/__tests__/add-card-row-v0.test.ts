import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddCardRowV0 } from '../tools/add-card-row-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-card-row-v0');
const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY_DOC, 'utf-8');
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
function collectIds(
  n: Record<string, unknown>,
  ids: string[],
  missing: string[],
  path = 'root',
): void {
  if (typeof n.id !== 'string' || n.id.length === 0) missing.push(path);
  else ids.push(n.id);
  if (Array.isArray(n.children)) {
    (n.children as Record<string, unknown>[]).forEach((c, i) => {
      if (c && typeof c === 'object') collectIds(c, ids, missing, `${path}/${i}`);
    });
  }
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['a.op', 'b.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_card_row_v0', () => {
  it('registered + required items (no children_type)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_card_row_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_card_row_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
    expect(JSON.stringify(def?.inputSchema.properties)).not.toContain('children_type');
  });

  it('builds scroll-row-wrapper + scroll-row + 2 cards (140×160, cornerRadius=20)', async () => {
    const fp = await fresh('a.op');
    await handleAddCardRowV0({
      filePath: fp,
      items: [{ title: 'Hiit', subtitle: '30 min', icon: 'flame' }, { title: 'Bare' }],
    });
    const wrapper = getRoot(await readDoc(fp));
    expect(wrapper.role).toBe('scroll-row-wrapper');
    expect(wrapper.clipContent).toBe(true);
    const row = (wrapper.children as Record<string, unknown>[])[0];
    expect(row.role).toBe('scroll-row');
    expect(row.padding).toEqual([0, 20]);
    const cards = row.children as Record<string, unknown>[];
    expect(cards.length).toBe(2);
    expect(cards[0].width).toBe(140);
    expect(cards[0].height).toBe(160);
    expect(cards[0].cornerRadius).toBe(20);
    expect(cards[0].role).toBe('card');
    // icon + title + subtitle
    const k0 = cards[0].children as Record<string, unknown>[];
    expect(k0.length).toBe(3);
    expect(k0[0].iconFontName).toBe('flame');
    expect(k0[1].role).toBe('heading');
    expect(k0[2].role).toBe('body');
    // bare card has only title
    const k1 = cards[1].children as Record<string, unknown>[];
    expect(k1.length).toBe(1);
    expect(k1[0].content).toBe('Bare');
  });

  it('every node has unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddCardRowV0({
      filePath: fp,
      items: [{ title: 'A', subtitle: 's', icon: 'i' }, { title: 'B' }],
    });
    const ids: string[] = [];
    const missing: string[] = [];
    collectIds(getRoot(await readDoc(fp)), ids, missing);
    expect(missing).toEqual([]);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched (side-effect invariant)', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddCardRowV0({
        filePath: fp,
        items: [{ title: 'X' }],
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
