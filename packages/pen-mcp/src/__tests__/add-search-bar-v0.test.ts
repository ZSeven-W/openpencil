import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSearchBarV0 } from '../tools/add-search-bar-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-search-bar-v0');
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

describe('add_search_bar_v0', () => {
  it('registered + no required fields (all optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_search_bar_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_search_bar_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('defaults: height=44, cornerRadius=22, leading=search, placeholder=Search...', async () => {
    const fp = await fresh('a.op');
    await handleAddSearchBarV0({ filePath: fp });
    const b = getRoot(await readDoc(fp));
    expect(b.role).toBe('search-bar');
    expect(b.width).toBe('fill_container');
    expect(b.height).toBe(44);
    expect(b.cornerRadius).toBe(22);
    expect(b.layout).toBe('horizontal');
    expect(b.alignItems).toBe('center');
    expect(b.padding).toEqual([0, 16]);
    const kids = b.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].iconFontName).toBe('search');
    expect(kids[0].width).toBe(20);
    expect(kids[1].type).toBe('text');
    expect(kids[1].content).toBe('Search...');
    expect(kids[1].fontSize).toBe(14);
  });

  it('custom placeholder and leading icon', async () => {
    const fp = await fresh('a.op');
    await handleAddSearchBarV0({
      filePath: fp,
      placeholder: 'Find anything',
      leading_icon: 'filter',
    });
    const b = getRoot(await readDoc(fp));
    const kids = b.children as Record<string, unknown>[];
    expect(kids[0].iconFontName).toBe('filter');
    expect(kids[1].content).toBe('Find anything');
  });

  it('3 unique ids', async () => {
    const fp = await fresh('a.op');
    await handleAddSearchBarV0({ filePath: fp });
    const b = getRoot(await readDoc(fp));
    const ids = [b.id, ...(b.children as Record<string, unknown>[]).map((c) => c.id)];
    expect(new Set(ids).size).toBe(3);
    for (const id of ids) expect(typeof id).toBe('string');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddSearchBarV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
