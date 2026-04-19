import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddIconButtonV0 } from '../tools/add-icon-button-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-icon-button-v0');
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

describe('add_icon_button_v0', () => {
  it('registered + required icon', () => {
    expect(DESIGN_TOOL_NAMES.has('add_icon_button_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_icon_button_v0');
    expect(def?.inputSchema.required).toEqual(['icon']);
  });

  it('default 44×44 with flex centering (NOT layout=none anti-pattern)', async () => {
    const fp = await fresh('a.op');
    await handleAddIconButtonV0({ filePath: fp, icon: 'search' });
    const btn = getRoot(await readDoc(fp));
    expect(btn.role).toBe('icon-button');
    expect(btn.width).toBe(44);
    expect(btn.height).toBe(44);
    expect(btn.cornerRadius).toBe(8);
    // Critical: NOT layout='none' (which renders unreliably per memory)
    expect(btn.layout).toBe('horizontal');
    expect(btn.justifyContent).toBe('center');
    expect(btn.alignItems).toBe('center');
    const kids = btn.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].iconFontName).toBe('search');
    expect(kids[0].iconFontFamily).toBe('lucide');
    expect(kids[0].width).toBe(24);
    expect(kids[0].height).toBe(24);
  });

  it('custom size + icon_size overrides propagate', async () => {
    const fp = await fresh('a.op');
    await handleAddIconButtonV0({
      filePath: fp,
      icon: 'menu',
      size: 40,
      icon_size: 20,
    });
    const btn = getRoot(await readDoc(fp));
    expect(btn.width).toBe(40);
    expect(btn.height).toBe(40);
    const icon = (btn.children as Record<string, unknown>[])[0];
    expect(icon.width).toBe(20);
    expect(icon.height).toBe(20);
  });

  it('button + icon both have unique ids', async () => {
    const fp = await fresh('a.op');
    await handleAddIconButtonV0({ filePath: fp, icon: 'x' });
    const btn = getRoot(await readDoc(fp));
    const iconNode = (btn.children as Record<string, unknown>[])[0];
    expect(typeof btn.id).toBe('string');
    expect(typeof iconNode.id).toBe('string');
    expect(btn.id).not.toBe(iconNode.id);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddIconButtonV0({ filePath: fp, icon: 'x', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
