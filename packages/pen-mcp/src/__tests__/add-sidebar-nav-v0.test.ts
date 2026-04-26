/**
 * Unit tests for add_sidebar_nav_v0 — desktop persistent left rail.
 * Distinct surface from `add_bottom_nav_v0` (mobile horizontal tabs).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSidebarNavV0 } from '../tools/add-sidebar-nav-v0';
import { invalidateCache } from '../document-manager';

const TMP_DIR = join(tmpdir(), 'openpencil-add-sidebar-nav-v0-tests');
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
  for (const f of ['nav.op', 'titled.op', 'active.op', 'clamp.op', 'parent.op']) {
    try {
      const fp = join(TMP_DIR, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_sidebar_nav_v0 — registration', () => {
  it('is registered in DESIGN_TOOL_DEFINITIONS + NAMES', () => {
    expect(DESIGN_TOOL_DEFINITIONS.map((t) => t.name)).toContain('add_sidebar_nav_v0');
    expect(DESIGN_TOOL_NAMES.has('add_sidebar_nav_v0')).toBe(true);
  });
});

describe('add_sidebar_nav_v0 — structure', () => {
  it('emits sidebar-nav frame with default 240 width + fill_container height + #FFFFFF fill', async () => {
    const fp = await fresh('nav.op');
    await handleAddSidebarNavV0({
      filePath: fp,
      items: [
        { label: 'Dashboard', icon: 'home' },
        { label: 'Settings', icon: 'settings' },
      ],
    });
    const nav = getRoot(await readDoc(fp));
    expect(nav.role).toBe('sidebar-nav');
    expect(nav.width).toBe(240);
    expect(nav.height).toBe('fill_container');
    expect(nav.layout).toBe('vertical');
    // Padding MUST use the unified `padding` field (number | [T,R] |
    // [T,R,B,L]); the CSS-style paddingTop/Right/Bottom/Left siblings
    // are silently dropped by the layout engine, leaving the rail with
    // no insets. Assert the array form survived to catch regressions.
    expect(nav.padding).toEqual([16, 12]);
    expect(nav.paddingTop).toBeUndefined();
    expect(nav.paddingLeft).toBeUndefined();
    const fill = nav.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#FFFFFF');
    const items = nav.children as Record<string, unknown>[];
    expect(items.length).toBe(2);
    expect(items[0].role).toBe('sidebar-nav-item');
    expect(items[0].height).toBe(40);
    expect(items[0].layout).toBe('horizontal');
    const itemKids = items[0].children as Record<string, unknown>[];
    expect(itemKids[0].type).toBe('icon_font');
    expect(itemKids[0].iconFontFamily).toBe('lucide');
    expect(itemKids[1].type).toBe('text');
    expect(itemKids[1].fontWeight).toBe(500);
  });

  it('renders optional title row above items when title is set', async () => {
    const fp = await fresh('titled.op');
    await handleAddSidebarNavV0({
      filePath: fp,
      title: 'Acme Inc',
      items: [{ label: 'Home', icon: 'home' }],
    });
    const nav = getRoot(await readDoc(fp));
    const kids = nav.children as Record<string, unknown>[];
    expect(kids.length).toBe(2); // title row + 1 item
    expect(kids[0].role).toBe('sidebar-nav-title');
    const titleText = (kids[0].children as Record<string, unknown>[])[0];
    expect(titleText.type).toBe('text');
    expect(titleText.content).toBe('Acme Inc');
    expect(titleText.fontSize).toBe(16);
    expect(titleText.fontWeight).toBe(700);
    expect(kids[1].role).toBe('sidebar-nav-item');
    expect(kids[0].padding).toEqual([8, 12, 24, 12]);
    expect(kids[0].paddingTop).toBeUndefined();
  });

  it('marks active item with sidebar-nav-item-active role + slate-100 fill + bolder darker label', async () => {
    const fp = await fresh('active.op');
    await handleAddSidebarNavV0({
      filePath: fp,
      items: [
        { label: 'Dashboard', icon: 'home', active: true },
        { label: 'Settings', icon: 'settings' },
      ],
    });
    const nav = getRoot(await readDoc(fp));
    const items = nav.children as Record<string, unknown>[];
    expect(items[0].role).toBe('sidebar-nav-item-active');
    const activeFill = items[0].fill as Array<{ color: string }> | undefined;
    expect(activeFill?.[0].color).toBe('#F1F5F9');
    const activeLabel = (items[0].children as Record<string, unknown>[])[1];
    expect(activeLabel.fontWeight).toBe(600);
    expect((activeLabel.fill as Array<{ color: string }>)[0].color).toBe('#0F172A');

    expect(items[1].role).toBe('sidebar-nav-item');
    expect(items[1].fill).toBeUndefined();
    const inactiveLabel = (items[1].children as Record<string, unknown>[])[1];
    expect(inactiveLabel.fontWeight).toBe(500);
    expect((inactiveLabel.fill as Array<{ color: string }>)[0].color).toBe('#475569');
  });

  it('clamps width to [180, 320]', async () => {
    const fp = await fresh('clamp.op');
    await handleAddSidebarNavV0({
      filePath: fp,
      width: 99,
      items: [{ label: 'A', icon: 'home' }],
    });
    const narrow = getRoot(await readDoc(fp));
    expect(narrow.width).toBe(180);
    invalidateCache(fp);
    await writeFile(fp, EMPTY_DOC, 'utf-8');
    await handleAddSidebarNavV0({
      filePath: fp,
      width: 999,
      items: [{ label: 'A', icon: 'home' }],
    });
    const wide = getRoot(await readDoc(fp));
    expect(wide.width).toBe(320);
  });

  it('every node in the tree has a non-empty unique id', async () => {
    const fp = await fresh('nav.op');
    await handleAddSidebarNavV0({
      filePath: fp,
      title: 'Acme',
      items: [
        { label: 'A', icon: 'home', active: true },
        { label: 'B', icon: 'settings' },
      ],
    });
    const nav = getRoot(await readDoc(fp));
    const ids: string[] = [];
    const missing: string[] = [];
    collectIds(nav, ids, missing);
    expect(missing).toEqual([]);
    expect(new Set(ids).size).toBe(ids.length);
    // 1 nav + (1 title row + 1 title text) + 2 items × (1 item frame + 1 icon + 1 label) = 9
    expect(ids.length).toBe(9);
  });

  it('throws on bogus parent_id AND leaves file untouched (side-effect invariant)', async () => {
    const fp = await fresh('parent.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddSidebarNavV0({
        filePath: fp,
        items: [{ label: 'Home', icon: 'home' }],
        parent_id: 'bogus-parent',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});
