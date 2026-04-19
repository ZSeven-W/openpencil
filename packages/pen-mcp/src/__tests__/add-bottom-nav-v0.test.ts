/**
 * Unit tests for add_bottom_nav_v0 — MVP element tool #2.
 * Anti-pattern origin: layout.md §NO FIXED-POSITION LAYOUT.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddBottomNavV0 } from '../tools/add-bottom-nav-v0';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-add-bottom-nav-v0-tests');
const EMPTY_DOC = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP_DIR, name);
  await writeFile(fp, EMPTY_DOC, 'utf-8');
  return fp;
}

async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}

function getRoot(doc: Record<string, unknown>): Record<string, unknown> {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const pageChildren = pages?.[0]?.children;
  const topChildren = doc['children'] as Record<string, unknown>[] | undefined;
  const root = pageChildren?.[0] ?? topChildren?.[0];
  if (!root) throw new Error('expected root');
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
  await mkdir(TMP_DIR, { recursive: true });
});

afterEach(async () => {
  for (const f of ['nav.op', 'active.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_bottom_nav_v0 — registration', () => {
  it('is registered in DESIGN_TOOL_DEFINITIONS + NAMES', () => {
    expect(DESIGN_TOOL_DEFINITIONS.map((t) => t.name)).toContain('add_bottom_nav_v0');
    expect(DESIGN_TOOL_NAMES.has('add_bottom_nav_v0')).toBe(true);
  });
});

describe('add_bottom_nav_v0 — structure', () => {
  it('produces bottom-tab-bar with fill_container + height=62 + horizontal + space_around', async () => {
    const fp = await fresh('nav.op');
    await handleAddBottomNavV0({
      filePath: fp,
      items: [
        { title: 'Home', icon: 'home' },
        { title: 'Search', icon: 'search' },
        { title: 'Profile', icon: 'user' },
      ],
    });
    const bar = getRoot(await readDoc(fp));
    expect(bar.role).toBe('bottom-tab-bar');
    expect(bar.width).toBe('fill_container');
    expect(bar.height).toBe(62);
    expect(bar.layout).toBe('horizontal');
    expect(bar.justifyContent).toBe('space_around');
    expect(bar.alignItems).toBe('center');
    const tabs = bar.children as Record<string, unknown>[];
    expect(tabs.length).toBe(3);
    for (const tab of tabs) {
      expect(tab.role).toBe('nav-item');
      expect(tab.layout).toBe('vertical');
      expect(tab.alignItems).toBe('center');
      const kids = tab.children as Record<string, unknown>[];
      expect(kids.length).toBe(2);
      expect(kids[0].type).toBe('icon_font');
      expect(kids[0].iconFontFamily).toBe('lucide');
      expect(kids[1].type).toBe('text');
    }
  });

  it('marks active tab with role nav-item-active + bolder label', async () => {
    const fp = await fresh('active.op');
    await handleAddBottomNavV0({
      filePath: fp,
      items: [
        { title: 'Home', icon: 'home', active: true },
        { title: 'Search', icon: 'search' },
      ],
    });
    const bar = getRoot(await readDoc(fp));
    const tabs = bar.children as Record<string, unknown>[];
    expect(tabs[0].role).toBe('nav-item-active');
    expect(tabs[1].role).toBe('nav-item');
    const activeLabel = (tabs[0].children as Record<string, unknown>[])[1];
    expect(activeLabel.fontWeight).toBe(600);
    const normalLabel = (tabs[1].children as Record<string, unknown>[])[1];
    expect(normalLabel.fontWeight).toBe(500);
  });

  it('every node in the tree has a non-empty unique id', async () => {
    const fp = await fresh('nav.op');
    await handleAddBottomNavV0({
      filePath: fp,
      items: [
        { title: 'A', icon: 'x' },
        { title: 'B', icon: 'y' },
      ],
    });
    const bar = getRoot(await readDoc(fp));
    const ids: string[] = [];
    const missing: string[] = [];
    collectIds(bar, ids, missing);
    expect(missing).toEqual([]);
    expect(new Set(ids).size).toBe(ids.length);
    // 1 bar + 2 tabs + 2 * 2 kids = 7
    expect(ids.length).toBe(7);
  });

  it('throws when parent_id refers to non-existent node (no silent no-op)', async () => {
    const fp = await fresh('nav.op');
    await expect(
      handleAddBottomNavV0({
        filePath: fp,
        items: [{ title: 'Home', icon: 'home' }],
        parent_id: 'bogus-parent',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
  });
});
