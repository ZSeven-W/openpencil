import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddListRowV0 } from '../tools/add-list-row-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-list-row-v0');
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

describe('add_list_row_v0', () => {
  it('registered + required title', () => {
    expect(DESIGN_TOOL_NAMES.has('add_list_row_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_list_row_v0');
    expect(def?.inputSchema.required).toEqual(['title']);
  });

  it('minimal (title only): 1 child — text stack (vertical wrapper)', async () => {
    const fp = await fresh('a.op');
    await handleAddListRowV0({ filePath: fp, title: 'Settings' });
    const row = getRoot(await readDoc(fp));
    expect(row.role).toBe('list-row');
    expect(row.width).toBe('fill_container');
    expect(row.height).toBe('fit_content');
    expect(row.layout).toBe('horizontal');
    expect(row.alignItems).toBe('center');
    expect(row.gap).toBe(12);
    expect(row.padding).toEqual([12, 16]);
    const kids = row.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('list-row-text');
    expect(kids[0].layout).toBe('vertical');
    expect(kids[0].width).toBe('fill_container');
  });

  it('full: leading + text stack + trailing = 3 top-level kids', async () => {
    const fp = await fresh('a.op');
    await handleAddListRowV0({
      filePath: fp,
      title: 'Notifications',
      subtitle: 'Push, email, and in-app',
      leading_icon: 'bell',
      trailing_icon: 'chevron-right',
    });
    const row = getRoot(await readDoc(fp));
    const kids = row.children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    // leading
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].iconFontName).toBe('bell');
    expect(kids[0].width).toBe(24);
    // text stack
    expect(kids[1].role).toBe('list-row-text');
    expect(kids[1].layout).toBe('vertical');
    expect(kids[1].width).toBe('fill_container');
    const stack = kids[1].children as Record<string, unknown>[];
    expect(stack.length).toBe(2);
    expect(stack[0].content).toBe('Notifications');
    expect(stack[0].fontSize).toBe(15);
    expect(stack[0].width).toBe('fill_container');
    expect(stack[0].textGrowth).toBe('fixed-width');
    expect(stack[1].content).toBe('Push, email, and in-app');
    expect(stack[1].fontSize).toBe(13);
    // trailing
    expect(kids[2].type).toBe('icon_font');
    expect(kids[2].iconFontName).toBe('chevron-right');
    expect(kids[2].width).toBe(16);
  });

  it('no-overlap invariant: text stack ALWAYS fill_container + vertical, NOT horizontal', async () => {
    // Regression guard: long titles must wrap vertically, not horizontally
    // push the trailing icon. This requires the text-stack wrapper to be
    // vertical (not horizontal) and have width=fill_container.
    const fp = await fresh('a.op');
    await handleAddListRowV0({
      filePath: fp,
      title: 'A very long title that would overflow without a fill_container wrapper',
      subtitle: 'And a supporting subtitle that also needs to wrap',
      trailing_icon: 'chevron-right',
    });
    const row = getRoot(await readDoc(fp));
    const kids = row.children as Record<string, unknown>[];
    // text stack is kids[0] when no leading icon
    const stack = kids[0];
    expect(stack.role).toBe('list-row-text');
    expect(stack.layout).toBe('vertical');
    expect(stack.width).toBe('fill_container');
    // trailing icon is kids[1]
    expect(kids[1].role ?? undefined).toBeUndefined(); // icon_font nodes have no role
    expect(kids[1].type).toBe('icon_font');
    expect(kids[1].iconFontName).toBe('chevron-right');
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddListRowV0({
      filePath: fp,
      title: 'T',
      subtitle: 'S',
      leading_icon: 'l',
      trailing_icon: 't',
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
    // row + leading icon + text stack + title + subtitle + trailing icon = 6
    expect(ids.length).toBe(6);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddListRowV0({ filePath: fp, title: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
