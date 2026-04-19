import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddFabV0 } from '../tools/add-fab-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-fab-v0');
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

describe('add_fab_v0', () => {
  it('registered + required icon', () => {
    expect(DESIGN_TOOL_NAMES.has('add_fab_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_fab_v0');
    expect(def?.inputSchema.required).toEqual(['icon']);
  });

  it('default 56×56 circular with centered icon', async () => {
    const fp = await fresh('a.op');
    await handleAddFabV0({ filePath: fp, icon: 'plus' });
    const fab = getRoot(await readDoc(fp));
    expect(fab.role).toBe('fab');
    expect(fab.width).toBe(56);
    expect(fab.height).toBe(56);
    expect(fab.cornerRadius).toBe(28);
    expect(fab.alignItems).toBe('center');
    expect(fab.justifyContent).toBe('center');
    const icon = (fab.children as Record<string, unknown>[])[0];
    expect(icon.iconFontName).toBe('plus');
    expect(icon.width).toBe(24);
  });

  it('custom size scales icon proportionally (~43%)', async () => {
    const fp = await fresh('a.op');
    await handleAddFabV0({ filePath: fp, icon: 'edit', size: 40 });
    const fab = getRoot(await readDoc(fp));
    expect(fab.width).toBe(40);
    expect(fab.cornerRadius).toBe(20);
    const icon = (fab.children as Record<string, unknown>[])[0];
    expect(icon.width).toBe(17); // round(40 * 0.43) = 17
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddFabV0({ filePath: fp, icon: 'x', parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
