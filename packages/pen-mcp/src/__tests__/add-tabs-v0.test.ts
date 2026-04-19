import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddTabsV0 } from '../tools/add-tabs-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-tabs-v0');
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

describe('add_tabs_v0', () => {
  it('registered + required items', () => {
    expect(DESIGN_TOOL_NAMES.has('add_tabs_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_tabs_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
  });

  it('3 tabs, active gets sibling underline rect + fontWeight 600', async () => {
    const fp = await fresh('a.op');
    await handleAddTabsV0({
      filePath: fp,
      items: [{ label: 'Overview', active: true }, { label: 'Details' }, { label: 'Reviews' }],
    });
    const bar = getRoot(await readDoc(fp));
    expect(bar.role).toBe('tabs');
    expect(bar.layout).toBe('horizontal');
    expect(bar.width).toBe('fill_container');
    const tabs = bar.children as Record<string, unknown>[];
    expect(tabs.length).toBe(3);
    expect(tabs[0].role).toBe('tab-active');
    expect(tabs[1].role).toBe('tab');
    expect(tabs[2].role).toBe('tab');
    // tabs never use directional-object stroke (unsupported by renderer)
    for (const tab of tabs) {
      if (tab.stroke !== undefined) {
        const strokeObj = tab.stroke as Record<string, unknown>;
        expect(typeof strokeObj.thickness === 'number' || Array.isArray(strokeObj.thickness)).toBe(
          true,
        );
      }
    }
    // active tab: 2 children (inner + underline rect)
    const activeChildren = tabs[0].children as Record<string, unknown>[];
    expect(activeChildren.length).toBe(2);
    const underline = activeChildren[1];
    expect(underline.type).toBe('rectangle');
    expect(underline.role).toBe('tab-underline');
    expect(underline.width).toBe('fill_container');
    expect(underline.height).toBe(2);
    // inactive tab: 1 child (inner only)
    const inactiveChildren = tabs[1].children as Record<string, unknown>[];
    expect(inactiveChildren.length).toBe(1);
    // active label weight 600 (reached via inner wrapper)
    const activeInner = activeChildren[0] as Record<string, unknown>;
    const activeLabel = (activeInner.children as Record<string, unknown>[])[0];
    expect(activeLabel.fontWeight).toBe(600);
    const inactiveInner = inactiveChildren[0] as Record<string, unknown>;
    const inactiveLabel = (inactiveInner.children as Record<string, unknown>[])[0];
    expect(inactiveLabel.fontWeight).toBe(500);
  });

  it('all tabs inactive: no underline rects anywhere', async () => {
    const fp = await fresh('a.op');
    await handleAddTabsV0({
      filePath: fp,
      items: [{ label: 'A' }, { label: 'B' }],
    });
    const tabs = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    for (const tab of tabs) {
      expect(tab.role).toBe('tab');
      const kids = tab.children as Record<string, unknown>[];
      expect(kids.length).toBe(1);
      expect(kids[0].type).toBe('frame'); // inner only, no underline rect
    }
  });

  it('every tab + inner + label + underline has unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddTabsV0({
      filePath: fp,
      items: [{ label: 'A', active: true }, { label: 'B' }],
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
    // bar + (tab_A + inner_A + label_A + underline_A) + (tab_B + inner_B + label_B) = 8
    expect(ids.length).toBe(8);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddTabsV0({ filePath: fp, items: [{ label: 'X' }], parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
