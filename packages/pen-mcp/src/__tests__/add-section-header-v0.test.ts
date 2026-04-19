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

  it('title-only: title wrapped in vertical container for correct wrap-height propagation', async () => {
    const fp = await fresh('a.op');
    await handleAddSectionHeaderV0({ filePath: fp, title: 'Recent Activity' });
    const header = getRoot(await readDoc(fp));
    expect(header.role).toBe('section-header');
    expect(header.width).toBe('fill_container');
    expect(header.height).toBe('fit_content');
    expect(header.layout).toBe('horizontal');
    expect(header.alignItems).toBe('center');
    expect(header.gap).toBe(16);
    expect(header.justifyContent).toBeUndefined();
    const kids = header.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    // Title is wrapped in a VERTICAL container so fill_container +
    // textGrowth:fixed-width text wraps correctly (per overflow.md rule)
    // and the wrapped height propagates to header's fit_content height.
    const titleContainer = kids[0];
    expect(titleContainer.role).toBe('section-header-title');
    expect(titleContainer.layout).toBe('vertical');
    expect(titleContainer.width).toBe('fill_container');
    expect(titleContainer.height).toBe('fit_content');
    const titleKids = titleContainer.children as Record<string, unknown>[];
    expect(titleKids.length).toBe(1);
    expect(titleKids[0].type).toBe('text');
    expect(titleKids[0].content).toBe('Recent Activity');
    expect(titleKids[0].role).toBe('heading');
    expect(titleKids[0].width).toBe('fill_container');
    expect(titleKids[0].textGrowth).toBe('fixed-width');
  });

  it('with action: title container + action group (label + icon)', async () => {
    const fp = await fresh('a.op');
    await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'Workouts',
      action: { label: 'See all', icon: 'arrow-right' },
    });
    const header = getRoot(await readDoc(fp));
    const kids = header.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    // Title container: vertical layout, fill_container — holds the text
    const titleContainer = kids[0];
    expect(titleContainer.role).toBe('section-header-title');
    expect(titleContainer.width).toBe('fill_container');
    expect(titleContainer.layout).toBe('vertical');
    const titleText = (titleContainer.children as Record<string, unknown>[])[0];
    expect(titleText.content).toBe('Workouts');
    // Action: fit_content on its own — always sits flush right
    const action = kids[1];
    expect(action.role).toBe('section-header-action');
    expect(action.layout).toBe('horizontal');
    expect(action.width).toBe('fit_content');
    const actionKids = action.children as Record<string, unknown>[];
    expect(actionKids.length).toBe(2);
    expect(actionKids[0].content).toBe('See all');
    expect(actionKids[1].iconFontName).toBe('arrow-right');
  });

  it('long title: wrapped height propagates to header fit_content (regression for Codex #10)', async () => {
    const fp = await fresh('a.op');
    await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'A Very Long Section Header Title That Would Otherwise Overflow Or Clip',
      action: { label: 'See all' },
    });
    const header = getRoot(await readDoc(fp));
    // Header height must be fit_content so wrap-grown title pushes it taller,
    // which in turn pushes following siblings in a vertical parent down
    // (no overlap with next content).
    expect(header.height).toBe('fit_content');
    const kids = header.children as Record<string, unknown>[];
    // Title container is VERTICAL so text fill_container+fixed-width wraps
    // correctly (per overflow.md rule — only vertical layout supports this).
    // Previous bug: text directly in horizontal header did not propagate
    // wrap height; following content would overlap the wrapped lines.
    const titleContainer = kids[0];
    expect(titleContainer.layout).toBe('vertical');
    expect(titleContainer.height).toBe('fit_content');
    const titleText = (titleContainer.children as Record<string, unknown>[])[0];
    expect(titleText.width).toBe('fill_container');
    expect(titleText.textGrowth).toBe('fixed-width');
    // Header must NOT have an explicit numeric height (would clip wrapped
    // title) or space_between (would cause horizontal overflow).
    expect(header.justifyContent).toBeUndefined();
    expect(header.gap).toBe(16);
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
    // header + title container + title text + action frame + (action label + action icon) = 6
    expect(ids.length).toBe(6);
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
