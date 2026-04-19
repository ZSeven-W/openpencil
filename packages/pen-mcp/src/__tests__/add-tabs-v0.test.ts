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

  it('3 tabs, active gets bottom stroke + fontWeight 600', async () => {
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
    // active tab has directional bottom stroke
    const activeStroke = tabs[0].stroke as Record<string, unknown>;
    expect((activeStroke.thickness as Record<string, unknown>).bottom).toBe(2);
    // non-active tab has no stroke
    expect(tabs[1].stroke).toBeUndefined();
    // active label weight 600
    const activeLabel = (tabs[0].children as Record<string, unknown>[])[0];
    expect(activeLabel.fontWeight).toBe(600);
    // non-active label weight 500
    const inactiveLabel = (tabs[1].children as Record<string, unknown>[])[0];
    expect(inactiveLabel.fontWeight).toBe(500);
  });

  it('all tabs inactive: none get stroke', async () => {
    const fp = await fresh('a.op');
    await handleAddTabsV0({
      filePath: fp,
      items: [{ label: 'A' }, { label: 'B' }],
    });
    const tabs = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    for (const tab of tabs) {
      expect(tab.role).toBe('tab');
      expect(tab.stroke).toBeUndefined();
    }
  });

  it('every tab + label has unique id', async () => {
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
    // bar + 2 tabs + 2 labels = 5
    expect(ids.length).toBe(5);
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
