import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSectionHeaderV0 } from '../tools/add-section-header-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-section-header-v0');
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

describe('add_section_header_v0', () => {
  it('registered + required title', () => {
    expect(DESIGN_TOOL_NAMES.has('add_section_header_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_section_header_v0');
    expect(def?.inputSchema.required).toEqual(['title']);
  });

  it('title-only: horizontal + space_between + 1 child (the title)', async () => {
    const fp = await fresh('a.op');
    await handleAddSectionHeaderV0({ filePath: fp, title: 'Recent Activity' });
    const header = getRoot(await readDoc(fp));
    expect(header.role).toBe('section-header');
    expect(header.width).toBe('fill_container');
    expect(header.layout).toBe('horizontal');
    expect(header.justifyContent).toBe('space_between');
    expect(header.alignItems).toBe('center');
    const kids = header.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('text');
    expect(kids[0].content).toBe('Recent Activity');
    expect(kids[0].role).toBe('heading');
  });

  it('with action: title + action group (label + icon)', async () => {
    const fp = await fresh('a.op');
    await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'Workouts',
      action: { label: 'See all', icon: 'arrow-right' },
    });
    const header = getRoot(await readDoc(fp));
    const kids = header.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].content).toBe('Workouts');
    const action = kids[1];
    expect(action.role).toBe('section-header-action');
    expect(action.layout).toBe('horizontal');
    const actionKids = action.children as Record<string, unknown>[];
    expect(actionKids.length).toBe(2);
    expect(actionKids[0].content).toBe('See all');
    expect(actionKids[1].iconFontName).toBe('arrow-right');
  });

  it('action without icon emits label-only', async () => {
    const fp = await fresh('a.op');
    await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'Stats',
      action: { label: 'View more' },
    });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const actionKids = kids[1].children as Record<string, unknown>[];
    expect(actionKids.length).toBe(1);
    expect(actionKids[0].content).toBe('View more');
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'Foo',
      action: { label: 'Bar', icon: 'x' },
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
    // header + title + action frame + (action label + action icon) = 5
    expect(ids.length).toBe(5);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddSectionHeaderV0({
        filePath: fp,
        title: 'X',
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
