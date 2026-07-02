import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddTopNavBarV0 } from '../tools/add-top-nav-bar-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-top-nav-bar-v0');
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

describe('add_top_nav_bar_v0', () => {
  it('registered + required title', () => {
    expect(DESIGN_TOOL_NAMES.has('add_top_nav_bar_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_top_nav_bar_v0');
    expect(def?.inputSchema.required).toEqual(['title']);
  });

  it('both icons: 3 children = leading button + title + trailing button', async () => {
    const fp = await fresh('a.op');
    await handleAddTopNavBarV0({
      filePath: fp,
      title: 'Settings',
      leading_icon: 'chevron-left',
      trailing_icon: 'more-vertical',
    });
    const bar = getRoot(await readDoc(fp));
    expect(bar.role).toBe('top-nav-bar');
    expect(bar.width).toBe('fill_container');
    expect(bar.height).toBe(56);
    expect(bar.justifyContent).toBe('space_between');
    expect(bar.padding).toEqual([0, 16]);
    const kids = bar.children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    expect(kids[0].role).toBe('icon-button');
    expect((kids[0].children as Record<string, unknown>[])[0].iconFontName).toBe('chevron-left');
    expect(kids[1].content).toBe('Settings');
    expect(kids[1].role).toBe('heading');
    expect(kids[2].role).toBe('icon-button');
    expect((kids[2].children as Record<string, unknown>[])[0].iconFontName).toBe('more-vertical');
  });

  it('no icons: 44×44 spacers on both sides keep title visually centered', async () => {
    const fp = await fresh('a.op');
    await handleAddTopNavBarV0({ filePath: fp, title: 'Home' });
    const bar = getRoot(await readDoc(fp));
    const kids = bar.children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    expect(kids[0].role).toBe('nav-spacer');
    expect(kids[0].width).toBe(44);
    expect(kids[0].height).toBe(44);
    expect(kids[2].role).toBe('nav-spacer');
  });

  it('asymmetric (leading only): trailing becomes spacer', async () => {
    const fp = await fresh('a.op');
    await handleAddTopNavBarV0({
      filePath: fp,
      title: 'Profile',
      leading_icon: 'arrow-left',
    });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids[0].role).toBe('icon-button');
    expect(kids[2].role).toBe('nav-spacer');
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddTopNavBarV0({
      filePath: fp,
      title: 'X',
      leading_icon: 'a',
      trailing_icon: 'b',
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
    // bar + 3 slots + 2 icons (leading/trailing) + 0 (title is text, counted) = bar + leading(btn+icon) + title + trailing(btn+icon) = 1+2+1+2 = 6
    expect(ids.length).toBe(6);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddTopNavBarV0({
        filePath: fp,
        title: 'X',
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
