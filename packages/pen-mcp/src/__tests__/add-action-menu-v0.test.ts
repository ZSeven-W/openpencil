import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddActionMenuV0 } from '../tools/add-action-menu-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-action-menu-v0');
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

describe('add_action_menu_v0', () => {
  it('registered; required=[items]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_action_menu_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_action_menu_v0');
    expect(def?.inputSchema.required).toEqual(['items']);
  });

  it('simple list: icon + label per item', async () => {
    const fp = await fresh('a.op');
    await handleAddActionMenuV0({
      filePath: fp,
      items: [
        { label: 'Edit', icon: 'pencil' },
        { label: 'Share', icon: 'share' },
      ],
    });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('action-menu');
    const items = root.children as Record<string, unknown>[];
    expect(items.length).toBe(2);
    expect(items[0].role).toBe('action-menu-item');
    const firstKids = items[0].children as Record<string, unknown>[];
    expect(firstKids[0].iconFontName).toBe('pencil');
    expect(firstKids[1].content).toBe('Edit');
  });

  it('destructive: red fill, role=action-menu-item-destructive', async () => {
    const fp = await fresh('a.op');
    await handleAddActionMenuV0({
      filePath: fp,
      items: [{ label: 'Delete', icon: 'trash', destructive: true }],
    });
    const root = getRoot(await readDoc(fp));
    const item = (root.children as Record<string, unknown>[])[0];
    expect(item.role).toBe('action-menu-item-destructive');
    const kids = item.children as Record<string, unknown>[];
    const labelFill = kids[1].fill as Array<{ color: string }>;
    expect(labelFill[0].color).toBe('#EF4444');
  });

  it('divider_before inserts a 1px hairline ABOVE the item', async () => {
    const fp = await fresh('a.op');
    await handleAddActionMenuV0({
      filePath: fp,
      items: [{ label: 'Edit' }, { label: 'Delete', destructive: true, divider_before: true }],
    });
    const root = getRoot(await readDoc(fp));
    const kids = root.children as Record<string, unknown>[];
    // Edit, divider, Delete = 3
    expect(kids.length).toBe(3);
    expect(kids[1].role).toBe('action-menu-divider');
    expect(kids[1].type).toBe('rectangle');
  });

  it('divider_before on FIRST item is ignored (no leading divider)', async () => {
    const fp = await fresh('a.op');
    await handleAddActionMenuV0({
      filePath: fp,
      items: [{ label: 'Edit', divider_before: true }, { label: 'Delete' }],
    });
    const root = getRoot(await readDoc(fp));
    const kids = root.children as Record<string, unknown>[];
    // No leading divider — just 2 items
    expect(kids.length).toBe(2);
    expect(kids[0].role).toBe('action-menu-item');
  });

  it('label-only item (no icon) omits the icon_font child', async () => {
    const fp = await fresh('a.op');
    await handleAddActionMenuV0({
      filePath: fp,
      items: [{ label: 'Just text' }],
    });
    const root = getRoot(await readDoc(fp));
    const item = (root.children as Record<string, unknown>[])[0];
    const kids = item.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('text');
  });

  it('width clamps (below 140 → 140)', async () => {
    const fp = await fresh('a.op');
    await handleAddActionMenuV0({ filePath: fp, items: [{ label: 'x' }], width: 50 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(140);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddActionMenuV0({ filePath: fp, items: [{ label: 'x' }], parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
